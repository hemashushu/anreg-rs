// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::fmt::Display;

use crate::{
    ast::AssertionName,
    instance::{CapturePosition, Instance},
};

trait TransitionTrait {
    fn check(&self, context: &mut Instance) -> CheckResult;
}

pub enum CheckResult {
    Success(/* position forward */ usize),
    Failure,
}

pub enum Transition {
    Jump(JumpTransition),
    Char(CharTransition),
    SpecialChar(SpecialCharTransition),
    String(StringTransition),
    CharSet(CharSetTransition),
    BackReference(BackReferenceTransition),
    Assertion(AssertionTransition),

    // capture
    CaptureStart(CaptureStartTransition),
    CaptureEnd(CaptureEndTransition),

    // reset the associated counter and the list of anchors
    CounterReset(CounterResetTransition),
    CounterInc(CounterIncTransition),
    CounterCheck(CounterCheckTransition),

    Repetition(RepetitionTransition),
    RepetitionAnchor(RepetitionAnchorTransition),
    Backtrack(BacktrackingTransition),

    // assertion
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
            Transition::Repetition(r) => write!(f, "{}", r),
            Transition::RepetitionAnchor(r) => write!(f, "{}", r),
            Transition::Backtrack(b) => write!(f, "{}", b),
            Transition::LookAheadAssertion(l) => write!(f, "{}", l),
            Transition::LookBehindAssertion(l) => write!(f, "{}", l),
        }
    }
}

// Jump/Epsilon
pub struct JumpTransition;

impl TransitionTrait for JumpTransition {
    fn check(&self, _context: &mut Instance) -> CheckResult {
        // always success
        CheckResult::Success(0)
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
    fn check(&self, context: &mut Instance) -> CheckResult {
        let thread = context.get_current_thread();
        let position = thread.get_position();

        if position >= thread.end {
            CheckResult::Failure
        } else {
            let current_char = context.get_char(position);
            if self.character == current_char {
                CheckResult::Success(1)
            } else {
                CheckResult::Failure
            }
        }
    }
}

impl Display for CharTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Char '{}'", self.character)
    }
}

pub struct SpecialCharTransition;

impl TransitionTrait for SpecialCharTransition {
    fn check(&self, context: &mut Instance) -> CheckResult {
        // 'special char' currently contains only the 'char_any'.
        //
        // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Regular_expressions/Character_classes
        // \n, \r, \u2028 or \u2029

        let thread = context.get_current_thread();
        let position = thread.get_position();

        if position >= thread.end {
            CheckResult::Failure
        } else {
            let current_char = context.get_char(position);
            if current_char != '\n' && current_char != '\r' {
                CheckResult::Success(1)
            } else {
                CheckResult::Failure
            }
        }
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
    fn check(&self, context: &mut Instance) -> CheckResult {
        let thread = context.get_current_thread();
        let position = thread.get_position();

        if position + self.length >= thread.end {
            CheckResult::Failure
        } else {
            let mut is_same = true;
            for idx in 0..self.length {
                if self.chars[idx] != context.get_char(idx + position) {
                    is_same = false;
                    break;
                }
            }

            if is_same {
                CheckResult::Success(self.length)
            } else {
                CheckResult::Failure
            }
        }
    }
}

impl Display for StringTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        /*
         * convert Vec<char> into String:
         * `let s:String = chars.iter().collect()`
         * or
         * `let s = String::from_iter(&chars)`
         */
        let s = String::from_iter(&self.chars);
        write!(f, "String \"{}\"", s)
    }
}

pub struct CharSetTransition {
    items: Vec<CharSetItem>,
    negative: bool,
}

pub struct CharRange {
    start: char,
    end_included: char,
}

impl CharRange {
    pub fn new(start: char, end_included: char) -> Self {
        CharRange {
            start,
            end_included,
        }
    }
}

