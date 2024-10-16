// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::fmt::Display;

use crate::{ast::AssertionName, context::Context};

pub enum Transition {
    Jump(JumpTransition),
    Char(CharTransition),
    SpecialChar(SpecialCharTransition),
    String(StringTransition),
    CharSet(CharSetTransition),
    BackReference(BackReferenceTransition),
    Assertion(AssertionTransition),
    CaptureStart(CaptureStartTransition),
    CaptureEnd(CaptureEndTransition),
    CounterReset(CounterResetTransition),
    CounterInc(CounterIncTransition),
    CounterCheck(CounterCheckTransition),
    RepetionAnchor(RepetitionAnchorTransition),
    LookAheadAssertion(LookAheadAssertionTransition),
    LookBehindAssertion(LookBehindAssertionTransition),
}

impl Display for Transition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Transition::Jump(j) => write!(f, "{}", j),
            Transition::Char(c) => write!(f, "{}", c),
            Transition::String(s) => write!(f, "{}", s),
            Transition::CharSet(c) => write!(f, "{}", c),
            Transition::SpecialChar(s) => write!(f, "{}", s),
            Transition::BackReference(b) => write!(f, "{}", b),
            Transition::Assertion(a) => write!(f, "{}", a),
            Transition::CaptureStart(m) => write!(f, "{}", m),
            Transition::CaptureEnd(m) => write!(f, "{}", m),
            Transition::CounterReset(c) => write!(f, "{}", c),
            Transition::CounterInc(c) => write!(f, "{}", c),
            Transition::CounterCheck(c) => write!(f, "{}", c),
            Transition::RepetionAnchor(r) => write!(f, "{}", r),
            Transition::LookAheadAssertion(l) => write!(f, "{}", l),
            Transition::LookBehindAssertion(l) => write!(f, "{}", l),
        }
    }
}

trait TransitionTrait {
    fn validated(&self, context: &Context) -> ValidateResult;

    // Not all transitions have a fixed length, e.g.
    // the length of "a{3,5}" varies, but the
    // "a{3}" is 3.
    //
    // Returns `None` for a non-fixed length.
    fn length(&self) -> Option<usize>;
}

pub struct ValidateResult {
    success: bool,
    forward: usize,
}

impl ValidateResult {
    fn new(success: bool, forward: usize) -> Self {
        ValidateResult { success, forward }
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

pub struct SpecialCharTransition;

impl TransitionTrait for SpecialCharTransition {
    fn validated(&self, context: &Context) -> ValidateResult {
        // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Regular_expressions/Character_classes
        // \n, \r, \u2028 or \u2029

        let c = context.get_current_char();
        let success = c != '\n' && c != '\r';
        ValidateResult::new(success, 1)
    }

    fn length(&self) -> Option<usize> {
        Some(1)
    }
}

impl Display for SpecialCharTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Any char")
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
         * or
         * `let s = String::from_iter(&self.chars)`
         */
        let s = String::from_iter(&self.chars);
        write!(f, "String \"{}\"", s)
    }
}

pub struct CharSetTransition {
    items: Vec<CharSetItem>,
    negative: bool,
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
    pub fn new(items: Vec<CharSetItem>, negative: bool) -> Self {
        CharSetTransition { items, negative }
    }

    pub fn new_preset_word() -> Self {
        let mut items: Vec<CharSetItem> = vec![];
        add_preset_word(&mut items);
        CharSetTransition::new(items, false)
    }

    pub fn new_preset_not_word() -> Self {
        let mut items: Vec<CharSetItem> = vec![];
        add_preset_word(&mut items);
        CharSetTransition::new(items, true)
    }

    pub fn new_preset_space() -> Self {
        let mut items: Vec<CharSetItem> = vec![];
        add_preset_space(&mut items);
        CharSetTransition::new(items, false)
    }

