// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::{
    ast::{CharRange, CharSet, CharSetElement, Expression, Literal, Program},
    error::Error,
    parser::parse_from_str,
    state::StateSet,
    transition::{
        CharSetItem, CharSetTransition, CharTransition, JumpTransition, SpecialCharTransition,
        StringTransition, Transition,
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
        // todo: add index group
        let result = self.emit_group(&self.program.expressions)?;
        self.state_set.start_node_index = result.in_state_index;
        self.state_set.end_node_index = result.out_state_index;
        Ok(())
    }

    fn emit_expression(&mut self, expression: &Expression) -> Result<EmitResult, Error> {
        let result = match expression {
            Expression::Literal(literal) => self.emit_literal(literal)?,
            Expression::Identifier(_) => todo!(),
            Expression::Assertion(_) => todo!(),
            Expression::Group(expressions) => self.emit_group(expressions)?,
            Expression::FunctionCall(_) => todo!(),
            Expression::Or(left, right) => self.emit_logic_or(left, right)?,
        };

        Ok(result)
    }

    fn emit_group(&mut self, expressions: &[Expression]) -> Result<EmitResult, Error> {
        // connecting two groups of states by adding a 'jump transition'
        //
        //     current                      next
        //  /-----------\               /-----------\
        // --o in  out o-- ==jump trans== --o in  out o--
        //  \-----------/               \-----------/
        //

        let mut results = vec![];
        for expression in expressions {
            results.push(self.emit_expression(expression)?);
        }

        if results.len() == 1 {
            // eliminates the nested group, e.g. '(((...)))'
            let result = results.pop().unwrap();
            Ok(result)
        } else {
            for idx in 0..(results.len() - 1) {
                let current_out_state_index = results[idx].out_state_index;
                let next_in_state_index = results[idx + 1].in_state_index;
                let transition = Transition::Jump(JumpTransition);
                self.state_set.append_transition(
                    current_out_state_index,
                    next_in_state_index,
                    transition,
                );
            }

            let result = EmitResult::new(
                results.first().unwrap().in_state_index,
                results.last().unwrap().out_state_index,
            );

            Ok(result)
        }
    }

    fn emit_logic_or(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> Result<EmitResult, Error> {
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

        Ok(EmitResult::new(in_state_index, out_state_index))
    }

    fn emit_literal(&mut self, literal: &Literal) -> Result<EmitResult, Error> {
        let result = match literal {
            Literal::Char(character) => self.emit_literal_char(*character)?,
            Literal::String(s) => self.emit_literal_string(s)?,
            Literal::CharSet(charset) => self.emit_literal_charset(charset)?,
            Literal::PresetCharSet(name) => self.emit_literal_preset_charset(name)?,
            Literal::Special(_) => self.emit_literal_special_char()?,
        };

        Ok(result)
    }

    fn emit_literal_char(&mut self, character: char) -> Result<EmitResult, Error> {
        let in_state_index = self.state_set.new_state();
        let out_state_index = self.state_set.new_state();

        let transition = Transition::Char(CharTransition::new(character));
        self.state_set
            .append_transition(in_state_index, out_state_index, transition);
        Ok(EmitResult::new(in_state_index, out_state_index))
    }

    fn emit_literal_special_char(&mut self) -> Result<EmitResult, Error> {
        let in_state_index = self.state_set.new_state();
        let out_state_index = self.state_set.new_state();

        let transition = Transition::SpecialChar(SpecialCharTransition);
        self.state_set
            .append_transition(in_state_index, out_state_index, transition);
        Ok(EmitResult::new(in_state_index, out_state_index))
    }

    fn emit_literal_string(&mut self, s: &str) -> Result<EmitResult, Error> {
        let in_state_index = self.state_set.new_state();
        let out_state_index = self.state_set.new_state();

        let transition = Transition::String(StringTransition::new(s));
        self.state_set
            .append_transition(in_state_index, out_state_index, transition);
        Ok(EmitResult::new(in_state_index, out_state_index))
    }

    fn emit_literal_preset_charset(&mut self, name: &str) -> Result<EmitResult, Error> {
        let in_state_index = self.state_set.new_state();
        let out_state_index = self.state_set.new_state();

        let charset_transition = match name {
            "char_word" => CharSetTransition::new_preset_word(),
            "char_not_word" => CharSetTransition::new_preset_not_word(),
            "char_space" => CharSetTransition::new_preset_space(),
            "char_not_space" => CharSetTransition::new_preset_not_space(),
            "char_digit" => CharSetTransition::new_preset_digit(),
            "char_not_digit" => CharSetTransition::new_preset_not_digit(),
            _ => {
                unreachable!()
            }
        };

        let transition = Transition::CharSet(charset_transition);
        self.state_set
            .append_transition(in_state_index, out_state_index, transition);
        Ok(EmitResult::new(in_state_index, out_state_index))
    }

    fn emit_literal_charset(&mut self, charset: &CharSet) -> Result<EmitResult, Error> {
        let in_state_index = self.state_set.new_state();
        let out_state_index = self.state_set.new_state();

        let mut items: Vec<CharSetItem> = vec![];
        append_charset(charset, &mut items)?;

        let charset_transition = CharSetTransition::new(items, charset.negative);
        let transition = Transition::CharSet(charset_transition);
        self.state_set
            .append_transition(in_state_index, out_state_index, transition);
        Ok(EmitResult::new(in_state_index, out_state_index))
    }
}

