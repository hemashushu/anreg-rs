// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::{
    ast::{Expression, Literal, Program},
    error::Error,
    parser::parse_from_str,
    state::StateSet,
    transition::{CharTransition, JumpTransition, Transition},
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
            Literal::Char(character) => self.emit_literal_char(*character, false)?,
            Literal::String(_) => todo!(),
            Literal::Status(_) => todo!(),
            Literal::CharSet(_) => todo!(),
            Literal::PresetCharSet(_) => todo!(),
        };

        Ok(result)
    }

    fn emit_literal_char(&mut self, character: char, inverse: bool) -> Result<EmitResult, Error> {
        let in_state_index = self.state_set.new_state();
        let out_state_index = self.state_set.new_state();
        let transition = Transition::Char(CharTransition::new(character /*, inverse */));
        self.state_set
            .append_transition(in_state_index, out_state_index, transition);
        Ok(EmitResult::new(in_state_index, out_state_index))
    }
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
}
