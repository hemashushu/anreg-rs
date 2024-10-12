// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::{
    ast::{
        AssertionName, CharRange, CharSet, CharSetElement, Expression, FunctionCall,
        FunctionCallArg, FunctionName, Literal, PresetCharSetName, Program,
    },
    error::Error,
    parser::parse_from_str,
    state::StateSet,
    transition::{
        add_char, add_preset_digit, add_preset_space, add_preset_word, add_range,
        AssertionTransition, BackReferenceTransition, CharSetItem, CharSetTransition,
        CharTransition, JumpTransition, MatchEndTransition, MatchStartTransition,
        SpecialCharTransition, StringTransition, Transition,
    },
};

pub fn compile(program: &Program) -> Result<StateSet, Error> {
    let mut state_set = StateSet::new();
    let mut compiler = Compiler::new(program, &mut state_set);
    compiler.compile()?;

    Ok(state_set)
}

pub fn compile_from_str(s: &str) -> Result<StateSet, Error> {
    let program = parse_from_str(s)?;
    compile(&program)
}

pub struct Compiler<'a> {
    program: &'a Program,
    state_set: &'a mut StateSet,
}

impl<'a> Compiler<'a> {
    fn new(program: &'a Program, state_set: &'a mut StateSet) -> Self {
        Compiler { program, state_set }
    }

    fn compile(&mut self) -> Result<(), Error> {
        self.emit_program(&self.program)
    }

    fn emit_program(&mut self, program: &Program) -> Result<(), Error> {
        // the `Program` node is actually a `Group` which omits the parentheses,
        // in addition, there may be the 'start' and 'end' assertions.

        // create the index 0 match for the program
        let match_index = self.state_set.new_match(None);

        let expressions = &program.expressions;

        let mut ports = vec![];
        for (expression_index, expression) in expressions.iter().enumerate() {
            if matches!(expression, Expression::Assertion(AssertionName::Start)) {
                if expression_index == 0 {
                    // skips the 'start' assertion if it is present.
                } else {
                    return Err(Error::Message(
                        "The assertion \"start\" can only be present at the beginning of expression."
                            .to_owned(),
                    ));
                }
            } else if matches!(expression, Expression::Assertion(AssertionName::End)) {
                if expression_index == expressions.len() - 1 {
                    // skips the 'end' assertion if it is present.
                } else {
                    return Err(Error::Message(
                        "The assertion \"end\" can only be present at the end of expression."
                            .to_owned(),
                    ));
                }
            } else {
                ports.push(self.emit_expression(expression)?);
            }
        }

        let program_port = if ports.is_empty() {
            // todo: empty expression
            Port::new(0, 0)
        } else if ports.len() == 1 {
            // eliminates the nested group, e.g. '(((...)))'
            ports.pop().unwrap()
        } else {
            for idx in 0..(ports.len() - 1) {
                let current_out_state_index = ports[idx].out_state_index;
                let next_in_state_index = ports[idx + 1].in_state_index;
                let transition = Transition::Jump(JumpTransition);
                self.state_set.append_transition(
                    current_out_state_index,
                    next_in_state_index,
                    transition,
                );
            }

            Port::new(
                ports.first().unwrap().in_state_index,
                ports.last().unwrap().out_state_index,
            )
        };

        //                              current
        //   in                      /-----------\                    out
        //  --o==match start trans==--o in  out o--==match end trans==o--
        //                           \-----------/
        //

        let in_state_index = self.state_set.new_state();
        let out_state_index = self.state_set.new_state();

        let match_start_transition = MatchStartTransition::new(match_index);
        let match_end_transition = MatchEndTransition::new(match_index);

        self.state_set.append_transition(
            in_state_index,
            program_port.in_state_index,
            Transition::MatchStart(match_start_transition),
        );

        self.state_set.append_transition(
            program_port.out_state_index,
            out_state_index,
            Transition::MatchEnd(match_end_transition),
        );

        let match_port = Port::new(in_state_index, out_state_index);

        // save the program ports
        self.state_set.start_node_index = match_port.in_state_index;
        self.state_set.end_node_index = match_port.out_state_index;

        Ok(())
    }