fn append_preset_charset_positive_only_by_name(
    name: &str,
    items: &mut Vec<CharSetItem>,
) -> Result<(), Error> {
    match name {
        "char_word" => {
            CharSetItem::add_preset_word(items);
        }
        "char_space" => {
            CharSetItem::add_preset_space(items);
        }
        "char_digit" => {
            CharSetItem::add_preset_digit(items);
        }
        "char_not_word" | "char_not_space" | "char_not_digit" => {
            return Err(Error::Message(format!(
                "Can not append negative preset charset \"{}\" into charset.",
                name
            )));
        }
        _ => {
            unreachable!()
        }
    }

    Ok(())
}

fn append_charset(charset: &CharSet, items: &mut Vec<CharSetItem>) -> Result<(), Error> {
    for element in &charset.elements {
        match element {
            CharSetElement::Char(c) => CharSetItem::add_char(items, *c),
            CharSetElement::CharRange(CharRange {
                start,
                end_included,
            }) => CharSetItem::add_range(items, *start, *end_included),
            CharSetElement::PresetCharSet(name) => {
                append_preset_charset_positive_only_by_name(name, items)?;
            }
            CharSetElement::CharSet(custom_charset) => {
                assert!(!custom_charset.negative);
                append_charset(custom_charset, items)?;
            }
        }
    }

    Ok(())
}

struct EmitResult {
    in_state_index: usize,
    out_state_index: usize,
}

impl EmitResult {
    fn new(in_state_index: usize, out_state_index: usize) -> Self {
        EmitResult {
            in_state_index,
            out_state_index,
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_str_eq;

    use super::compile_from_str;

    #[test]
    fn test_compile_char() {
        {
            let state_set = compile_from_str(r#"'a'"#).unwrap();
            let s = state_set.generate_states_and_transitions_text();

            assert_str_eq!(
                s,
                "\
> 0
  -> 1, Char 'a'
< 1"
            );
        }

        {
            let state_set = compile_from_str(r#"'a', 'b', 'c'"#).unwrap();
            let s = state_set.generate_states_and_transitions_text();

            assert_str_eq!(
                s,
                "\
> 0
  -> 1, Char 'a'
- 1
  -> 2, Jump
- 2
  -> 3, Char 'b'
- 3
  -> 4, Jump
- 4
  -> 5, Char 'c'
< 5"
            );
        }

        {
            let state_set = compile_from_str(r#"'a',('b','c'), 'd'"#).unwrap();
            let s = state_set.generate_states_and_transitions_text();

            assert_str_eq!(
                s,
                "\
> 0
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
< 7"
            );
        }

        {
            let state_set = compile_from_str(r#"'a',('b', ('c', 'd'), 'e'), 'f'"#).unwrap();
            let s = state_set.generate_states_and_transitions_text();

            assert_str_eq!(
                s,
                "\
> 0
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
< 11"
            );
        }
    }

    #[test]
    fn test_compile_logic_or() {
        {
            let state_set = compile_from_str(r#"'a' || 'b'"#).unwrap();
            let s = state_set.generate_states_and_transitions_text();

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
> 4
  -> 0, Jump
  -> 2, Jump
< 5"
            );
        }

        {
            // "'a', 'b' || 'c', 'd'" == "'a', ('b' || 'c'), 'd'"
            let state_set = compile_from_str(r#"'a', 'b' || 'c', 'd'"#).unwrap();
            let s = state_set.generate_states_and_transitions_text();

            assert_str_eq!(
                s,
                "\
> 0
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
< 9"
            );
        }

        {
            // "'a', 'b' || 'c', 'd'" == "'a', ('b' || 'c'), 'd'"
            assert_str_eq!(
                compile_from_str(r#"'a', 'b' || 'c', 'd'"#)
                    .unwrap()
                    .generate_states_and_transitions_text(),
                compile_from_str(r#"'a', ('b' || 'c'), 'd'"#)
                    .unwrap()
                    .generate_states_and_transitions_text()
            );
        }

        // associativity

        {
            let state_set = compile_from_str(r#"'a' || 'b' || 'c'"#).unwrap();
            let s = state_set.generate_states_and_transitions_text();

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
> 8
  -> 0, Jump
  -> 6, Jump
< 9"
            );
        }
    }

    #[test]
    fn test_compile_special_char() {
        // todo
    }

    #[test]
    fn test_compile_preset_charset() {
        // todo
    }

    #[test]
    fn test_compile_charset() {
        // todo
    }
}
