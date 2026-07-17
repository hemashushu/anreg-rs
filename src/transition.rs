// Copyright (c) 2026 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

use std::fmt::Display;

/// A `Transition` represents a state transition in a regular expression engine.
/// Each transition contains logic to match a specific pattern (for example, a character or string).
/// When executed, it processes the input character from a given position
/// and returns the result: either a failure or a success with additional information
/// (for example, how many characters to move forward).
///
/// A transition is similar to the condition in an `if` statement in programming languages.
#[derive(Debug)]
pub enum Transition {
    // Basic transitions
    Jump(JumpTransition),                                   // unconditional jump
    Char(CharTransition),                                   // match a single specified character
    AnyChar(AnyCharTransition),                             // match any single character
    String(StringTransition),                               // match a specific string
    CharSet(CharSetTransition),                             // match a set of characters or ranges
    BackReference(BackReferenceTransition), // match a backreference to a capture group
    LineBoundaryAssertion(LineBoundaryAssertionTransition), // match line start or end
    WordBoundaryAssertion(WordBoundaryAssertionTransition), // match word boundary or non-word boundary

    // Capture group transitions
    CaptureStart(CaptureStartTransition),
    CaptureEnd(CaptureEndTransition),

    // Counter transitions
    // Since the repetition may be nested, we need to use a counter and
    // a pair of save/load transitions to track the number of repetitions.
    CounterReset(CounterResetTransition), // Reset the counter to zero
    CounterIncrement(CounterIncrementTransition), // Increment the counter by one

    // Repetition transitions
    RepetitionForward(RepetitionForwardTransition),
    RepetitionBack(RepetitionBackTransition),

    // Look around assertion transitions
    LookAheadAssertion(LookAheadAssertionTransition),
    LookBehindAssertion(LookBehindAssertionTransition),
}

/// Represents a transition that performs an unconditional jump.
#[derive(Debug)]
pub struct JumpTransition;

/// Represents a transition that matches a single character.
#[derive(Debug)]
pub struct CharTransition {
    pub codepoint: u32,     // Unicode codepoint of the character
    pub byte_length: usize, // Length of the character in bytes
}

// Represents a transition for the special character - any character.
#[derive(Debug)]
pub struct AnyCharTransition;

/// Represents a transition that matches a specific string.
#[derive(Debug)]
pub struct StringTransition {
    pub codepoints: Vec<u32>, // Unicode codepoints of the string
    pub byte_length: usize,   // Total byte length of the string
}

/// Represents a transition that matches a set of characters or ranges.
#[derive(Debug)]
pub struct CharSetTransition {
    pub items: Vec<CharSetItem>, // List of characters or ranges
    pub negative: bool,          // Whether the set is negated
}

/// Represents an item in a character set, either a single character or a range.
#[derive(Debug)]
pub enum CharSetItem {
    Char(u32),        // A single character
    Range(CharRange), // A range of characters
}

/// Represents a range of characters (inclusive).
#[derive(Debug)]
pub struct CharRange {
    pub start: u32,         // Start of the range
    pub end_inclusive: u32, // End of the range (inclusive)
}

/// Represents a transition that matches a backreference to a capture group.
#[derive(Debug)]
pub struct BackReferenceTransition {
    pub capture_group_index: usize, // Index of the capture group
}

/// Represents a transition that asserts an anchor (for example, start or end of input).
#[derive(Debug)]
pub struct LineBoundaryAssertionTransition {
    // Indicates the type of anchor assertion
    // - false: start of line/input
    // - true: end of line/input
    pub is_end: bool,
}

/// Represents a transition that asserts a boundary (for example, word boundary).
#[derive(Debug)]
pub struct WordBoundaryAssertionTransition {
    // Indicates the type of boundary assertion
    // - false: word boundary
    // - true: non-word boundary
    pub is_negative: bool,
}

/// Represents the start of a capture group.
#[derive(Debug)]
pub struct CaptureStartTransition {
    pub capture_group_index: usize, // Index of the capture group
}

