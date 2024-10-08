// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

pub struct Context {
    pub text: Vec<char>,      // the source text
    pub length: usize,        // the length of source text
    pub fixed_start: bool,    // it is true when the expression starts with `^`
    pub fixed_end: bool,      // it is true when the expression ends with `$`
    pub cursors: Vec<Cursor>, // the `Cursor` stack.
    pub position: usize,      // it is sync to the position of the last cursor
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

// the Cursor stack demo:
//
// 1. normal mode
//
// start                                 end
// 0                                     len
// |====*================================| <-- cursor 0
//      ^__ position (move to right only)
//
// 2. backtracking
//
// start                                 end
// 0                                     len
// |-------------------------------------|
// |======*==============================| <-- cursor 0
// | pos__^     |===*====================| <-- cursor 1
// |            ^   ^__ pos' move to right only
// |            |__ start' move to left only
// |                                     |
// |-------------------------------------|
//
// 3. return to normal mode again, pop up upper frame
//
// start                                 end
// 0                                     len
// |=========*===========================| <-- cursor 0
//           ^__ position (move to right only)
//
//
// 4. backtracking within backtracking
//
// start                                 end
// 0                                     len
// |-------------------------------------|
// |======*==============================| <-- cursor 0
// | pos__^     |====*===================| <-- cursor 1
// |    start'__^    ^__ pos' ^          |
// |                          |===*======| <-- cursor 2
// |                 start''__^   ^__ pos''
// |                                     |
// |-------------------------------------|
//
// 5. return to normal mode again, pop up all upper frames
//
// start                                 end
// 0                                     len
// |=============*=======================| <-- cursor 0
//               ^__ position (move to right only)

impl Context {
    #[inline]
    pub fn get_current_char(&self) -> char {
        self.get_char(self.position)
    }

    #[inline]
    pub fn is_first_char(&self) -> bool {
        self.position == 0
    }

    #[inline]
    pub fn is_last_char(&self) -> bool {
        self.position == (self.length - 1)
    }

    pub fn is_word_bound(&self) -> bool {
        let current_char = self.get_current_char();

        if is_word_char(current_char) {
            !is_word_char(self.get_previous_char()) || !is_word_char(self.get_next_char())
        } else {
            is_word_char(self.get_previous_char()) || is_word_char(self.get_next_char())
        }
    }

    #[inline]
    pub fn get_char(&self, position: usize) -> char {
        self.text[position]
    }

    #[inline]
    fn get_previous_char(&self) -> char {
        if self.is_first_char() {
            '\0'
        } else {
            self.get_char(self.position - 1)
        }
    }

    #[inline]
    fn get_next_char(&self) -> char {
        if self.is_last_char() {
            '\0'
        } else {
            self.get_char(self.position + 1)
        }
    }
}

fn is_word_char(c: char) -> bool {
    ('a'..='z').any(|e| e == c)
        || ('A'..='Z').any(|e| e == c)
        || ('0'..='9').any(|e| e == c)
        || c == '_'
}
