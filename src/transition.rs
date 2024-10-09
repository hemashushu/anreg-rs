// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::fmt::Display;

trait ITransition {
    fn validated(&self, source: &[char], position: usize, length: usize) -> bool;

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
            Transition::Jump(_) => f.write_str("Epsilon"),
            Transition::Char(CharTransition { character, inverse }) => {
                write!(
                    f,
                    "Char {}",
                    if *inverse {
                        format!("^{}", character)
                    } else {
                        character.to_string()
                    }
                )
            }
        }
    }
}

// Epsilon
pub struct JumpTransition;

pub struct CharTransition {
    pub character: char,
    pub inverse: bool,
}

impl CharTransition {
    pub fn new(character: char, inverse: bool) -> Self {
        CharTransition { character, inverse }
    }
}

impl ITransition for JumpTransition {
    fn validated(&self, _source: &[char], _position: usize, _length: usize) -> bool {
        true
    }

    fn forward(&self) -> usize {
        0
    }
}

impl ITransition for CharTransition {
    fn validated(&self, source: &[char], position: usize, _length: usize) -> bool {
        (self.character == source[position]) ^ self.inverse
    }

    fn forward(&self) -> usize {
        1
    }
}