    pub fn new_preset_not_space() -> Self {
        let mut items: Vec<CharSetItem> = vec![];
        add_preset_space(&mut items);
        CharSetTransition::new(items, true)
    }

    pub fn new_preset_digit() -> Self {
        let mut items: Vec<CharSetItem> = vec![];
        add_preset_digit(&mut items);
        CharSetTransition::new(items, false)
    }

    pub fn new_preset_not_digit() -> Self {
        let mut items: Vec<CharSetItem> = vec![];
        add_preset_digit(&mut items);
        CharSetTransition::new(items, true)
    }
}

pub fn add_char(items: &mut Vec<CharSetItem>, c: char) {
    items.push(CharSetItem::Char(c));
}

pub fn add_range(items: &mut Vec<CharSetItem>, start: char, end_included: char) {
    items.push(CharSetItem::Range(Range::new(start, end_included)));
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

        ValidateResult::new(found ^ self.negative, 1)
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
                CharSetItem::Char(c) => match c {
                    '\t' => "'\\t'".to_owned(),
                    '\r' => "'\\r'".to_owned(),
                    '\n' => "'\\n'".to_owned(),
                    _ => format!("'{}'", c),
                },
                CharSetItem::Range(r) => format!("'{}'..'{}'", r.start, r.end_included),
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

pub struct AssertionTransition {
    name: AssertionName,
}

impl AssertionTransition {
    pub fn new(name: AssertionName) -> Self {
        AssertionTransition { name }
    }
}

impl TransitionTrait for AssertionTransition {
    fn validated(&self, context: &Context) -> ValidateResult {
        todo!()
    }

    fn length(&self) -> Option<usize> {
        Some(0)
    }
}

impl Display for AssertionTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Assertion \"{}\"", self.name)
    }
}

pub struct BackReferenceTransition {
    capture_index: usize,
}

impl BackReferenceTransition {
    pub fn new(capture_index: usize) -> Self {
        BackReferenceTransition { capture_index }
    }
}

impl TransitionTrait for BackReferenceTransition {
    fn validated(&self, context: &Context) -> ValidateResult {
        todo!()
    }

    fn length(&self) -> Option<usize> {
        None
    }
}

impl Display for BackReferenceTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Back reference {{{}}}", self.capture_index)
    }
}

pub struct CaptureStartTransition {
    capture_index: usize,
}

pub struct CaptureEndTransition {
    capture_index: usize,
}

impl CaptureStartTransition {
    pub fn new(capture_index: usize) -> Self {
        CaptureStartTransition { capture_index }
    }
}

impl CaptureEndTransition {
    pub fn new(capture_index: usize) -> Self {
        CaptureEndTransition { capture_index }
    }
}

impl TransitionTrait for CaptureStartTransition {
    fn validated(&self, context: &Context) -> ValidateResult {
        todo!()
    }

    fn length(&self) -> Option<usize> {
        todo!()
    }
}

impl TransitionTrait for CaptureEndTransition {
    fn validated(&self, context: &Context) -> ValidateResult {
        todo!()
    }

    fn length(&self) -> Option<usize> {
        todo!()
    }
}

impl Display for CaptureStartTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Capture start {{{}}}", self.capture_index)
    }
}

impl Display for CaptureEndTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Capture end {{{}}}", self.capture_index)
    }
}

pub struct CounterResetTransition {
    counter_index: usize,
}

pub struct CounterIncTransition {
    counter_index: usize,
}

pub struct CounterCheckTransition {
    counter_index: usize,
    repetition_type: RepetitionType,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RepetitionType {
    Specified(usize),
    Range(usize, usize),
}

impl CounterResetTransition {
    pub fn new(counter_index: usize) -> Self {
        CounterResetTransition { counter_index }
    }
}

impl CounterIncTransition {
    pub fn new(counter_index: usize) -> Self {
        CounterIncTransition { counter_index }
    }
}

impl CounterCheckTransition {
    pub fn new(counter_index: usize, repetition_type: RepetitionType) -> Self {
        CounterCheckTransition {
            counter_index,
            repetition_type,
        }
    }
}

impl TransitionTrait for CounterResetTransition {
    fn validated(&self, context: &Context) -> ValidateResult {
        todo!()
    }

