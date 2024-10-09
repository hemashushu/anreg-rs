// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::state::StateSet;

struct Context {
    pub source: Box<Source>,
    pub cursors: Vec<Cursor>, // the `Cursor` stack.
    pub state: Box<StateSet>,
}

struct Source {
    pub text: Vec<char>,   // the source text
    pub length: usize,     // the length of source text
    pub fixed_start: bool, // it is true when the expression starts with `^`
    pub fixed_end: bool,   // it is true when the expression ends with `$`
}

// The `Cursor` can only be moved to left as a whole,
// and cannot exceed the `position` of the previous `Cursor` (if it exists).
// If the previous `Cursor` does not exist, it cannot be moved.
pub struct Cursor {
    pub start: usize, // the start poisition
    pub end: usize,   // the end position, it is the length of source text.

    // the position of the currently matched character.
    // unlike the `start` position of `Cursor`, this value can only
    // be increased (moved to right).
    pub position: usize,
    // // the position of the currently consumed character.
    // // which is normally the same as `match_position`, but
    // // is different only when an asserted match is encountered.
    // pub text_position: usize,
}