    fn emit_expression(&mut self, expression: &Expression) -> Result<Port, Error> {
        let result = match expression {
            Expression::Literal(literal) => self.emit_literal(literal)?,
            Expression::Identifier(identifier) => self.emit_backreference(identifier)?,
            Expression::Assertion(name) => self.emit_assertion(name)?,
            Expression::Group(expressions) => self.emit_group(expressions)?,
            Expression::FunctionCall(function_call) => self.emit_function_call(function_call)?,
            Expression::Or(left, right) => self.emit_logic_or(left, right)?,
        };

        Ok(result)
    }

    fn emit_group(&mut self, expressions: &[Expression]) -> Result<Port, Error> {
        // connecting two groups of states by adding a 'jump transition'
        //
        //     current                      next
        //  /-----------\               /-----------\
        // --o in  out o-- ==jump trans== --o in  out o--
        //  \-----------/               \-----------/
        //

        if expressions.is_empty() {
            return Err(Error::Message("A group should not be empty.".to_owned()));
        }

        let mut ports = vec![];
        for expression in expressions {
            ports.push(self.emit_expression(expression)?);
        }

        if ports.len() == 1 {
            // eliminates the nested group, e.g. '(((...)))'
            let result = ports.pop().unwrap();
            Ok(result)
        } else {
            for idx in 0..(ports.len() - 1) {
                let current_out_state_index = ports[idx].out_state_index;
                let next_in_state_index = ports[idx + 1].in_state_index;
                let transition = Transition::Jump(JumpTransition);
                self.state_set.append_transition(
                    current_out_state_index,
                    next_in_state_index,
                    transition,
                );
            }

            let result = Port::new(
                ports.first().unwrap().in_state_index,
                ports.last().unwrap().out_state_index,
            );

            Ok(result)
        }
    }

    fn emit_logic_or(&mut self, left: &Expression, right: &Expression) -> Result<Port, Error> {
        //                    left
        //                /-----------\
        //      /==jump==--o in  out o--==jump==\
        //  in  |         \-----------/         |  out
        // --o==|                               |==o--
        //      |             right             |
        //      |         /-----------\         |
        //      \==jump==--o in  out o--==jump==/
        //                \-----------/

        let left_result = self.emit_expression(left)?;
        let right_result = self.emit_expression(right)?;

        let in_state_index = self.state_set.new_state();
        let out_state_index = self.state_set.new_state();

        self.state_set.append_transition(
            in_state_index,
            left_result.in_state_index,
            Transition::Jump(JumpTransition),
        );

        self.state_set.append_transition(
            in_state_index,
            right_result.in_state_index,
            Transition::Jump(JumpTransition),
        );

        self.state_set.append_transition(
            left_result.out_state_index,
            out_state_index,
            Transition::Jump(JumpTransition),
        );

        self.state_set.append_transition(
            right_result.out_state_index,
            out_state_index,
            Transition::Jump(JumpTransition),
        );

        Ok(Port::new(in_state_index, out_state_index))
    }

    fn emit_function_call(&mut self, function_call: &FunctionCall) -> Result<Port, Error> {
        match &function_call.name {
            FunctionName::Optional => todo!(),
            FunctionName::OneOrMore => todo!(),
            FunctionName::ZeroOrMore => todo!(),
            FunctionName::Repeat => todo!(),
            FunctionName::RepeatRange => todo!(),
            FunctionName::AtLeast => todo!(),
            FunctionName::OptionalLazy => todo!(),
            FunctionName::OneOrMoreLazy => todo!(),
            FunctionName::ZeroOrMoreLazy => todo!(),
            FunctionName::RepeatLazy => todo!(),
            FunctionName::RepeatRangeLazy => todo!(),
            FunctionName::AtLeastLazy => todo!(),
            FunctionName::IsBefore => todo!(),
            FunctionName::IsAfter => todo!(),
            FunctionName::IsNotBefore => todo!(),
            FunctionName::IsNotAfter => todo!(),
            FunctionName::Name => {
                self.emit_function_call_name(&function_call.expression, &function_call.args)
            }
            FunctionName::Index => self.emit_function_call_index(&function_call.expression),
        }
    }

