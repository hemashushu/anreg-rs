// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::fmt::Display;

use crate::context::Context;

trait TransitionTrait {
    fn validated(&self, context: &Context) -> bool;

    // Move position
    fn forward(&self) -> usize;
}

pub enum Transition {
    Jump(JumpTransition),
    Char(CharTransition),
}

impl Display for Transition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Transition::Jump(_) => f.write_str("Jump"),
            Transition::Char(CharTransition {
                character, /*, inverse */
            }) => {
                write!(
                    f,
                    "Char '{}'",
                    character // if *inverse {
                              //     format!("^{}", character)
                              // } else {
                              //     character.to_string()
                              // }
                )
            }
        }
    }
}

// Jump/Epsilon
pub struct JumpTransition;

pub struct CharTransition {
    pub character: char,
    // pub inverse: bool,
}

impl CharTransition {
    pub fn new(character: char /*, inverse: bool */) -> Self {
        CharTransition {
            character, /*, inverse */
        }
    }
}

impl TransitionTrait for JumpTransition {
    fn validated(&self, _context: &Context) -> bool {
        true
    }

    fn forward(&self) -> usize {
        0
    }
}

impl TransitionTrait for CharTransition {
    fn validated(&self, context: &Context) -> bool {
        self.character == context.get_current_char() /* ^ self.inverse */
    }

    fn forward(&self) -> usize {
        1
    }
}