    fn length(&self) -> Option<usize> {
        Some(0)
    }
}

impl TransitionTrait for CounterIncTransition {
    fn validated(&self, context: &Context) -> ValidateResult {
        todo!()
    }

    fn length(&self) -> Option<usize> {
        Some(0)
    }
}

impl TransitionTrait for CounterCheckTransition {
    fn validated(&self, context: &Context) -> ValidateResult {
        todo!()
    }

    fn length(&self) -> Option<usize> {
        Some(0)
    }
}

impl Display for CounterResetTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Counter reset %{}", self.counter_index)
    }
}

impl Display for CounterIncTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Counter inc %{}", self.counter_index)
    }
}

impl Display for CounterCheckTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Counter check %{}, {}",
            self.counter_index, self.repetition_type
        )
    }
}

impl Display for RepetitionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepetitionType::Specified(n) => write!(f, "times {}", n),
            RepetitionType::Range(m, n) => {
                if n == &usize::MAX {
                    write!(f, "from {}, to MAX", m)
                } else {
                    write!(f, "from {}, to {}", m, n)
                }
            }
        }
    }
}

pub struct RepetitionAnchorTransition {
    counter_index: usize,

    // to indicate when to start recording,
    // the value of 'threshold' is the times (included) of repetition.
    threshold: usize,
}

impl RepetitionAnchorTransition {
    pub fn new(counter_index: usize, threshold: usize) -> Self {
        RepetitionAnchorTransition {
            counter_index,
            threshold,
        }
    }
}

impl TransitionTrait for RepetitionAnchorTransition {
    fn validated(&self, context: &Context) -> ValidateResult {
        todo!()
    }

    fn length(&self) -> Option<usize> {
        Some(0)
    }
}

impl Display for RepetitionAnchorTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Repetition anchor %{}, threshold {}",
            self.counter_index, self.threshold
        )
    }
}

pub struct LookAheadAssertionTransition {
    stateset_index: usize,
    negative: bool,
}

pub struct LookBehindAssertionTransition {
    stateset_index: usize,
    negative: bool,
    pattern_chars_length: usize,
}

impl LookAheadAssertionTransition {
    pub fn new(stateset_index: usize, negative: bool) -> Self {
        LookAheadAssertionTransition {
            stateset_index,
            negative,
        }
    }
}

impl LookBehindAssertionTransition {
    pub fn new(stateset_index: usize, negative: bool, pattern_chars_length: usize) -> Self {
        LookBehindAssertionTransition {
            stateset_index,
            negative,
            pattern_chars_length,
        }
    }
}

impl TransitionTrait for LookAheadAssertionTransition {
    fn validated(&self, context: &Context) -> ValidateResult {
        todo!()
    }

    fn length(&self) -> Option<usize> {
        None
    }
}

impl TransitionTrait for LookBehindAssertionTransition {
    fn validated(&self, context: &Context) -> ValidateResult {
        todo!()
    }

    fn length(&self) -> Option<usize> {
        Some(self.pattern_chars_length)
    }
}

impl Display for LookAheadAssertionTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.negative {
            write!(f, "Look ahead negative ${}", self.stateset_index)
        } else {
            write!(f, "Look ahead ${}", self.stateset_index)
        }
    }
}

impl Display for LookBehindAssertionTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.negative {
            write!(
                f,
                "Look behind negative ${}, pattern length {}",
                self.stateset_index, self.pattern_chars_length
            )
        } else {
            write!(
                f,
                "Look behind ${}, pattern length {}",
                self.stateset_index, self.pattern_chars_length
            )
        }
    }
}