    fn emit_literal(&mut self, literal: &Literal) -> Result<Port, Error> {
        let result = match literal {
            Literal::Char(character) => self.emit_literal_char(*character)?,
            Literal::String(s) => self.emit_literal_string(s)?,
            Literal::CharSet(charset) => self.emit_literal_charset(charset)?,
            Literal::PresetCharSet(name) => self.emit_literal_preset_charset(name)?,
            Literal::Special(_) => self.emit_literal_special_char()?,
        };

        Ok(result)
    }

    fn emit_literal_char(&mut self, character: char) -> Result<Port, Error> {
        let in_state_index = self.state_set.new_state();
        let out_state_index = self.state_set.new_state();

        let transition = Transition::Char(CharTransition::new(character));
        self.state_set
            .append_transition(in_state_index, out_state_index, transition);
        Ok(Port::new(in_state_index, out_state_index))
    }

    fn emit_literal_special_char(&mut self) -> Result<Port, Error> {
        let in_state_index = self.state_set.new_state();
        let out_state_index = self.state_set.new_state();

        let transition = Transition::SpecialChar(SpecialCharTransition);
        self.state_set
            .append_transition(in_state_index, out_state_index, transition);
        Ok(Port::new(in_state_index, out_state_index))
    }

    fn emit_literal_string(&mut self, s: &str) -> Result<Port, Error> {
        let in_state_index = self.state_set.new_state();
        let out_state_index = self.state_set.new_state();

        let transition = Transition::String(StringTransition::new(s));
        self.state_set
            .append_transition(in_state_index, out_state_index, transition);
        Ok(Port::new(in_state_index, out_state_index))
    }

    fn emit_literal_preset_charset(&mut self, name: &PresetCharSetName) -> Result<Port, Error> {
        let in_state_index = self.state_set.new_state();
        let out_state_index = self.state_set.new_state();

        let charset_transition = match name {
            PresetCharSetName::CharWord => CharSetTransition::new_preset_word(),
            PresetCharSetName::CharNotWord => CharSetTransition::new_preset_not_word(),
            PresetCharSetName::CharSpace => CharSetTransition::new_preset_space(),
            PresetCharSetName::CharNotSpace => CharSetTransition::new_preset_not_space(),
            PresetCharSetName::CharDigit => CharSetTransition::new_preset_digit(),
            PresetCharSetName::CharNotDigit => CharSetTransition::new_preset_not_digit(),
        };

        let transition = Transition::CharSet(charset_transition);
        self.state_set
            .append_transition(in_state_index, out_state_index, transition);
        Ok(Port::new(in_state_index, out_state_index))
    }

    fn emit_literal_charset(&mut self, charset: &CharSet) -> Result<Port, Error> {
        let in_state_index = self.state_set.new_state();
        let out_state_index = self.state_set.new_state();

        let mut items: Vec<CharSetItem> = vec![];
        Compiler::append_charset(charset, &mut items)?;

        let charset_transition = CharSetTransition::new(items, charset.negative);
        let transition = Transition::CharSet(charset_transition);
        self.state_set
            .append_transition(in_state_index, out_state_index, transition);
        Ok(Port::new(in_state_index, out_state_index))
    }

    fn append_preset_charset_positive_only(
        name: &PresetCharSetName,
        items: &mut Vec<CharSetItem>,
    ) -> Result<(), Error> {
        match name {
            PresetCharSetName::CharWord => {
                add_preset_word(items);
            }
            PresetCharSetName::CharSpace => {
                add_preset_space(items);
            }
            PresetCharSetName::CharDigit => {
                add_preset_digit(items);
            }
            _ => {
                return Err(Error::Message(format!(
                    "Can not append negative preset charset \"{}\" into charset.",
                    name
                )));
            }
        }

        Ok(())
    }

    fn append_charset(charset: &CharSet, items: &mut Vec<CharSetItem>) -> Result<(), Error> {
        for element in &charset.elements {
            match element {
                CharSetElement::Char(c) => add_char(items, *c),
                CharSetElement::CharRange(CharRange {
                    start,
                    end_included,
                }) => add_range(items, *start, *end_included),
                CharSetElement::PresetCharSet(name) => {
                    Compiler::append_preset_charset_positive_only(name, items)?;
                }
                CharSetElement::CharSet(custom_charset) => {
                    assert!(!custom_charset.negative);
                    Compiler::append_charset(custom_charset, items)?;
                }
            }
        }

        Ok(())
    }