/// Represents the end of a capture group.
#[derive(Debug)]
pub struct CaptureEndTransition {
    pub capture_group_index: usize, // Index of the capture group
}

/// Represents a transition that resets a counter.
#[derive(Debug)]
pub struct CounterResetTransition {
    pub counter_index: usize, // Index of the counter to reset
}

/// Represents a transition that increments the counter.
#[derive(Debug)]
pub struct CounterIncrementTransition {
    pub counter_index: usize, // Index of the counter to increment
}

/// Represents a transition that checks the counter and moves forward if the condition is satisfied.
///
/// Jump forward if the counter value is satisfies the range of allowed repetitions.
#[derive(Debug)]
pub struct RepetitionForwardTransition {
    pub repetition_type: RepetitionType, // Type of repetition to check
}

/// Represents a transition that checks the counter and jumps back if the condition is satisfied.
///
/// Jump back if the counter value is less than the allowed number of repetitions.
#[derive(Debug)]
pub struct RepetitionBackTransition {
    pub repetition_type: RepetitionType, // Type of repetition
}

/// Represents a lookahead assertion transition.
#[derive(Debug)]
pub struct LookAheadAssertionTransition {
    pub route_index: usize, // Index of the route to evaluate
    pub is_negative: bool,  // Whether the assertion is negative
}

/// Represents a lookbehind assertion transition.
#[derive(Debug)]
pub struct LookBehindAssertionTransition {
    pub route_index: usize,          // Index of the route to evaluate
    pub is_negative: bool,           // Whether the assertion is negative
    pub match_length_in_char: usize, // Length of the match in characters
}

impl CharTransition {
    pub fn new(c: char) -> Self {
        let byte_length = c.len_utf8();
        CharTransition {
            codepoint: (c as u32),
            byte_length,
        }
    }
}

impl StringTransition {
    pub fn new(s: &str) -> Self {
        let chars: Vec<u32> = s.chars().map(|item| item as u32).collect();
        let byte_length = s.len();
        StringTransition {
            codepoints: chars,
            byte_length,
        }
    }
}

impl CharSetItem {
    pub fn new_char(character: char) -> Self {
        CharSetItem::Char(character as u32)
    }

    pub fn new_range(start: char, end_inclusive: char) -> Self {
        let char_range = CharRange {
            start: start as u32,
            end_inclusive: end_inclusive as u32,
        };
        CharSetItem::Range(char_range)
    }
}

impl CharSetTransition {
    pub fn new(items: Vec<CharSetItem>, negative: bool) -> Self {
        CharSetTransition { items, negative }
    }

    pub fn new_preset_charset_word() -> Self {
        let mut items: Vec<CharSetItem> = vec![];
        add_preset_word(&mut items);
        CharSetTransition::new(items, false)
    }

    pub fn new_preset_charset_not_word() -> Self {
        let mut items: Vec<CharSetItem> = vec![];
        add_preset_word(&mut items);
        CharSetTransition::new(items, true)
    }

    pub fn new_preset_charset_space() -> Self {
        let mut items: Vec<CharSetItem> = vec![];
        add_preset_space(&mut items);
        CharSetTransition::new(items, false)
    }

    pub fn new_preset_charset_not_space() -> Self {
        let mut items: Vec<CharSetItem> = vec![];
        add_preset_space(&mut items);
        CharSetTransition::new(items, true)
    }

    pub fn new_preset_charset_digit() -> Self {
        let mut items: Vec<CharSetItem> = vec![];
        add_preset_digit(&mut items);
        CharSetTransition::new(items, false)
    }

    pub fn new_preset_charset_not_digit() -> Self {
        let mut items: Vec<CharSetItem> = vec![];
        add_preset_digit(&mut items);
        CharSetTransition::new(items, true)
    }

    // pub fn new_preset_hex() -> Self {
    //     let mut items: Vec<CharSetItem> = vec![];
    //     add_preset_hex(&mut items);
    //     CharSetTransition::new(items, false)
    // }
}

