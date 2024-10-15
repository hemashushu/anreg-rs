// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

pub struct Context {
    pub text: Vec<char>, // the source text
    pub length: usize,   // the length of source text

    pub cursors: Vec<Cursor>, // the `Cursor` stack.
    pub position: usize,      // it is sync to the position of the last cursor
    pub results: Vec<Result>, // the results of matches
    pub counters: Vec<usize>, // repetitions counters
}

// The `Cursor` can only be moved to left as a whole,
// and cannot exceed the `position` of the previous `Cursor` (if it exists).
// If the previous `Cursor` does not exist, it cannot be moved.
pub struct Cursor {
    pub start: usize, // the start poisition
    pub end: usize,   // the end position, it is the length of source text.

    // store the positions of greedy repetition for
    // backtracking.
    pub anchors: Vec<usize>,

    // the position of the currently matched character.
    // unlike the `start` position of `Cursor`, this value can only
    // be increased (moved to right).
    pub position: usize,
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
// |          v-- pos                    |
// |====#=#=#=*==========================| <-- cursor 0
// |    ^ ^ ^ ^                          |
// |    | | | |===*======================| <-- cursor 1
// | anchors/ ^   ^__ pos' move to right only
// |          |__ start' move to left by anchor
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
// |            v-- pos                  |
// |======#==#==*========================| <-- cursor 0
// | anchors ^  ^            v-- pos'    |
// |            |====#===#===*===========| <-- cursor 1
// |    start'__^            ^           |
// |                         |===*=======| <-- cursor 2
// |                start''__^   ^__ pos''
// |                                     |
// |-------------------------------------|
//
// 5. return to normal mode again, pop up all upper frames
//
// start                                 end
// 0                                     len
// |==================*=================| <-- cursor 0
//                    ^__ position (move to right only)

pub struct Result {
    start: usize,
    end_included: usize,
}

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

        if Context::is_word_char(current_char) {
            !Context::is_word_char(self.get_previous_char())
                || !Context::is_word_char(self.get_next_char())
        } else {
            Context::is_word_char(self.get_previous_char())
                || Context::is_word_char(self.get_next_char())
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

    #[inline]
    fn is_word_char(c: char) -> bool {
        ('a'..='z').any(|e| e == c)
            || ('A'..='Z').any(|e| e == c)
            || ('0'..='9').any(|e| e == c)
            || c == '_'
    }
}