    fn emit_assertion(&mut self, name: &AssertionName) -> Result<Port, Error> {
        let in_state_index = self.state_set.new_state();
        let out_state_index = self.state_set.new_state();

        let assertion_transition = AssertionTransition::new(*name);

        let transition = Transition::Assertion(assertion_transition);
        self.state_set
            .append_transition(in_state_index, out_state_index, transition);
        Ok(Port::new(in_state_index, out_state_index))
    }

    fn emit_backreference(&mut self, name: &str) -> Result<Port, Error> {
        let in_state_index = self.state_set.new_state();
        let out_state_index = self.state_set.new_state();

        let match_index_option = self.state_set.find_match_index(name);
        let match_index = if let Some(i) = match_index_option {
            i
        } else {
            return Err(Error::Message(format!(
                "Cannot find the match with name: \"{}\".",
                name
            )));
        };

        let transition = Transition::BackReference(BackReferenceTransition::new(match_index));
        self.state_set
            .append_transition(in_state_index, out_state_index, transition);
        Ok(Port::new(in_state_index, out_state_index))
    }

    fn emit_function_call_name(
        &mut self,
        expression: &Expression,
        args: &[FunctionCallArg],
    ) -> Result<Port, Error> {
        let name = if let FunctionCallArg::Identifier(s) = &args[0] {
            s.to_owned()
        } else {
            unreachable!();
        };

        self.continue_emit_function_call_name_and_index(expression, Some(name))
    }

    fn emit_function_call_index(&mut self, expression: &Expression) -> Result<Port, Error> {
        self.continue_emit_function_call_name_and_index(expression, None)
    }

    fn continue_emit_function_call_name_and_index(
        &mut self,
        expression: &Expression,
        name_option: Option<String>,
    ) -> Result<Port, Error> {
        let match_index = self.state_set.new_match(name_option);
        let port = self.emit_expression(expression)?;

        //                              current
        //   in                      /-----------\                    out
        //  --o==match start trans==--o in  out o--==match end trans==o--
        //                           \-----------/
        //

        let in_state_index = self.state_set.new_state();
        let out_state_index = self.state_set.new_state();

        let match_start_transition = MatchStartTransition::new(match_index);
        let match_end_transition = MatchEndTransition::new(match_index);

        self.state_set.append_transition(
            in_state_index,
            port.in_state_index,
            Transition::MatchStart(match_start_transition),
        );

        self.state_set.append_transition(
            port.out_state_index,
            out_state_index,
            Transition::MatchEnd(match_end_transition),
        );

        Ok(Port::new(in_state_index, out_state_index))
    }
}

struct Port {
    in_state_index: usize,
    out_state_index: usize,
}

