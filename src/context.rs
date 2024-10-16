// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::vec;

pub struct Context<'a> {
    pub chars: &'a [char], // the source text
    number_of_captures: usize,
    number_of_counters: usize,

    pub threads: Vec<Thread>,
    pub capture_positions: Vec<CapturePosition>, // the results of matches
    pub counters: Vec<usize>,                    // repetitions counters
}

impl<'a> Context<'a> {
    pub fn new(chars: &'a [char], number_of_captures: usize, number_of_counters: usize) -> Self {
        Context {
            chars,
            number_of_captures,
            number_of_counters,
            threads: vec![],
            capture_positions: vec![],
            counters: vec![],
        }
    }

    pub fn reset(&mut self, start: usize) {
        self.threads = vec![Thread::new(start, self.chars.len(), start)];
        self.capture_positions = vec![CapturePosition::default(); self.number_of_captures];
        self.counters = vec![0; self.number_of_counters];
    }
}

pub struct Thread {
    pub start: usize,              // poisition included
    pub end: usize,                // position excluded
    pub stateset_index: usize,     //
    pub cursor_stack: Vec<Cursor>, // the `Cursor` stack.
}

impl Thread {
    pub fn new(start: usize, end: usize, stateset_index: usize) -> Thread {
        Thread {
            start,
            end,
            stateset_index,
            cursor_stack: vec![Cursor::new(start, end)],
        }
    }
}

// The `Cursor` can only be moved to left as a whole,
// and cannot exceed the `position` of the previous `Cursor` (if it exists).
// If the previous `Cursor` does not exist, it cannot be moved.
pub struct Cursor {
    pub start: usize, // position included
    pub end: usize,   // position excluded.

    // store the positions of greedy repetition for
    // backtracking.
    pub anchors: Vec<usize>,

    // the position of the currently matched character.
    // unlike the `start` position of `Cursor`, this value can only
    // be increased (moved to right).
    pub position: usize,
}

impl Cursor {
    pub fn new(start: usize, end: usize) -> Self {
        Cursor {
            start,
            end,
            anchors: vec![],
            position: start,
        }
    }
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

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub struct CapturePosition {
    pub start: usize,        // position included
    pub end_included: usize, // position included
}

impl<'a> Context<'a> {
    #[inline]
    pub fn get_current_thread(&self) -> &Thread {
        self.threads.last().unwrap()
    }

    #[inline]
    pub fn get_current_cursor(&self) -> &Cursor {
        self.get_current_thread().cursor_stack.last().unwrap()
    }

    #[inline]
    pub fn get_current_position(&self) -> usize {
        self.get_current_cursor().position
    }

    #[inline]
    pub fn get_current_char(&self) -> char {
        self.get_char(self.get_current_position())
    }

    #[inline]
    pub fn is_first_char(&self) -> bool {
        self.get_current_position() == self.get_current_thread().start
    }

    #[inline]
    pub fn is_last_char(&self) -> bool {
        self.get_current_position() == self.get_current_thread().end - 1
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
        self.chars[position]
    }

    #[inline]
    fn get_previous_char(&self) -> char {
        if self.is_first_char() {
            '\0'
        } else {
            self.get_char(self.get_current_position() - 1)
        }
    }

    #[inline]
    fn get_next_char(&self) -> char {
        if self.is_last_char() {
            '\0'
        } else {
            self.get_char(self.get_current_position() + 1)
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
