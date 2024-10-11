// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::fmt::Display;

use crate::context::Context;

pub struct ValidateResult {
    success: bool,
    forward: usize,
}

impl ValidateResult {
    fn new(success: bool, forward: usize) -> Self {
        ValidateResult { success, forward }
    }
}

trait TransitionTrait {
    fn validated(&self, context: &Context) -> ValidateResult;

    // Not all transitions have a fixed length, e.g.
    // the length of "a{3,5}" varies, but the
    // "a{3}" is 3.
    fn length(&self) -> Option<usize>;
}

pub enum Transition {
    Jump(JumpTransition),
    Char(CharTransition),
    String(StringTransition),
    CharSet(CharSetTransition),
}

impl Display for Transition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Transition::Jump(j) => write!(f, "{}", j),
            Transition::Char(c) => write!(f, "{}", c),
            Transition::String(s) => write!(f, "{}", s),
            Transition::CharSet(c) => write!(f, "{}", c),
        }
    }
}

// Jump/Epsilon
pub struct JumpTransition;

impl TransitionTrait for JumpTransition {
    fn validated(&self, _context: &Context) -> ValidateResult {
        ValidateResult::new(true, 0)
    }

    fn length(&self) -> Option<usize> {
        Some(0)
    }
}

impl Display for JumpTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Jump")
    }
}

pub struct CharTransition {
    pub character: char,
}

impl CharTransition {
    pub fn new(character: char) -> Self {
        CharTransition { character }
    }
}

impl TransitionTrait for CharTransition {
    fn validated(&self, context: &Context) -> ValidateResult {
        let success = self.character == context.get_current_char();
        ValidateResult::new(success, 1)
    }

    fn length(&self) -> Option<usize> {
        Some(1)
    }
}

impl Display for CharTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Char '{}'", self.character)
    }
}

pub struct StringTransition {
    chars: Vec<char>,
    length: usize,
}

impl StringTransition {
    pub fn new(s: &str) -> Self {
        let chars: Vec<char> = s.chars().collect();
        let length = chars.len();
        StringTransition { chars, length }
    }
}

impl TransitionTrait for StringTransition {
    fn validated(&self, context: &Context) -> ValidateResult {
        todo!()
    }

    fn length(&self) -> Option<usize> {
        Some(self.length)
    }
}

impl Display for StringTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        /*
         * convert Vec<char> into String:
         * `let s:String = chars.iter().collect()`
         */
        let s = String::from_iter(&self.chars);
        write!(f, "String \"{}\"", s)
    }
}

pub struct CharSetTransition {
    items: Vec<CharSetItem>,
    inverse: bool,
}

pub struct Range {
    start: char,
    end_included: char,
}

impl Range {
    pub fn new(start: char, end_included: char) -> Self {
        Range {
            start,
            end_included,
        }
    }
}

pub enum CharSetItem {
    Char(char),
    Range(Range),
}

impl CharSetTransition {
    pub fn new(items: Vec<CharSetItem>, inverse: bool) -> Self {
        CharSetTransition { items, inverse }
    }

    pub fn new_preset_word() -> Self {
        let mut charset = CharSetTransition::new(vec![], false);
        charset.add_preset_word();
        charset
    }

    pub fn new_preset_not_word() -> Self {
        let mut charset = CharSetTransition::new(vec![], true);
        charset.add_preset_word();
        charset
    }

    pub fn new_preset_space() -> Self {
        let mut charset = CharSetTransition::new(vec![], false);
        charset.add_preset_space();
        charset
    }

    pub fn new_preset_not_space() -> Self {
        let mut charset = CharSetTransition::new(vec![], true);
        charset.add_preset_space();
        charset
    }

    pub fn new_preset_digit() -> Self {
        let mut charset = CharSetTransition::new(vec![], false);
        charset.add_preset_digit();
        charset
    }

    pub fn new_preset_not_digit() -> Self {
        let mut charset = CharSetTransition::new(vec![], true);
        charset.add_preset_digit();
        charset
    }

    pub fn add_char(&mut self, c: char) {
        self.items.push(CharSetItem::Char(c));
    }

    pub fn add_range(&mut self, start: char, end_included: char) {
        self.items
            .push(CharSetItem::Range(Range::new(start, end_included)));
    }

    pub fn add_items(&mut self, mut custom_items: Vec<CharSetItem>) {
        self.items.append(&mut custom_items) // Vec::append = move
    }

    pub fn add_preset_space(&mut self) {
        // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Regular_expressions/Character_classes
        // [\f\n\r\t\v\u0020\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000\ufeff]
        self.add_char(' ');
        self.add_char('\t');
        self.add_char('\r');
        self.add_char('\n');
    }

    pub fn add_preset_word(&mut self) {
        // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Regular_expressions/Character_classes
        // [A-Za-z0-9_]
        self.add_range('A', 'Z');
        self.add_range('a', 'z');
        self.add_range('0', '9');
        self.add_char('_');
    }

    pub fn add_preset_digit(&mut self) {
        // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Regular_expressions/Character_classes
        // [0-9]
        self.add_range('0', '9');
    }
}

impl TransitionTrait for CharSetTransition {
    fn validated(&self, context: &Context) -> ValidateResult {
        let current = context.get_current_char();
        let mut found: bool = false;

        for item in &self.items {
            found = match item {
                CharSetItem::Char(c) => current == *c,
                CharSetItem::Range(r) => current >= r.start && current <= r.end_included,
            };

            if found {
                break;
            }
        }

        ValidateResult::new(found ^ self.inverse, 1)
    }

    fn length(&self) -> Option<usize> {
        Some(1)
    }
}

impl Display for CharSetTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut lines = vec![];
        for item in &self.items {
            let line = match item {
                CharSetItem::Char(c) => c.to_string(),
                CharSetItem::Range(r) => format!("'{}'..'{}'", r.start, r.end_included),
            };
            lines.push(line);
        }

        let content = lines.join(", ");
        if self.inverse {
            write!(f, "![{}]", content)
        } else {
            write!(f, "[{}]", content)
        }
    }
}
