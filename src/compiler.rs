// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::{char::MAX, usize};

use crate::{
    ast::{
        AssertionName, CharRange, CharSet, CharSetElement, Expression, FunctionCall,
        FunctionCallArg, FunctionName, Literal, PresetCharSetName, Program,
    },
    error::Error,
    image::{Image, StateSet},
    parser::parse_from_str,
    transition::{
        add_char, add_preset_digit, add_preset_space, add_preset_word, add_range,
        AssertionTransition, BackReferenceTransition, CharSetItem, CharSetTransition,
        CharTransition, CounterCheckTransition, CounterIncTransition, CounterResetTransition,
        JumpTransition, MatchEndTransition, MatchStartTransition, RepetitionAnchorTransition,
        RepetitionType, SpecialCharTransition, StringTransition, Transition,
    },
};

pub fn compile(program: &Program) -> Result<Image, Error> {
    let mut image = Image::new();
    let stateset_index = image.new_stateset();

    let mut compiler = Compiler::new(program, &mut image, stateset_index);
    compiler.compile()?;

    Ok(image)
}

pub fn compile_from_str(s: &str) -> Result<Image, Error> {
    let program = parse_from_str(s)?;
    compile(&program)
}

pub struct Compiler<'a> {
    program: &'a Program,
    // stateset: &'a mut StateSet,
    image: &'a mut Image,
    stateset_index: usize, // index of the current stateset
}

impl<'a> Compiler<'a> {
    fn new(
        program: &'a Program,
        /* stateset: &'a mut StateSet*/ image: &'a mut Image,
        stateset_index: usize,
    ) -> Self {
        Compiler {
            program,
            image,
            stateset_index,
        }
    }

    fn get_current_stateset(&mut self) -> &mut StateSet {
        self.image.get_stateset_ref_mut(self.stateset_index)
    }

    fn compile(&mut self) -> Result<(), Error> {
        self.emit_program(&self.program)
    }

