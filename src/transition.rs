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

// Jump/Epsilon
pub struct JumpTransition;

pub struct CharTransition {
    pub character: char,
}

pub struct SpecialCharTransition;

pub struct StringTransition {
    chars: Vec<char>,
    length: usize,
}

pub struct CharSetTransition {
    items: Vec<CharSetItem>,
    negative: bool,
}

pub struct CharRange {
    start: char,
    end_included: char,
}

pub struct BackReferenceTransition {
    capture_index: usize,
}

pub struct AssertionTransition {
    name: AssertionName,
}

pub struct CaptureStartTransition {
    capture_index: usize,
}

pub struct CaptureEndTransition {
    capture_index: usize,
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

pub struct RepetitionTransition {
    counter_index: usize,
    repetition_type: RepetitionType,
}

pub struct RepetitionAnchorTransition {
    counter_index: usize,
    repetition_type: RepetitionType,
}

pub struct BacktrackingTransition {
    counter_index: usize,
    anchor_state_index: usize,
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

impl CharTransition {
    pub fn new(character: char) -> Self {
        CharTransition { character }
    }
}

impl StringTransition {
    pub fn new(s: &str) -> Self {
        let chars: Vec<char> = s.chars().collect();
        let length = chars.len();
        StringTransition { chars, length }
    }
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

impl BackReferenceTransition {
    pub fn new(capture_index: usize) -> Self {
        BackReferenceTransition { capture_index }
    }
}

impl AssertionTransition {
    pub fn new(name: AssertionName) -> Self {
        AssertionTransition { name }
    }
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

#[derive(Debug, PartialEq, Clone)]
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

impl BacktrackingTransition {
    pub fn new(counter_index: usize, anchor_state_index: usize) -> Self {
        BacktrackingTransition {
            counter_index,
            anchor_state_index,
        }
    }
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

impl Display for JumpTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Jump")
    }
}

impl Display for CharTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Char '{}'", self.character)
    }
}

impl Display for SpecialCharTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Any char")
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

impl Display for BackReferenceTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Back reference {{{}}}", self.capture_index)
    }
}

impl Display for AssertionTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Assertion \"{}\"", self.name)
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

impl Display for BacktrackingTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Backtrack %{} -> {}",
            self.counter_index, self.anchor_state_index
        )
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