pub fn add_char(items: &mut Vec<CharSetItem>, c: char) {
    items.push(CharSetItem::new_char(c));
}

pub fn add_range(items: &mut Vec<CharSetItem>, start: char, end_inclusive: char) {
    items.push(CharSetItem::new_range(start, end_inclusive));
}

pub fn add_preset_space(items: &mut Vec<CharSetItem>) {
    // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Regular_expressions/Character_classes
    // [\f\n\r\t\v\u0020\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000\ufeff]
    add_char(items, ' ');
    add_char(items, '\t');
    add_char(items, '\r');
    add_char(items, '\n');
}

pub fn add_preset_word(items: &mut Vec<CharSetItem>) {
    // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Regular_expressions/Character_classes
    // [A-Za-z0-9_]
    add_range(items, 'A', 'Z');
    add_range(items, 'a', 'z');
    add_range(items, '0', '9');
    add_char(items, '_');
}

pub fn add_preset_digit(items: &mut Vec<CharSetItem>) {
    // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Regular_expressions/Character_classes
    // [0-9]
    add_range(items, '0', '9');
}

// pub fn add_preset_hex(items: &mut Vec<CharSetItem>) {
//     // [a-fA-F0-9]
//     add_range(items, 'A', 'F');
//     add_range(items, 'a', 'f');
//     add_range(items, '0', '9');
// }

impl BackReferenceTransition {
    pub fn new(capture_group_index: usize) -> Self {
        BackReferenceTransition {
            capture_group_index,
        }
    }
}

impl LineBoundaryAssertionTransition {
    pub fn new(is_end: bool) -> Self {
        LineBoundaryAssertionTransition { is_end }
    }
}

impl WordBoundaryAssertionTransition {
    pub fn new(is_negative: bool) -> Self {
        WordBoundaryAssertionTransition { is_negative }
    }
}

impl CaptureStartTransition {
    pub fn new(capture_group_index: usize) -> Self {
        CaptureStartTransition {
            capture_group_index,
        }
    }
}

impl CaptureEndTransition {
    pub fn new(capture_group_index: usize) -> Self {
        CaptureEndTransition {
            capture_group_index,
        }
    }
}

impl CounterResetTransition {
    pub fn new(counter_index: usize) -> Self {
        CounterResetTransition { counter_index }
    }
}

impl CounterIncrementTransition {
    pub fn new(counter_index: usize) -> Self {
        CounterIncrementTransition { counter_index }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RepetitionType {
    Repeat(usize),
    RepeatFrom(usize),
    RepeatRange(usize, usize),
}

impl RepetitionForwardTransition {
    pub fn new(repetition_type: RepetitionType) -> Self {
        RepetitionForwardTransition { repetition_type }
    }
}

impl RepetitionBackTransition {
    pub fn new(repetition_type: RepetitionType) -> Self {
        RepetitionBackTransition { repetition_type }
    }
}

impl LookAheadAssertionTransition {
    pub fn new(route_index: usize, is_negative: bool) -> Self {
        LookAheadAssertionTransition {
            route_index,
            is_negative,
        }
    }
}

impl LookBehindAssertionTransition {
    pub fn new(route_index: usize, is_negative: bool, match_length_in_char: usize) -> Self {
        LookBehindAssertionTransition {
            route_index,
            is_negative,
            match_length_in_char,
        }
    }
}

impl Display for Transition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Transition::Jump(t) => write!(f, "{}", t),
            Transition::Char(t) => write!(f, "{}", t),
            Transition::String(t) => write!(f, "{}", t),
            Transition::CharSet(t) => write!(f, "{}", t),
            Transition::AnyChar(t) => write!(f, "{}", t),
            Transition::BackReference(t) => write!(f, "{}", t),
            Transition::LineBoundaryAssertion(t) => write!(f, "{}", t),
            Transition::WordBoundaryAssertion(t) => write!(f, "{}", t),
            Transition::CaptureStart(t) => write!(f, "{}", t),
            Transition::CaptureEnd(t) => write!(f, "{}", t),
            Transition::CounterReset(t) => write!(f, "{}", t),
            Transition::CounterIncrement(t) => write!(f, "{}", t),
            Transition::RepetitionForward(t) => write!(f, "{}", t),
            Transition::RepetitionBack(t) => write!(f, "{}", t),
            Transition::LookAheadAssertion(t) => write!(f, "{}", t),
            Transition::LookBehindAssertion(t) => write!(f, "{}", t),
        }
    }
}