pub enum CharSetItem {
    Char(char),
    Range(CharRange),
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
    items.push(CharSetItem::Range(CharRange::new(start, end_included)));
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
    fn check(&self, context: &mut Instance) -> CheckResult {
        let thread = context.get_current_thread();
        let position = thread.get_position();

        if position >= thread.end {
            return CheckResult::Failure;
        }

        let current_char = context.get_char(position);
        let mut found: bool = false;

        for item in &self.items {
            found = match item {
                CharSetItem::Char(c) => current_char == *c,
                CharSetItem::Range(r) => current_char >= r.start && current_char <= r.end_included,
            };

            if found {
                break;
            }
        }

        if found ^ self.negative {
            CheckResult::Success(1)
        } else {
            CheckResult::Failure
        }
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
    fn check(&self, context: &mut Instance) -> CheckResult {
        let success = match self.name {
            AssertionName::Start => context.is_first_char(),
            AssertionName::End => context.is_last_char(),
            AssertionName::IsBound => context.is_word_bound(),
            AssertionName::IsNotBound => !context.is_word_bound(),
        };

        if success {
            CheckResult::Success(0)
        } else {
            CheckResult::Failure
        }
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
    fn check(&self, context: &mut Instance) -> CheckResult {
        let CapturePosition {
            start,
            end_included,
        } = &context.capture_positions[self.capture_index];

        let chars = &context.chars[*start..=*end_included];
        let length = end_included - start + 1;

        let thread = context.get_current_thread();
        let position = thread.get_position();

        if position + length >= thread.end {
            CheckResult::Failure
        } else {
            let mut is_same = true;
            for idx in 0..length {
                if chars[idx] != context.get_char(idx + position) {
                    is_same = false;
                    break;
                }
            }

            if is_same {
                CheckResult::Success(length)
            } else {
                CheckResult::Failure
            }
        }
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
    fn check(&self, context: &mut Instance) -> CheckResult {
        let position = context.get_current_position();
        context.capture_positions[self.capture_index].start = position;
        CheckResult::Success(0)
    }
}

impl TransitionTrait for CaptureEndTransition {
    fn check(&self, context: &mut Instance) -> CheckResult {
        let position = context.get_current_position();
        context.capture_positions[self.capture_index].end_included = position;
        CheckResult::Success(0)
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
    fn check(&self, context: &mut Instance) -> CheckResult {
        // reset counter
        context.counters[self.counter_index] = 0;

        // reset anchors also
        context.anchors[self.counter_index] = vec![];

        CheckResult::Success(0)
    }
}

impl TransitionTrait for CounterIncTransition {
    fn check(&self, context: &mut Instance) -> CheckResult {
        context.counters[self.counter_index] += 1;
        CheckResult::Success(0)
    }
}

impl TransitionTrait for CounterCheckTransition {
    fn check(&self, context: &mut Instance) -> CheckResult {
        /**
         *
         *
         *
         *
         *
         *
         */
        todo!()
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

pub struct RepetitionTransition {
    counter_index: usize,
    repetition_type: RepetitionType,
}

pub struct RepetitionAnchorTransition {
    counter_index: usize,

    // // to indicate when to start recording,
    // // the value of 'threshold' is the times (included) of repetition.
    // threshold: usize,
    repetition_type: RepetitionType,
}

impl RepetitionTransition {
    pub fn new(counter_index: usize, repetition_type: RepetitionType) -> Self {
        RepetitionTransition {
            counter_index,
            repetition_type,
        }
    }
}

impl RepetitionAnchorTransition {
    pub fn new(counter_index: usize, repetition_type: RepetitionType) -> Self {
        RepetitionAnchorTransition {
            counter_index,
            repetition_type,
        }
    }
}

impl TransitionTrait for RepetitionTransition {
    fn check(&self, context: &mut Instance) -> CheckResult {
        todo!()
    }
}

impl TransitionTrait for RepetitionAnchorTransition {
    fn check(&self, context: &mut Instance) -> CheckResult {
        todo!()
    }
}

impl Display for RepetitionTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Repetition %{}, {}",
            self.counter_index, self.repetition_type
        )
    }
}

impl Display for RepetitionAnchorTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Repetition anchor %{}, {}",
            self.counter_index, self.repetition_type
        )
    }
}

pub struct BacktrackingTransition {
    counter_index: usize,
    anchor_state_index: usize,
}

impl BacktrackingTransition {
    pub fn new(counter_index: usize, anchor_state_index: usize) -> Self {
        BacktrackingTransition {
            counter_index,
            anchor_state_index,
        }
    }
}

impl TransitionTrait for BacktrackingTransition {
    fn check(&self, context: &mut Instance) -> CheckResult {
        // move back the position by anchor,
        // and build a stackframe for the target state node
        // todo
        CheckResult::Failure
    }
}

impl Display for BacktrackingTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Backtrack %{} -> {}",
            self.counter_index, self.anchor_state_index
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
    fn check(&self, context: &mut Instance) -> CheckResult {
        todo!()
    }

    // fn length(&self) -> Option<usize> {
    //     None
    // }
}

impl TransitionTrait for LookBehindAssertionTransition {
    fn check(&self, context: &mut Instance) -> CheckResult {
        todo!()
    }

    // fn length(&self) -> Option<usize> {
    //     Some(self.pattern_chars_length)
    // }
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