    fn emit_program(&mut self, program: &Program) -> Result<(), Error> {
        // the `Program` node is actually a `Group` which omits the parentheses,
        // in addition, there may be the 'start' and 'end' assertions.

        // create the index 0 match for the program
        let match_index = self.image.new_match(None);

        let expressions = &program.expressions;

        let mut fixed_start = false;
        let mut fixed_end = false;
        let mut ports = vec![];

        for (expression_index, expression) in expressions.iter().enumerate() {
            if matches!(expression, Expression::Assertion(AssertionName::Start)) {
                if expression_index == 0 {
                    // skips the 'start' assertion emitting.
                    fixed_start = true;
                } else {
                    return Err(Error::Message(
                        "The assertion \"start\" can only be present at the beginning of expression."
                            .to_owned(),
                    ));
                }
            } else if matches!(expression, Expression::Assertion(AssertionName::End)) {
                if expression_index == expressions.len() - 1 {
                    // skips the 'end' assertion emitting.
                    fixed_end = true;
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

        let stateset = self.get_current_stateset();

        let program_port = if ports.is_empty() {
            // empty expression
            let relay_state_index = stateset.new_state();
            Port::new(relay_state_index, relay_state_index)
        } else if ports.len() == 1 {
            // single expression
            ports.pop().unwrap()
        } else {
            for idx in 0..(ports.len() - 1) {
                let current_out_state_index = ports[idx].out_state_index;
                let next_in_state_index = ports[idx + 1].in_state_index;
                let transition = Transition::Jump(JumpTransition);
                stateset.append_transition(
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

        //     match start     comp        match end
        //        trans    /-----------\    trans
        //  --o===========--o in  out o--==========o--
        //   in            \-----------/           out

        let in_state_index = stateset.new_state();
        let out_state_index = stateset.new_state();

        let match_start_transition = MatchStartTransition::new(match_index);
        let match_end_transition = MatchEndTransition::new(match_index);

        stateset.append_transition(
            in_state_index,
            program_port.in_state_index,
            Transition::MatchStart(match_start_transition),
        );

        stateset.append_transition(
            program_port.out_state_index,
            out_state_index,
            Transition::MatchEnd(match_end_transition),
        );

        let match_port = Port::new(in_state_index, out_state_index);

        // save the program ports
        stateset.start_node_index = match_port.in_state_index;
        stateset.end_node_index = match_port.out_state_index;
        stateset.fixed_start = fixed_start;
        stateset.fixed_end = fixed_end;

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
        /*
         * the "group" of ANREG is different from the "group" of
         * ordinary regular expressions.
         * the "group" of ANREG is just a series of parenthesized patterns
         * that are not captured unless called by the 'name' or 'index' function.
         * e.g.
         * ANREG `('a', 'b', char_word+)` is equivalent to oridinary regex `ab\w+`
         * the "group" of ANREG is used to group patterns and
         * change operator precedence and associativity
         */

        // connecting two groups of states by adding a 'jump transition'
        //
        //     prev                      next
        //  /-----------\    jump    /-----------\
        // --o in  out o--==========--o in  out o--
        //  \-----------/            \-----------/

        let mut ports = vec![];
        for expression in expressions {
            ports.push(self.emit_expression(expression)?);
        }

        let stateset = self.get_current_stateset();

        let port = if ports.len() == 0 {
            // empty group
            let relay_state_index = stateset.new_state();
            Port::new(relay_state_index, relay_state_index)
        } else if ports.len() == 1 {
            // single expression.
            // maybe a group also, so return the underlay port directly
            // to eliminates the nested group, e.g. '(((...)))'.
            ports.pop().unwrap()
        } else {
            for idx in 0..(ports.len() - 1) {
                let current_out_state_index = ports[idx].out_state_index;
                let next_in_state_index = ports[idx + 1].in_state_index;
                let transition = Transition::Jump(JumpTransition);
                stateset.append_transition(
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

        Ok(port)
    }

    fn emit_logic_or(&mut self, left: &Expression, right: &Expression) -> Result<Port, Error> {
        //                    left
        //         jump   /-----------\   jump
        //      /========--o in  out o--========\
        //  in  |         \-----------/         |  out
        // --o==|                               |==o--
        //      |             right             |
        //      |         /-----------\         |
        //      \========--o in  out o--========/
        //         jump   \-----------/   jump

        let left_port = self.emit_expression(left)?;
        let right_port = self.emit_expression(right)?;

        let stateset = self.get_current_stateset();

        let in_state_index = stateset.new_state();
        let out_state_index = stateset.new_state();

        stateset.append_transition(
            in_state_index,
            left_port.in_state_index,
            Transition::Jump(JumpTransition),
        );

        stateset.append_transition(
            in_state_index,
            right_port.in_state_index,
            Transition::Jump(JumpTransition),
        );

        stateset.append_transition(
            left_port.out_state_index,
            out_state_index,
            Transition::Jump(JumpTransition),
        );

        stateset.append_transition(
            right_port.out_state_index,
            out_state_index,
            Transition::Jump(JumpTransition),
        );

        Ok(Port::new(in_state_index, out_state_index))
    }

    fn emit_function_call(&mut self, function_call: &FunctionCall) -> Result<Port, Error> {
        let expression = &function_call.expression;
        let args = &function_call.args;

        let is_lazy = match &function_call.name {
            FunctionName::OptionalLazy
            | FunctionName::OneOrMoreLazy
            | FunctionName::ZeroOrMoreLazy
            | FunctionName::RepeatRangeLazy
            | FunctionName::AtLeastLazy => true,
            _ => false,
        };

        match &function_call.name {
            // Quantifier
            FunctionName::Optional | FunctionName::OptionalLazy => {
                self.emit_optional(expression, is_lazy)
            }
            FunctionName::OneOrMore | FunctionName::OneOrMoreLazy => {
                // {1,MAX}
                self.emit_repeat_range(expression, 1, usize::MAX, is_lazy)
            }
            FunctionName::ZeroOrMore | FunctionName::ZeroOrMoreLazy => {
                // {0,MAX}
                // optional + one_or_more
                let port = self.emit_repeat_range(expression, 1, usize::MAX, is_lazy)?;
                self.continue_emit_optional(port, is_lazy)
            }
            FunctionName::Repeat => {
                let times = if let FunctionCallArg::Number(n) = &args[0] {
                    *n
                } else {
                    unreachable!()
                };

                if times == 0 {
                    // {0}
                    // return a new empty transition
                    self.emit_empty()
                } else if times == 1 {
                    // {1}
                    // return the expression without repetition
                    self.emit_expression(expression)
                } else {
                    // {m}
                    // repeat specified
                    self.emit_repeat_specified(expression, times)
                }
            }
            FunctionName::RepeatRange | FunctionName::RepeatRangeLazy => {
                let from = if let FunctionCallArg::Number(n) = &args[0] {
                    *n
                } else {
                    unreachable!()
                };

                let to = if let FunctionCallArg::Number(n) = &args[1] {
                    *n
                } else {
                    unreachable!()
                };

                if from > to {
                    return Err(Error::Message(
                        "Repeated range values should be from small to large.".to_owned(),
                    ));
                }

                if from == 0 {
                    if to == 0 {
                        // {0,0}
                        // return a new empty transition
                        self.emit_empty()
                    } else if to == 1 {
                        // {0,1}
                        // optional
                        self.emit_optional(expression, is_lazy)
                    } else {
                        // {0,m}
                        // optional + range
                        let port = self.emit_repeat_range(expression, 1, to, is_lazy)?;
                        self.continue_emit_optional(port, is_lazy)
                    }
                } else {
                    if to == 1 {
                        // {1,1}
                        // return the expression without repetition
                        self.emit_expression(expression)
                    } else if from == to {
                        // {m,m}
                        // repeat specified
                        self.emit_repeat_specified(expression, from)
                    } else {
                        // {m,n}
                        // repeat range
                        self.emit_repeat_range(expression, from, to, is_lazy)
                    }
                }
            }
            FunctionName::AtLeast | FunctionName::AtLeastLazy => {
                let from = if let FunctionCallArg::Number(n) = &args[0] {
                    *n
                } else {
                    unreachable!()
                };

                if from == 0 {
                    // {0,MAX}
                    // optional + one_or_more
                    let port = self.emit_repeat_range(expression, 1, usize::MAX, is_lazy)?;
                    self.continue_emit_optional(port, is_lazy)
                } else {
                    // {m,MAX}
                    // repeat range
                    self.emit_repeat_range(expression, from, usize::MAX, is_lazy)
                }
            }

            // Assertions
            FunctionName::IsBefore => todo!(),
            FunctionName::IsAfter => todo!(),
            FunctionName::IsNotBefore => todo!(),
            FunctionName::IsNotAfter => todo!(),

            // Capture
            FunctionName::Name => self.emit_capture_name(expression, args),
            FunctionName::Index => self.emit_capture_index(expression),
        }
    }

    fn emit_empty(&mut self) -> Result<Port, Error> {
        let stateset = self.get_current_stateset();

        /*
        let in_state_index = stateset.new_state();
        let out_state_index = stateset.new_state();
        let transition = Transition::Jump(JumpTransition);
        stateset.append_transition(in_state_index, out_state_index, transition);
        Ok(Port::new(in_state_index, out_state_index))
        */

        let relay_state_index = stateset.new_state();
        Ok(Port::new(relay_state_index, relay_state_index))
    }

    fn emit_literal(&mut self, literal: &Literal) -> Result<Port, Error> {
        let port = match literal {
            Literal::Char(character) => self.emit_literal_char(*character)?,
            Literal::String(s) => self.emit_literal_string(s)?,
            Literal::CharSet(charset) => self.emit_literal_charset(charset)?,
            Literal::PresetCharSet(name) => self.emit_literal_preset_charset(name)?,
            Literal::Special(_) => self.emit_literal_special_char()?,
        };

        Ok(port)
    }

    fn emit_literal_char(&mut self, character: char) -> Result<Port, Error> {
        let stateset = self.get_current_stateset();
        let in_state_index = stateset.new_state();
        let out_state_index = stateset.new_state();

        let transition = Transition::Char(CharTransition::new(character));
        stateset.append_transition(in_state_index, out_state_index, transition);
        Ok(Port::new(in_state_index, out_state_index))
    }

    fn emit_literal_special_char(&mut self) -> Result<Port, Error> {
        let stateset = self.get_current_stateset();
        let in_state_index = stateset.new_state();
        let out_state_index = stateset.new_state();

        let transition = Transition::SpecialChar(SpecialCharTransition);
        stateset.append_transition(in_state_index, out_state_index, transition);
        Ok(Port::new(in_state_index, out_state_index))
    }

    fn emit_literal_string(&mut self, s: &str) -> Result<Port, Error> {
        let stateset = self.get_current_stateset();
        let in_state_index = stateset.new_state();
        let out_state_index = stateset.new_state();

        let transition = Transition::String(StringTransition::new(s));
        stateset.append_transition(in_state_index, out_state_index, transition);
        Ok(Port::new(in_state_index, out_state_index))
    }

    fn emit_literal_preset_charset(&mut self, name: &PresetCharSetName) -> Result<Port, Error> {
        let stateset = self.get_current_stateset();
        let in_state_index = stateset.new_state();
        let out_state_index = stateset.new_state();

        let charset_transition = match name {
            PresetCharSetName::CharWord => CharSetTransition::new_preset_word(),
            PresetCharSetName::CharNotWord => CharSetTransition::new_preset_not_word(),
            PresetCharSetName::CharSpace => CharSetTransition::new_preset_space(),
            PresetCharSetName::CharNotSpace => CharSetTransition::new_preset_not_space(),
            PresetCharSetName::CharDigit => CharSetTransition::new_preset_digit(),
            PresetCharSetName::CharNotDigit => CharSetTransition::new_preset_not_digit(),
        };

        let transition = Transition::CharSet(charset_transition);
        stateset.append_transition(in_state_index, out_state_index, transition);
        Ok(Port::new(in_state_index, out_state_index))
    }

    fn emit_literal_charset(&mut self, charset: &CharSet) -> Result<Port, Error> {
        let stateset = self.get_current_stateset();
        let in_state_index = stateset.new_state();
        let out_state_index = stateset.new_state();

        let mut items: Vec<CharSetItem> = vec![];
        Compiler::append_charset(charset, &mut items)?;

        let charset_transition = CharSetTransition::new(items, charset.negative);
        let transition = Transition::CharSet(charset_transition);
        stateset.append_transition(in_state_index, out_state_index, transition);
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
        let stateset = self.get_current_stateset();
        let in_state_index = stateset.new_state();
        let out_state_index = stateset.new_state();

        let assertion_transition = AssertionTransition::new(*name);

        let transition = Transition::Assertion(assertion_transition);
        stateset.append_transition(in_state_index, out_state_index, transition);
        Ok(Port::new(in_state_index, out_state_index))
    }

    fn emit_backreference(&mut self, name: &str) -> Result<Port, Error> {
        let match_index_option = self.image.find_match_index(name);
        let match_index = if let Some(i) = match_index_option {
            i
        } else {
            return Err(Error::Message(format!(
                "Cannot find the match with name: \"{}\".",
                name
            )));
        };

        let stateset = self.get_current_stateset();

        let in_state_index = stateset.new_state();
        let out_state_index = stateset.new_state();

        let transition = Transition::BackReference(BackReferenceTransition::new(match_index));
        stateset.append_transition(in_state_index, out_state_index, transition);
        Ok(Port::new(in_state_index, out_state_index))
    }

    fn emit_capture_name(
        &mut self,
        expression: &Expression,
        args: &[FunctionCallArg],
    ) -> Result<Port, Error> {
        let name = if let FunctionCallArg::Identifier(s) = &args[0] {
            s.to_owned()
        } else {
            unreachable!();
        };

        self.continue_emit_capture(expression, Some(name))
    }

    fn emit_capture_index(&mut self, expression: &Expression) -> Result<Port, Error> {
        self.continue_emit_capture(expression, None)
    }

    fn continue_emit_capture(
        &mut self,
        expression: &Expression,
        name_option: Option<String>,
    ) -> Result<Port, Error> {
        let match_index = self.image.new_match(name_option);
        let port = self.emit_expression(expression)?;

        //     match start      comp        match end
        //   in   trans    /-----------\    trans  out
        //  --o===========--o in  out o--==========o--
        //                 \-----------/

        let stateset = self.get_current_stateset();
        let in_state_index = stateset.new_state();
        let out_state_index = stateset.new_state();

        let match_start_transition = MatchStartTransition::new(match_index);
        let match_end_transition = MatchEndTransition::new(match_index);

        stateset.append_transition(
            in_state_index,
            port.in_state_index,
            Transition::MatchStart(match_start_transition),
        );

        stateset.append_transition(
            port.out_state_index,
            out_state_index,
            Transition::MatchEnd(match_end_transition),
        );

        Ok(Port::new(in_state_index, out_state_index))
    }

    fn emit_optional(&mut self, expression: &Expression, is_lazy: bool) -> Result<Port, Error> {
        // greedy optional
        //
        //                    comp
        //   in     jmp  /-----------\  jmp out
        //  --o|o--=====--o in  out o--=====o--
        //     |o--\     \-----------/      ^
        //      ^  |                        |
        //      |  \========================/
        //      |           jump trans
        //      \ switch these two trans for lazy matching

        let port = self.emit_expression(expression)?;
        self.continue_emit_optional(port, is_lazy)
    }

    fn continue_emit_optional(&mut self, port: Port, is_lazy: bool) -> Result<Port, Error> {
        let stateset = self.get_current_stateset();

        let in_state_index = stateset.new_state();
        let out_state_index = stateset.new_state();

        if is_lazy {
            stateset.append_transition(
                in_state_index,
                out_state_index,
                Transition::Jump(JumpTransition),
            );
        }

        stateset.append_transition(
            in_state_index,
            port.in_state_index,
            Transition::Jump(JumpTransition),
        );

        stateset.append_transition(
            port.out_state_index,
            out_state_index,
            Transition::Jump(JumpTransition),
        );

        if !is_lazy {
            stateset.append_transition(
                in_state_index,
                out_state_index,
                Transition::Jump(JumpTransition),
            );
        }

        Ok(Port::new(in_state_index, out_state_index))
    }

    fn emit_repeat_specified(
        &mut self,
        expression: &Expression,
        times: usize,
    ) -> Result<Port, Error> {
        assert!(times > 1);
        self.continue_emit_repetition(expression, RepetitionType::Specified(times), true)
    }

    fn emit_repeat_range(
        &mut self,
        expression: &Expression,
        from: usize,
        to: usize,
        is_lazy: bool,
    ) -> Result<Port, Error> {
        assert!(from > 0 && to > 1 && to > from);
        self.continue_emit_repetition(expression, RepetitionType::Range(from, to), is_lazy)
    }

    fn continue_emit_repetition(
        &mut self,
        expression: &Expression,
        repetition_type: RepetitionType,
        is_lazy: bool,
    ) -> Result<Port, Error> {
        // lazy repetition
        //                                         counter
        //                             comp      | inc
        //   in        left  jump  /-----------\ v trans  right     out
        //  --o----------o--======--o in  out o-----------o|o-------o--
        //      ^ cnter  ^         \-----------/           |o-\  ^ counter
        //      | reset  |                                    |  | check
        //        trans  \====================================/    trans
        //                               ^                    ^
        //  repetition anchor trans for  |                    |
        //  greedy range repetition,     |                    |
        //  jump trans for other reps.   /                    |
        //     switch these two trans pos for greedy matching |

        let port = self.emit_expression(expression)?;
        let counter_index = self.image.new_counter();

        let stateset = self.get_current_stateset();

        let in_state_index = stateset.new_state();
        let left_state_index = stateset.new_state();
        let right_state_index = stateset.new_state();
        let out_state_index = stateset.new_state();

        stateset.append_transition(
            in_state_index,
            left_state_index,
            Transition::CounterReset(CounterResetTransition::new(counter_index)),
        );

        stateset.append_transition(
            left_state_index,
            port.in_state_index,
            Transition::Jump(JumpTransition),
        );

        stateset.append_transition(
            port.out_state_index,
            right_state_index,
            Transition::CounterInc(CounterIncTransition::new(counter_index)),
        );

        let goto_check_and_exit = |ss: &mut StateSet| {
            ss.append_transition(
                right_state_index,
                out_state_index,
                Transition::CounterCheck(CounterCheckTransition::new(
                    counter_index,
                    repetition_type,
                )),
            );
        };

        let goto_redo = |ss: &mut StateSet| {
            let threshold_option = if is_lazy {
                None
            } else {
                if let RepetitionType::Range(from, _) = repetition_type {
                    Some(from)
                } else {
                    None
                }
            };

            let transition = if let Some(threshold) = threshold_option {
                Transition::RepetionAnchor(RepetitionAnchorTransition::new(
                    counter_index,
                    threshold,
                ))
            } else {
                Transition::Jump(JumpTransition)
            };

            ss.append_transition(right_state_index, left_state_index, transition);
        };

        if is_lazy {
            goto_check_and_exit(stateset);
            goto_redo(stateset);
        } else {
            goto_redo(stateset);
            goto_check_and_exit(stateset);
        }

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
            let image = compile_from_str(r#"'a'"#).unwrap();
            let s = image.get_image_text();

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
            let image = compile_from_str(r#"'a', 'b', 'c'"#).unwrap();
            let s = image.get_image_text();

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
            let image = compile_from_str(r#"'a',('b','c'), 'd'"#).unwrap();
            let s = image.get_image_text();

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
            let image = compile_from_str(r#"'a',('b', ('c', 'd'), 'e'), 'f'"#).unwrap();
            let s = image.get_image_text();

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
            let image = compile_from_str(r#"'a' || 'b'"#).unwrap();
            let s = image.get_image_text();

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
            let image = compile_from_str(r#"'a' || 'b' || 'c'"#).unwrap();
            let s = image.get_image_text();

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
            let image = compile_from_str(r#"('a' || 'b') || 'c'"#).unwrap();
            let s = image.get_image_text();

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
            let image = compile_from_str(r#"'a', 'b' || 'c', 'd'"#).unwrap();
            let s = image.get_image_text();

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
            let image = compile_from_str(r#"('a', 'b') || 'c'"#).unwrap();
            let s = image.get_image_text();

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
            let image = compile_from_str(r#"'a', char_any"#).unwrap();
            let s = image.get_image_text();

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
            let image = compile_from_str(r#"'a', char_word, char_space, char_digit"#).unwrap();
            let s = image.get_image_text();

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
            let image =
                compile_from_str(r#"'a', char_not_word, char_not_space, char_not_digit"#).unwrap();
            let s = image.get_image_text();

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
            let image = compile_from_str(r#"['a', '0'..'7']"#).unwrap();
            let s = image.get_image_text();

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
            let image = compile_from_str(r#"!['a','0'..'7']"#).unwrap();
            let s = image.get_image_text();

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
            let image = compile_from_str(r#"[char_word, char_space]"#).unwrap();
            let s = image.get_image_text();

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
            let image = compile_from_str(r#"['a', ['x'..'z']]"#).unwrap();
            let s = image.get_image_text();

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
            let image =
                compile_from_str(r#"[['+', '-'], ['0'..'9', ['a'..'f', char_space]]]"#).unwrap();
            let s = image.get_image_text();

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
            let image = compile_from_str(
                r#"
define(prefix, ['+', '-'])
define(letter, ['a'..'f', char_space])
[prefix, ['0'..'9', letter]]"#,
            )
            .unwrap();
            let s = image.get_image_text();

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
            let mut image = compile_from_str(r#"start, is_bound, 'a'"#).unwrap();
            let s = image.get_image_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Assertion \"is_bound\"
- 1
  -> 2, Jump
- 2
  -> 3, Char 'a'
- 3
  -> 5, Match end {0}
> 4
  -> 0, Match start {0}
< 5
# {0}"
            );

            // check the 'fixed_start' and 'fixed_end'
            assert!(image.get_stateset_ref_mut(0).fixed_start);
            assert!(!image.get_stateset_ref_mut(0).fixed_end);
        }

        {
            let mut image = compile_from_str(r#"is_not_bound, 'a', end"#).unwrap();
            let s = image.get_image_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Assertion \"is_not_bound\"
- 1
  -> 2, Jump
- 2
  -> 3, Char 'a'
- 3
  -> 5, Match end {0}
> 4
  -> 0, Match start {0}
< 5
# {0}"
            );

            // check the 'fixed_start' and 'fixed_end'
            assert!(!image.get_stateset_ref_mut(0).fixed_start);
            assert!(image.get_stateset_ref_mut(0).fixed_end);
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
    fn test_compile_capture_name() {
        // function call, and rear function call
        {
            let image = compile_from_str(r#"name('a', foo), 'b'.name(bar)"#).unwrap();
            let s = image.get_image_text();

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
            let image =
                compile_from_str(r#"name(('a', 'b'), foo), ('x' || 'y').name(bar)"#).unwrap();
            let s = image.get_image_text();

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
            let image = compile_from_str(r#"name(name('a', foo), bar)"#).unwrap();
            let s = image.get_image_text();

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
            let image = compile_from_str(r#"'a'.name(foo).name(bar)"#).unwrap();
            let s = image.get_image_text();

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
    fn test_compile_capture_index() {
        // function call, and rear function call
        {
            let image = compile_from_str(r#"index('a'), 'b'.index()"#).unwrap();
            let s = image.get_image_text();

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
            let image = compile_from_str(r#"'a'.name(foo), 'b', foo"#).unwrap();
            let s = image.get_image_text();

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

    #[test]
    fn test_compile_optional() {
        // greedy
        {
            let image = compile_from_str(r#"'a'?"#).unwrap();
            let s = image.get_image_text();
            // println!("{}", s);

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 3, Jump
- 2
  -> 0, Jump
  -> 3, Jump
- 3
  -> 5, Match end {0}
> 4
  -> 2, Match start {0}
< 5
# {0}"
            );
        }

        // lazy
        {
            let image = compile_from_str(r#"'a'??"#).unwrap();
            let s = image.get_image_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 3, Jump
- 2
  -> 3, Jump
  -> 0, Jump
- 3
  -> 5, Match end {0}
> 4
  -> 2, Match start {0}
< 5
# {0}"
            );
        }
    }

    #[test]
    fn test_compile_notations() {
        // optional
        {
            let image = compile_from_str(r#"'a'?"#).unwrap();
            let s = image.get_image_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 3, Jump
- 2
  -> 0, Jump
  -> 3, Jump
- 3
  -> 5, Match end {0}
> 4
  -> 2, Match start {0}
< 5
# {0}"
            );
        }

        // lazy optional
        {
            let image = compile_from_str(r#"'a'??"#).unwrap();
            let s = image.get_image_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 3, Jump
- 2
  -> 3, Jump
  -> 0, Jump
- 3
  -> 5, Match end {0}
> 4
  -> 2, Match start {0}
< 5
# {0}"
            );
        }

        // one or more
        {
            let image = compile_from_str(r#"'a'+"#).unwrap();
            let s = image.get_image_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 4, Counter inc <0>
- 2
  -> 3, Counter reset <0>
- 3
  -> 0, Jump
- 4
  -> 3, Repetition anchor <0>, threshold 1
  -> 5, Counter check <0>, from 1, to MAX
- 5
  -> 7, Match end {0}
> 6
  -> 2, Match start {0}
< 7
# {0}"
            );
        }

        // lazy one or more
        {
            let image = compile_from_str(r#"'a'+?"#).unwrap();
            let s = image.get_image_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 4, Counter inc <0>
- 2
  -> 3, Counter reset <0>
- 3
  -> 0, Jump
- 4
  -> 5, Counter check <0>, from 1, to MAX
  -> 3, Jump
- 5
  -> 7, Match end {0}
> 6
  -> 2, Match start {0}
< 7
# {0}"
            );
        }

        // zero or more
        {
            let image = compile_from_str(r#"'a'*"#).unwrap();
            let s = image.get_image_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 4, Counter inc <0>
- 2
  -> 3, Counter reset <0>
- 3
  -> 0, Jump
- 4
  -> 3, Repetition anchor <0>, threshold 1
  -> 5, Counter check <0>, from 1, to MAX
- 5
  -> 7, Jump
- 6
  -> 2, Jump
  -> 7, Jump
- 7
  -> 9, Match end {0}
> 8
  -> 6, Match start {0}
< 9
# {0}"
            );
        }

        // lazy zero or more
        {
            let image = compile_from_str(r#"'a'*?"#).unwrap();
            let s = image.get_image_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 4, Counter inc <0>
- 2
  -> 3, Counter reset <0>
- 3
  -> 0, Jump
- 4
  -> 5, Counter check <0>, from 1, to MAX
  -> 3, Jump
- 5
  -> 7, Jump
- 6
  -> 7, Jump
  -> 2, Jump
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
    fn test_compile_repeat_specified() {
        // repeat 1+
        {
            let image = compile_from_str(r#"'a'{2}"#).unwrap();
            let s = image.get_image_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 4, Counter inc <0>
- 2
  -> 3, Counter reset <0>
- 3
  -> 0, Jump
- 4
  -> 5, Counter check <0>, times 2
  -> 3, Jump
- 5
  -> 7, Match end {0}
> 6
  -> 2, Match start {0}
< 7
# {0}"
            );
        }

        // repeat 1
        {
            let image = compile_from_str(r#"'a'{1}"#).unwrap();
            let s = image.get_image_text();

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

        // repeat 0
        {
            let image = compile_from_str(r#"'a'{0}"#).unwrap();
            let s = image.get_image_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 2, Match end {0}
> 1
  -> 0, Match start {0}
< 2
# {0}"
            );
        }
    }

    #[test]
    fn test_compile_repeat_range() {
        // greedy
        {
            let image = compile_from_str(r#"'a'{3,5}"#).unwrap();
            let s = image.get_image_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 4, Counter inc <0>
- 2
  -> 3, Counter reset <0>
- 3
  -> 0, Jump
- 4
  -> 3, Repetition anchor <0>, threshold 3
  -> 5, Counter check <0>, from 3, to 5
- 5
  -> 7, Match end {0}
> 6
  -> 2, Match start {0}
< 7
# {0}"
            );
        }

        // lazy
        {
            let image = compile_from_str(r#"'a'{3,5}?"#).unwrap();
            let s = image.get_image_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 4, Counter inc <0>
- 2
  -> 3, Counter reset <0>
- 3
  -> 0, Jump
- 4
  -> 5, Counter check <0>, from 3, to 5
  -> 3, Jump
- 5
  -> 7, Match end {0}
> 6
  -> 2, Match start {0}
< 7
# {0}"
            );
        }

        // {m, m}
        {
            assert_str_eq!(
                compile_from_str(r#"'a'{3,3}"#).unwrap().get_image_text(),
                compile_from_str(r#"'a'{3}"#).unwrap().get_image_text()
            )
        }

        // {1, 1}
        {
            assert_str_eq!(
                compile_from_str(r#"'a'{1,1}"#).unwrap().get_image_text(),
                compile_from_str(r#"'a'"#).unwrap().get_image_text()
            )
        }

        // {0, m}
        {
            let image = compile_from_str(r#"'a'{0,5}"#).unwrap();
            let s = image.get_image_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 4, Counter inc <0>
- 2
  -> 3, Counter reset <0>
- 3
  -> 0, Jump
- 4
  -> 3, Repetition anchor <0>, threshold 1
  -> 5, Counter check <0>, from 1, to 5
- 5
  -> 7, Jump
- 6
  -> 2, Jump
  -> 7, Jump
- 7
  -> 9, Match end {0}
> 8
  -> 6, Match start {0}
< 9
# {0}"
            );
        }

        // {0, m} lazy
        {
            let image = compile_from_str(r#"'a'{0,5}?"#).unwrap();
            let s = image.get_image_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 4, Counter inc <0>
- 2
  -> 3, Counter reset <0>
- 3
  -> 0, Jump
- 4
  -> 5, Counter check <0>, from 1, to 5
  -> 3, Jump
- 5
  -> 7, Jump
- 6
  -> 7, Jump
  -> 2, Jump
- 7
  -> 9, Match end {0}
> 8
  -> 6, Match start {0}
< 9
# {0}"
            );
        }

        // {0, 1}
        {
            assert_str_eq!(
                compile_from_str(r#"'a'{0,1}"#).unwrap().get_image_text(),
                compile_from_str(r#"'a'?"#).unwrap().get_image_text()
            )
        }

        // {0, 1} lazy
        {
            assert_str_eq!(
                compile_from_str(r#"'a'{0,1}?"#).unwrap().get_image_text(),
                compile_from_str(r#"'a'??"#).unwrap().get_image_text()
            )
        }

        // {0, 0}
        {
            let image = compile_from_str(r#"'a'{0,0}"#).unwrap();
            let s = image.get_image_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 2, Match end {0}
> 1
  -> 0, Match start {0}
< 2
# {0}"
            );
        }
    }

    #[test]
    fn test_compile_repeat_at_least() {
        // {m,}
        {
            let image = compile_from_str(r#"'a'{3,}"#).unwrap();
            let s = image.get_image_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 4, Counter inc <0>
- 2
  -> 3, Counter reset <0>
- 3
  -> 0, Jump
- 4
  -> 3, Repetition anchor <0>, threshold 3
  -> 5, Counter check <0>, from 3, to MAX
- 5
  -> 7, Match end {0}
> 6
  -> 2, Match start {0}
< 7
# {0}"
            );
        }

        // lazy
        {
            let image = compile_from_str(r#"'a'{3,}?"#).unwrap();
            let s = image.get_image_text();

            assert_str_eq!(
                s,
                "\
- 0
  -> 1, Char 'a'
- 1
  -> 4, Counter inc <0>
- 2
  -> 3, Counter reset <0>
- 3
  -> 0, Jump
- 4
  -> 5, Counter check <0>, from 3, to MAX
  -> 3, Jump
- 5
  -> 7, Match end {0}
> 6
  -> 2, Match start {0}
< 7
# {0}"
            );
        }

        // {1,} == one_or_more
        {
            assert_str_eq!(
                compile_from_str(r#"'a'{1,}"#).unwrap().get_image_text(),
                compile_from_str(r#"'a'+"#).unwrap().get_image_text()
            );
        }

        // {1,}? == lazy one_or_more
        {
            assert_str_eq!(
                compile_from_str(r#"'a'{1,}?"#).unwrap().get_image_text(),
                compile_from_str(r#"'a'+?"#).unwrap().get_image_text()
            );
        }

        // {0,} == zero_or_more
        {
            assert_str_eq!(
                compile_from_str(r#"'a'{0,}"#).unwrap().get_image_text(),
                compile_from_str(r#"'a'*"#).unwrap().get_image_text()
            );
        }

        // {0,}? == lazy zero_or_more
        {
            assert_str_eq!(
                compile_from_str(r#"'a'{0,}?"#).unwrap().get_image_text(),
                compile_from_str(r#"'a'*?"#).unwrap().get_image_text()
            );
        }
    }
}