impl Display for JumpTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Jump")
    }
}

fn escape_char(c: char) -> String {
    match c {
        '\n' => "\\n".to_string(),
        '\r' => "\\r".to_string(),
        '\t' => "\\t".to_string(),
        '\\' => "\\\\".to_string(),
        '\'' => "\\\'".to_string(),
        '\"' => "\\\"".to_string(),
        _ => c.to_string(),
    }
}

impl Display for CharTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c = unsafe { char::from_u32_unchecked(self.codepoint) };
        write!(f, "Char '{}'", escape_char(c))
    }
}

impl Display for AnyCharTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Any char")
    }
}

impl Display for StringTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self
            .codepoints
            .iter()
            .map(|item| unsafe { char::from_u32_unchecked(*item) })
            .map(escape_char)
            .collect::<Vec<String>>()
            .join("");
        write!(f, "String \"{}\"", s)
    }
}

impl Display for CharSetTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut lines = vec![];
        for item in &self.items {
            let line = match item {
                CharSetItem::Char(codepoint) => {
                    let c = unsafe { char::from_u32_unchecked(*codepoint) };
                    format!("'{}'", escape_char(c))
                }
                CharSetItem::Range(r) => {
                    let start = unsafe { char::from_u32_unchecked(r.start) };
                    let end_inclusive = unsafe { char::from_u32_unchecked(r.end_inclusive) };
                    format!("'{}'..'{}'", escape_char(start), escape_char(end_inclusive))
                }
            };
            lines.push(line);
        }

        let content = lines.join(", ");
        if self.negative {
            write!(f, "Charset ![{}]", content)
        } else {
            write!(f, "Charset [{}]", content)
        }
    }
}

impl Display for BackReferenceTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Back reference {{{}}}", self.capture_group_index)
    }
}

impl Display for LineBoundaryAssertionTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_end {
            write!(f, "Line boundary assertion is_end()")
        } else {
            write!(f, "Line boundary assertion is_start()")
        }
    }
}

impl Display for WordBoundaryAssertionTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_negative {
            write!(f, "Word boundary assertion is_not_bound()")
        } else {
            write!(f, "Word boundary assertion is_bound()")
        }
    }
}

impl Display for CaptureStartTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Capture start {{{}}}", self.capture_group_index)
    }
}

impl Display for CaptureEndTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Capture end {{{}}}", self.capture_group_index)
    }
}

impl Display for CounterResetTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Counter reset")
    }
}

impl Display for CounterIncrementTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Counter increment")
    }
}

impl Display for RepetitionForwardTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Repetition forward {}", self.repetition_type)
    }
}

impl Display for RepetitionBackTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Repetition back {}", self.repetition_type)
    }
}

impl Display for RepetitionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepetitionType::Repeat(n) => write!(f, "[{}]", n),
            RepetitionType::RepeatFrom(m) => write!(f, "[{}..]", m),
            RepetitionType::RepeatRange(m, n) => write!(f, "[{}..{}]", m, n),
        }
    }
}

impl Display for LookAheadAssertionTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_negative {
            write!(f, "Look ahead negative ${}", self.route_index)
        } else {
            write!(f, "Look ahead ${}", self.route_index)
        }
    }
}

impl Display for LookBehindAssertionTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_negative {
            write!(
                f,
                "Look behind negative ${}, match length {}",
                self.route_index, self.match_length_in_char
            )
        } else {
            write!(
                f,
                "Look behind ${}, match length {}",
                self.route_index, self.match_length_in_char
            )
        }
    }
}
