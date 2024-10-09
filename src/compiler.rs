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
            Expression::Alternation(_, _) => todo!(),
        };

        Ok(result)
    }

    fn emit_group(&mut self, expressions: &[Expression]) -> Result<EmitResult, Error> {
        // the _group_ of ANREG is different from the _group_ of
        // oridinary regular expressions.
        // ANREG's group is just a series of patterns, which will not
        // be captured unless enclosed by the function 'name' or 'index'

        // connecting two groups of states
        //
        //     current                   next
        //  /-----------\            /-----------\
        // --o in  out o-- ==jump== --o in  out o--
        //  \-----------/      ^     \-----------/
        //                     |
        //                     \-- jump transition

        let mut results = vec![];
        for expression in expressions {
            results.push(self.emit_expression(expression)?);
        }

        if results.len() == 1 {
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

    fn emit_literal(&mut self, literal: &Literal) -> Result<EmitResult, Error> {
        let result = match literal {
            Literal::Char(character) => self.emit_literal_char(*character, false)?,
            Literal::String(_) => todo!(),
            Literal::Symbol(_) => todo!(),
            Literal::CharSet(_) => todo!(),
            Literal::PresetCharSet(_) => todo!(),
        };

        Ok(result)
    }

    fn emit_literal_char(&mut self, character: char, inverse: bool) -> Result<EmitResult, Error> {
        let in_state_index = self.state_set.new_state();
        let out_state_index = self.state_set.new_state();
        let transition = Transition::Char(CharTransition::new(character, inverse));
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
            let s = state_set.get_states_and_transitions_text();

            assert_str_eq!(
                s,
                "\
> idx:0, head:Some(0), tail:Some(0)
  * link idx:0, prev:None, next:None
    trans idx:0, target state:1, [Char a]
< idx:1, head:None, tail:None"
            );
        }

        {
            let state_set = compile_from_str(r#"'a', 'b', 'c'"#).unwrap();
            let s = state_set.get_states_and_transitions_text();

            // println!("{}", s);
            assert_str_eq!(
                s,
                "\
> idx:0, head:Some(0), tail:Some(0)
  * link idx:0, prev:None, next:None
    trans idx:0, target state:1, [Char a]
- idx:1, head:Some(3), tail:Some(3)
  * link idx:3, prev:None, next:None
    trans idx:3, target state:2, [Epsilon]
- idx:2, head:Some(1), tail:Some(1)
  * link idx:1, prev:None, next:None
    trans idx:1, target state:3, [Char b]
- idx:3, head:Some(4), tail:Some(4)
  * link idx:4, prev:None, next:None
    trans idx:4, target state:4, [Epsilon]
- idx:4, head:Some(2), tail:Some(2)
  * link idx:2, prev:None, next:None
    trans idx:2, target state:5, [Char c]
< idx:5, head:None, tail:None"
            );
        }
    }
}