impl Port {
    fn new(in_state_index: usize, out_state_index: usize) -> Self {
        Port {
            in_state_index,
            out_state_index,
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_str_eq;

    use crate::error::Error;

    use super::compile_from_str;

    #[test]
    fn test_compile_char() {
        // single char
        {
            let state_set = compile_from_str(r#"'a'"#).unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 3, Match end {0}
> 2
  -> 0, Match start {0}
< 3
# {0}"
            );
        }

        // sequence chars
        {
            let state_set = compile_from_str(r#"'a', 'b', 'c'"#).unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 2, Jump
- 2
  -> 3, Char 'b'
- 3
  -> 4, Jump
- 4
  -> 5, Char 'c'
- 5
  -> 7, Match end {0}
> 6
  -> 0, Match start {0}
< 7
# {0}"
            );
        }

        // char group
        // note: the group of anreg is different from traditional regex, it is
        // only a sequence pattern.
        {
            let state_set = compile_from_str(r#"'a',('b','c'), 'd'"#).unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 2, Jump
- 2
  -> 3, Char 'b'
- 3
  -> 4, Jump
- 4
  -> 5, Char 'c'
- 5
  -> 6, Jump
- 6
  -> 7, Char 'd'
- 7
  -> 9, Match end {0}
> 8
  -> 0, Match start {0}
< 9
# {0}"
            );
        }

        // nested groups
        {
            let state_set = compile_from_str(r#"'a',('b', ('c', 'd'), 'e'), 'f'"#).unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 2, Jump
- 2
  -> 3, Char 'b'
- 3
  -> 4, Jump
- 4
  -> 5, Char 'c'
- 5
  -> 6, Jump
- 6
  -> 7, Char 'd'
- 7
  -> 8, Jump
- 8
  -> 9, Char 'e'
- 9
  -> 10, Jump
- 10
  -> 11, Char 'f'
- 11
  -> 13, Match end {0}
> 12
  -> 0, Match start {0}
< 13
# {0}"
            );
        }
    }

    #[test]
    fn test_compile_logic_or() {
        // two operands
        {
            let state_set = compile_from_str(r#"'a' || 'b'"#).unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 5, Jump
- 2
  -> 3, Char 'b'
- 3
  -> 5, Jump
- 4
  -> 0, Jump
  -> 2, Jump
- 5
  -> 7, Match end {0}
> 6
  -> 4, Match start {0}
< 7
# {0}"
            );
        }

        // three operands
        // operator associativity
        // the current interpreter is right-associative, so:
        // "'a' || 'b' || 'c'" => "'a' || ('b' || 'c')"
        {
            let state_set = compile_from_str(r#"'a' || 'b' || 'c'"#).unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 9, Jump
- 2
  -> 3, Char 'b'
- 3
  -> 7, Jump
- 4
  -> 5, Char 'c'
- 5
  -> 7, Jump
- 6
  -> 2, Jump
  -> 4, Jump
- 7
  -> 9, Jump
- 8
  -> 0, Jump
  -> 6, Jump
- 9
  -> 11, Match end {0}
> 10
  -> 8, Match start {0}
< 11
# {0}"
            );
        }

        // use "group" to change associativity
        {
            let state_set = compile_from_str(r#"('a' || 'b') || 'c'"#).unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 5, Jump
- 2
  -> 3, Char 'b'
- 3
  -> 5, Jump
- 4
  -> 0, Jump
  -> 2, Jump
- 5
  -> 9, Jump
- 6
  -> 7, Char 'c'
- 7
  -> 9, Jump
- 8
  -> 4, Jump
  -> 6, Jump
- 9
  -> 11, Match end {0}
> 10
  -> 8, Match start {0}
< 11
# {0}"
            );
        }

        // operator precedence
        // "||" is higher than ","
        // "'a', 'b' || 'c', 'd'" => "'a', ('b' || 'c'), 'd'"
        {
            let state_set = compile_from_str(r#"'a', 'b' || 'c', 'd'"#).unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 6, Jump
- 2
  -> 3, Char 'b'
- 3
  -> 7, Jump
- 4
  -> 5, Char 'c'
- 5
  -> 7, Jump
- 6
  -> 2, Jump
  -> 4, Jump
- 7
  -> 8, Jump
- 8
  -> 9, Char 'd'
- 9
  -> 11, Match end {0}
> 10
  -> 0, Match start {0}
< 11
# {0}"
            );
        }

        // use "group" to change precedence
        {
            let state_set = compile_from_str(r#"('a', 'b') || 'c'"#).unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 2, Jump
- 2
  -> 3, Char 'b'
- 3
  -> 7, Jump
- 4
  -> 5, Char 'c'
- 5
  -> 7, Jump
- 6
  -> 0, Jump
  -> 4, Jump
- 7
  -> 9, Match end {0}
> 8
  -> 6, Match start {0}
< 9
# {0}"
            );
        }
    }

    #[test]
    fn test_compile_special_char() {
        {
            let state_set = compile_from_str(r#"'a', char_any"#).unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 2, Jump
- 2
  -> 3, Any char
- 3
  -> 5, Match end {0}
> 4
  -> 0, Match start {0}
< 5
# {0}"
            );
        }
    }

    #[test]
    fn test_compile_preset_charset() {
        // positive preset charset
        {
            let state_set = compile_from_str(r#"'a', char_word, char_space, char_digit"#).unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                r#"- 0
  -> 1, Char 'a'
- 1
  -> 2, Jump
- 2
  -> 3, Charset ['A'..'Z', 'a'..'z', '0'..'9', '_']
- 3
  -> 4, Jump
- 4
  -> 5, Charset [' ', '\t', '\r', '\n']
- 5
  -> 6, Jump
- 6
  -> 7, Charset ['0'..'9']
- 7
  -> 9, Match end {0}
> 8
  -> 0, Match start {0}
< 9
# {0}"#
            );
        }

        // negative preset charset
        {
            let state_set =
                compile_from_str(r#"'a', char_not_word, char_not_space, char_not_digit"#).unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                r#"- 0
  -> 1, Char 'a'
- 1
  -> 2, Jump
- 2
  -> 3, Charset !['A'..'Z', 'a'..'z', '0'..'9', '_']
- 3
  -> 4, Jump
- 4
  -> 5, Charset ![' ', '\t', '\r', '\n']
- 5
  -> 6, Jump
- 6
  -> 7, Charset !['0'..'9']
- 7
  -> 9, Match end {0}
> 8
  -> 0, Match start {0}
< 9
# {0}"#
            );
        }
    }

    #[test]
    fn test_compile_charset() {
        // build with char and range
        {
            let state_set = compile_from_str(r#"['a', '0'..'7']"#).unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Charset ['a', '0'..'7']
- 1
  -> 3, Match end {0}
> 2
  -> 0, Match start {0}
< 3
# {0}"
            );
        }

        // negative charset
        {
            let state_set = compile_from_str(r#"!['a','0'..'7']"#).unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Charset !['a', '0'..'7']
- 1
  -> 3, Match end {0}
> 2
  -> 0, Match start {0}
< 3
# {0}"
            );
        }

        // build with preset charset
        {
            let state_set = compile_from_str(r#"[char_word, char_space]"#).unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                r#"- 0
  -> 1, Charset ['A'..'Z', 'a'..'z', '0'..'9', '_', ' ', '\t', '\r', '\n']
- 1
  -> 3, Match end {0}
> 2
  -> 0, Match start {0}
< 3
# {0}"#
            );
        }

        // nested charset
        {
            let state_set = compile_from_str(r#"['a', ['x'..'z']]"#).unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Charset ['a', 'x'..'z']
- 1
  -> 3, Match end {0}
> 2
  -> 0, Match start {0}
< 3
# {0}"
            );
        }

        // deep nested charset
        {
            let state_set =
                compile_from_str(r#"[['+', '-'], ['0'..'9', ['a'..'f', char_space]]]"#).unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                r#"- 0
  -> 1, Charset ['+', '-', '0'..'9', 'a'..'f', ' ', '\t', '\r', '\n']
- 1
  -> 3, Match end {0}
> 2
  -> 0, Match start {0}
< 3
# {0}"#
            );
        }

        // build with marco
        {
            let state_set = compile_from_str(
                r#"
define(prefix, ['+', '-'])
define(letter, ['a'..'f', char_space])
[prefix, ['0'..'9', letter]]"#,
            )
            .unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                r#"- 0
  -> 1, Charset ['+', '-', '0'..'9', 'a'..'f', ' ', '\t', '\r', '\n']
- 1
  -> 3, Match end {0}
> 2
  -> 0, Match start {0}
< 3
# {0}"#
            );
        }

        // err: negative preset charset in custom charset
        {
            assert!(matches!(
                compile_from_str(r#"[char_not_word]"#),
                Err(Error::Message(_))
            ));
        }

        // err: negative custom charset in custom charset
        // "Unexpected char set element."
        {
            assert!(matches!(
                compile_from_str(r#"['+', !['a'..'f']]"#),
                Err(Error::MessageWithLocation(_, _))
            ));
        }
    }

    #[test]
    fn test_compile_assertion() {
        {
            let state_set = compile_from_str(r#"start, is_bound, 'a', is_not_bound, end"#).unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                r#"- 0
  -> 1, Assertion "is_bound"
- 1
  -> 2, Jump
- 2
  -> 3, Char 'a'
- 3
  -> 4, Jump
- 4
  -> 5, Assertion "is_not_bound"
- 5
  -> 7, Match end {0}
> 6
  -> 0, Match start {0}
< 7
# {0}"#
            );
        }

        // err: assert "start" can only be present at the beginning of expression
        {
            assert!(matches!(
                compile_from_str(r#"'a', start, 'b'"#),
                Err(Error::Message(_))
            ));
        }

        // err: assert "end" can only be present at the end of expression
        {
            assert!(matches!(
                compile_from_str(r#"'a', end, 'b'"#),
                Err(Error::Message(_))
            ));
        }
    }

    #[test]
    fn test_compile_function_call_name() {
        // function call, and rear function call
        {
            let state_set = compile_from_str(r#"name('a', foo), 'b'.name(bar)"#).unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 3, Match end {1}
- 2
  -> 0, Match start {1}
- 3
  -> 6, Jump
- 4
  -> 5, Char 'b'
- 5
  -> 7, Match end {2}
- 6
  -> 4, Match start {2}
- 7
  -> 9, Match end {0}
> 8
  -> 2, Match start {0}
< 9
# {0}
# {1}, foo
# {2}, bar"
            );
        }

        // complex expressions as function call args
        {
            let state_set =
                compile_from_str(r#"name(('a', 'b'), foo), ('x' || 'y').name(bar)"#).unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 2, Jump
- 2
  -> 3, Char 'b'
- 3
  -> 5, Match end {1}
- 4
  -> 0, Match start {1}
- 5
  -> 12, Jump
- 6
  -> 7, Char 'x'
- 7
  -> 11, Jump
- 8
  -> 9, Char 'y'
- 9
  -> 11, Jump
- 10
  -> 6, Jump
  -> 8, Jump
- 11
  -> 13, Match end {2}
- 12
  -> 10, Match start {2}
- 13
  -> 15, Match end {0}
> 14
  -> 4, Match start {0}
< 15
# {0}
# {1}, foo
# {2}, bar"
            );
        }

        // nested function call
        {
            let state_set = compile_from_str(r#"name(name('a', foo), bar)"#).unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 3, Match end {2}
- 2
  -> 0, Match start {2}
- 3
  -> 5, Match end {1}
- 4
  -> 2, Match start {1}
- 5
  -> 7, Match end {0}
> 6
  -> 4, Match start {0}
< 7
# {0}
# {1}, bar
# {2}, foo"
            );
        }

        // chaining function call
        {
            let state_set = compile_from_str(r#"'a'.name(foo).name(bar)"#).unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 3, Match end {2}
- 2
  -> 0, Match start {2}
- 3
  -> 5, Match end {1}
- 4
  -> 2, Match start {1}
- 5
  -> 7, Match end {0}
> 6
  -> 4, Match start {0}
< 7
# {0}
# {1}, bar
# {2}, foo"
            );
        }
    }

    #[test]
    fn test_compile_function_call_index() {
        // function call, and rear function call
        {
            let state_set = compile_from_str(r#"index('a'), 'b'.index()"#).unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 3, Match end {1}
- 2
  -> 0, Match start {1}
- 3
  -> 6, Jump
- 4
  -> 5, Char 'b'
- 5
  -> 7, Match end {2}
- 6
  -> 4, Match start {2}
- 7
  -> 9, Match end {0}
> 8
  -> 2, Match start {0}
< 9
# {0}
# {1}
# {2}"
            );
        }
    }

    // 'backreference' requires the 'name' and 'index' functions
    // to be completed first
    #[test]
    fn test_compile_backreference() {
        {
            let state_set = compile_from_str(r#"'a'.name(foo), 'b', foo"#).unwrap();
            let s = state_set.print_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 3, Match end {1}
- 2
  -> 0, Match start {1}
- 3
  -> 4, Jump
- 4
  -> 5, Char 'b'
- 5
  -> 6, Jump
- 6
  -> 7, Back reference {1}
- 7
  -> 9, Match end {0}
> 8
  -> 2, Match start {0}
< 9
# {0}
# {1}, foo"
            );
        }
    }
}