impl Transition {
    pub fn check(&self, instance: &mut Instance, position: usize) -> CheckResult {
        match self {
            Transition::Jump(_) => {
                // always success
                CheckResult::Success(0)
            }
            Transition::Char(transition) => {
                let thread = instance.get_current_thread_ref();

                if position >= thread.end_position {
                    CheckResult::Failure
                } else {
                    let current_char = get_char(instance, position);
                    if transition.character == current_char {
                        CheckResult::Success(1)
                    } else {
                        CheckResult::Failure
                    }
                }
            }
            Transition::SpecialChar(transition) => {
                // 'special char' currently contains only the 'char_any'.
                //
                // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Regular_expressions/Character_classes
                // \n, \r, \u2028 or \u2029

                let thread = instance.get_current_thread_ref();

                if position >= thread.end_position {
                    CheckResult::Failure
                } else {
                    let current_char = get_char(instance, position);
                    if current_char != '\n' && current_char != '\r' {
                        CheckResult::Success(1)
                    } else {
                        CheckResult::Failure
                    }
                }
            }
            Transition::String(transition) => {
                let thread = instance.get_current_thread_ref();

                if position + transition.length >= thread.end_position {
                    CheckResult::Failure
                } else {
                    let mut is_same = true;
                    for idx in 0..transition.length {
                        if transition.chars[idx] != get_char(instance, idx + position) {
                            is_same = false;
                            break;
                        }
                    }

                    if is_same {
                        CheckResult::Success(transition.length)
                    } else {
                        CheckResult::Failure
                    }
                }
            }
            Transition::CharSet(transition) => {
                let thread = instance.get_current_thread_ref();

                if position >= thread.end_position {
                    return CheckResult::Failure;
                }

                let current_char = get_char(instance, position);
                let mut found: bool = false;

                for item in &transition.items {
                    found = match item {
                        CharSetItem::Char(c) => current_char == *c,
                        CharSetItem::Range(r) => {
                            current_char >= r.start && current_char <= r.end_included
                        }
                    };

                    if found {
                        break;
                    }
                }

                if found ^ transition.negative {
                    CheckResult::Success(1)
                } else {
                    CheckResult::Failure
                }
            }
            Transition::BackReference(transition) => {
                let CapturePosition {
                    start,
                    end_included,
                } = &instance.capture_positions[transition.capture_index];

                let chars = &instance.chars[*start..=*end_included];
                let length = end_included - start + 1;

                let thread = instance.get_current_thread_ref();

                if position + length >= thread.end_position {
                    CheckResult::Failure
                } else {
                    let mut is_same = true;
                    for idx in 0..length {
                        if chars[idx] != get_char(instance, idx + position) {
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
            Transition::Assertion(transition) => {
                let success = match transition.name {
                    AssertionName::Start => is_first_char(instance, position),
                    AssertionName::End => is_last_char(instance, position),
                    AssertionName::IsBound => is_word_bound(instance, position),
                    AssertionName::IsNotBound => !is_word_bound(instance, position),
                };

                if success {
                    CheckResult::Success(0)
                } else {
                    CheckResult::Failure
                }
            }
            Transition::CaptureStart(transition) => {
                instance.capture_positions[transition.capture_index].start = position;
                CheckResult::Success(0)
            }
            Transition::CaptureEnd(transition) => {
                instance.capture_positions[transition.capture_index].end_included = position;
                CheckResult::Success(0)
            }
            Transition::CounterReset(transition) => {
                // reset counter
                instance.counters[transition.counter_index] = 0;

                // reset anchors also
                instance.anchors[transition.counter_index] = vec![];

                CheckResult::Success(0)
            }
            Transition::CounterInc(transition) => {
                instance.counters[transition.counter_index] += 1;
                CheckResult::Success(0)
            }
            Transition::CounterCheck(transition) => {
                let count = instance.counters[transition.counter_index];
                let success = match transition.repetition_type {
                    RepetitionType::Specified(m) => count == m,
                    RepetitionType::Range(from, to) => count >= from && count <= to,
                };
                if success {
                    CheckResult::Success(0)
                } else {
                    CheckResult::Failure
                }
            }
            Transition::Repetition(transition) => {
                let count = instance.counters[transition.counter_index];
                let success = match transition.repetition_type {
                    RepetitionType::Specified(max) => count < max,
                    RepetitionType::Range(_, max) => count < max,
                };
                if success {
                    CheckResult::Success(0)
                } else {
                    CheckResult::Failure
                }
            }
            Transition::RepetitionAnchor(transition) => {
                let count = instance.counters[transition.counter_index];
                let success = match transition.repetition_type {
                    RepetitionType::Specified(max) => count < max,
                    RepetitionType::Range(_, max) => count < max,
                };

                if success {
                    // add anchor
                    instance.anchors[transition.counter_index].push(position);

                    // return
                    CheckResult::Success(0)
                } else {
                    CheckResult::Failure
                }
            }
            Transition::Backtrack(transition) => {
                // move back the position by anchor
                let previous_position_opt = instance.anchors[transition.counter_index].pop();

                if let Some(previous_position) = previous_position_opt {
                    // build a stackframe for the target state node
                    todo!()
                }

                CheckResult::Failure
            }
            Transition::LookAheadAssertion(transition) => todo!(),
            Transition::LookBehindAssertion(transition) => todo!(),
        }
    }
}

#[inline]
fn get_char(instance: &Instance, position: usize) -> char {
    instance.chars[position]
}

#[inline]
fn is_first_char(_instance: &Instance, position: usize) -> bool {
    position == 0
}

#[inline]
fn is_last_char(instance: &Instance, position: usize) -> bool {
    position == instance.chars.len() - 1
}

fn get_previous_char(instance: &Instance, position: usize) -> char {
    if is_first_char(instance, position) {
        '\0'
    } else {
        get_char(instance, position - 1)
    }
}

fn get_next_char(instance: &Instance, position: usize) -> char {
    if is_last_char(instance, position) {
        '\0'
    } else {
        get_char(instance, position + 1)
    }
}

fn is_word_bound(instance: &Instance, position: usize) -> bool {
    let current_char = get_char(instance, position);

    if is_word_char(current_char) {
        !is_word_char(get_previous_char(instance, position))
            || !is_word_char(get_next_char(instance, position))
    } else {
        is_word_char(get_previous_char(instance, position))
            || is_word_char(get_next_char(instance, position))
    }
}

#[inline]
fn is_word_char(c: char) -> bool {
    ('a'..='z').any(|e| e == c)
        || ('A'..='Z').any(|e| e == c)
        || ('0'..='9').any(|e| e == c)
        || c == '_'
}

pub enum CheckResult {
    Success(/* position forward */ usize),
    Failure,
}
