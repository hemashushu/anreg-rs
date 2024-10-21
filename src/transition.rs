// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::fmt::Display;

use crate::{
    ast::AssertionName,
    instance::{Instance, MatchRange},
    utf8reader::{self, read_char, read_previous_char},
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
    RepetitionWithAnchor(RepetitionWithAnchorTransition),
    Backtrack(BacktrackingTransition),

    // assertion
    LookAheadAssertion(LookAheadAssertionTransition),
    LookBehindAssertion(LookBehindAssertionTransition),
}

// Jump/Epsilon
pub struct JumpTransition;

pub struct CharTransition {
    // pub character: char,
    pub bytes: [u8; 4],
    pub byte_length: usize,
}

// There is only `char_any` currently
pub struct SpecialCharTransition;

pub struct StringTransition {
    // pub chars: Vec<char>,
    // pub length: usize,
    pub bytes: Vec<u8>,
    pub byte_length: usize,
}

pub struct CharSetTransition {
    pub items: Vec<CharSetItem>,
    pub negative: bool,
}

pub enum CharSetItem {
    Char(u32),
    Range(CharRange),
}

pub struct CharRange {
    pub start: u32,        // char,
    pub end_included: u32, // char,
}

pub struct BackReferenceTransition {
    pub capture_group_index: usize,
}

pub struct AssertionTransition {
    pub name: AssertionName,
}

pub struct CaptureStartTransition {
    pub capture_group_index: usize,
}

pub struct CaptureEndTransition {
    pub capture_group_index: usize,
}

pub struct CounterResetTransition {
    pub counter_index: usize,
}

pub struct CounterIncTransition {
    pub counter_index: usize,
}

pub struct CounterCheckTransition {
    pub counter_index: usize,
    pub repetition_type: RepetitionType,
}

pub struct RepetitionTransition {
    pub counter_index: usize,
    pub repetition_type: RepetitionType,
}

pub struct RepetitionWithAnchorTransition {
    pub counter_index: usize,
    pub repetition_type: RepetitionType,
}

pub struct BacktrackingTransition {
    pub counter_index: usize,
    pub anchor_node_index: usize,
}

pub struct LookAheadAssertionTransition {
    pub line_index: usize,
    pub negative: bool,
}

pub struct LookBehindAssertionTransition {
    pub line_index: usize,
    pub negative: bool,
    pub pattern_chars_length: usize,
}

impl CharTransition {
    pub fn new(character: char) -> Self {
        let mut bytes = [0u8; 4];
        character.encode_utf8(&mut bytes);
        let byte_length = character.len_utf8();
        CharTransition { bytes, byte_length }
    }
}

impl StringTransition {
    pub fn new(s: &str) -> Self {
        let bytes: Vec<u8> = s.as_bytes().to_vec();
        let byte_length = bytes.len();
        StringTransition { bytes, byte_length }
    }
}

impl CharSetItem {
    pub fn new_char(character: char) -> Self {
        // let mut bytes = [0u8; 4];
        // character.encode_utf8(&mut bytes);
        // let byte_length = character.len_utf8();
        CharSetItem::Char(character as u32)
    }

    pub fn new_range(start: char, end_included: char) -> Self {
        let char_range = CharRange {
            start: start as u32,
            end_included: end_included as u32,
        };
        CharSetItem::Range(char_range)
    }
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
    items.push(CharSetItem::new_char(c));
}

pub fn add_range(items: &mut Vec<CharSetItem>, start: char, end_included: char) {
    items.push(CharSetItem::new_range(start, end_included));
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
    pub fn new(capture_group_index: usize) -> Self {
        BackReferenceTransition {
            capture_group_index,
        }
    }
}

impl AssertionTransition {
    pub fn new(name: AssertionName) -> Self {
        AssertionTransition { name }
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

impl RepetitionWithAnchorTransition {
    pub fn new(counter_index: usize, repetition_type: RepetitionType) -> Self {
        RepetitionWithAnchorTransition {
            counter_index,
            repetition_type,
        }
    }
}

impl BacktrackingTransition {
    pub fn new(counter_index: usize, anchor_node_index: usize) -> Self {
        BacktrackingTransition {
            counter_index,
            anchor_node_index,
        }
    }
}

impl LookAheadAssertionTransition {
    pub fn new(line_index: usize, negative: bool) -> Self {
        LookAheadAssertionTransition {
            line_index,
            negative,
        }
    }
}

impl LookBehindAssertionTransition {
    pub fn new(line_index: usize, negative: bool, pattern_chars_length: usize) -> Self {
        LookBehindAssertionTransition {
            line_index,
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
            Transition::RepetitionWithAnchor(r) => write!(f, "{}", r),
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
        let (codepoint, _) = utf8reader::read_char(&self.bytes, 0);
        let c = unsafe { char::from_u32_unchecked(codepoint) };
        write!(f, "Char '{}'", c)
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
        let data = self.bytes.clone();
        write!(f, "String \"{}\"", String::from_utf8(data).unwrap())
    }
}

impl Display for CharSetTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut lines = vec![];
        for item in &self.items {
            let line = match item {
                CharSetItem::Char(codepoint) => {
                    // let (codepoint, _) = utf8reader::read_char(bytes, 0);
                    let c = unsafe { char::from_u32_unchecked(*codepoint) };
                    match c {
                        '\t' => "'\\t'".to_owned(),
                        '\r' => "'\\r'".to_owned(),
                        '\n' => "'\\n'".to_owned(),
                        _ => format!("'{}'", c),
                    }
                }
                CharSetItem::Range(r) => {
                    let start = unsafe { char::from_u32_unchecked(r.start) };
                    let end_included = unsafe { char::from_u32_unchecked(r.end_included) };
                    format!("'{}'..'{}'", start, end_included)
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

impl Display for AssertionTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Assertion \"{}\"", self.name)
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

impl Display for RepetitionWithAnchorTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Repetition with anchor %{}, {}",
            self.counter_index, self.repetition_type
        )
    }
}

impl Display for BacktrackingTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Backtrack %{} -> {}",
            self.counter_index, self.anchor_node_index
        )
    }
}

impl Display for LookAheadAssertionTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.negative {
            write!(f, "Look ahead negative ${}", self.line_index)
        } else {
            write!(f, "Look ahead ${}", self.line_index)
        }
    }
}

impl Display for LookBehindAssertionTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.negative {
            write!(
                f,
                "Look behind negative ${}, pattern length {}",
                self.line_index, self.pattern_chars_length
            )
        } else {
            write!(
                f,
                "Look behind ${}, pattern length {}",
                self.line_index, self.pattern_chars_length
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
                    let mut is_same = true;
                    for (idx, b) in transition
                        .bytes
                        .iter()
                        .take(transition.byte_length)
                        .enumerate()
                    {
                        if b != &instance.bytes[position + idx] {
                            is_same = false;
                            break;
                        }
                    }

                    if is_same {
                        CheckResult::Success(transition.byte_length)
                    } else {
                        CheckResult::Failure
                    }
                }
            }
            Transition::SpecialChar(_transition) => {
                // 'special char' currently contains only the 'char_any'.
                //
                // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Regular_expressions/Character_classes
                // \n, \r, \u2028 or \u2029

                let thread = instance.get_current_thread_ref();

                if position >= thread.end_position {
                    CheckResult::Failure
                } else {
                    let (current_char, byte_length) = get_char(instance, position);
                    if current_char != '\n' as u32 && current_char != '\r' as u32 {
                        CheckResult::Success(byte_length)
                    } else {
                        CheckResult::Failure
                    }
                }
            }
            Transition::String(transition) => {
                let thread = instance.get_current_thread_ref();

                if position + transition.byte_length > thread.end_position {
                    CheckResult::Failure
                } else {
                    let mut is_same = true;
                    for (idx, b) in transition.bytes.iter().enumerate() {
                        if b != &instance.bytes[position + idx] {
                            is_same = false;
                            break;
                        }
                    }

                    if is_same {
                        CheckResult::Success(transition.byte_length)
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

                let (current_char, byte_length) = get_char(instance, position);
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
                    CheckResult::Success(byte_length)
                } else {
                    CheckResult::Failure
                }
            }
            Transition::BackReference(transition) => {
                let MatchRange { start, end } =
                    &instance.match_ranges[transition.capture_group_index];

                let bytes = &instance.bytes[*start..*end];
                let byte_length = end - start;

                let thread = instance.get_current_thread_ref();

                if position + byte_length >= thread.end_position {
                    CheckResult::Failure
                } else {
                    let mut is_same = true;

                    for (idx, c) in bytes.iter().enumerate() {
                        if c != &instance.bytes[idx + position] {
                            is_same = false;
                            break;
                        }
                    }

                    if is_same {
                        CheckResult::Success(byte_length)
                    } else {
                        CheckResult::Failure
                    }
                }
            }
            Transition::Assertion(transition) => {
                let success = match transition.name {
                    AssertionName::Start => is_first_char(instance, position),
                    AssertionName::End => is_end(instance, position),
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
                instance.match_ranges[transition.capture_group_index].start = position;
                CheckResult::Success(0)
            }
            Transition::CaptureEnd(transition) => {
                instance.match_ranges[transition.capture_group_index].end = position;
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
                let can_forward = match transition.repetition_type {
                    RepetitionType::Specified(m) => count == m,
                    RepetitionType::Range(from, to) => count >= from && count <= to,
                };
                if can_forward {
                    CheckResult::Success(0)
                } else {
                    CheckResult::Failure
                }
            }
            Transition::Repetition(transition) => {
                let count = instance.counters[transition.counter_index];
                let can_backward = match transition.repetition_type {
                    RepetitionType::Specified(times) => count < times,
                    RepetitionType::Range(_, to) => count < to,
                };
                if can_backward {
                    CheckResult::Success(0)
                } else {
                    CheckResult::Failure
                }
            }
            Transition::RepetitionWithAnchor(transition) => {
                let count = instance.counters[transition.counter_index];
                let (should_anchor, can_backward) = match transition.repetition_type {
                    RepetitionType::Specified(times) => (false, count < times),
                    RepetitionType::Range(from, to) => (count > from, count < to),
                };

                if can_backward {
                    if should_anchor {
                        instance.anchors[transition.counter_index].push(position);
                    }
                    CheckResult::Success(0)
                } else {
                    CheckResult::Failure
                }
            }
            Transition::Backtrack(transition) => {
                // move the position back by anchor
                // note:
                // actually the last one of anchors is redundant,
                // because the position has already been tried and failed.
                let previous_position_opt = instance.anchors[transition.counter_index].pop();

                if let Some(previous_position) = previous_position_opt {
                    // build a stackframe for the target state node
                    instance.append_tasks_by_node(transition.anchor_node_index, previous_position);
                }

                // always return `failure`
                CheckResult::Failure
            }
            Transition::LookAheadAssertion(_transition) => todo!(),
            Transition::LookBehindAssertion(_transition) => todo!(),
        }
    }
}

#[inline]
fn get_char(instance: &Instance, position: usize) -> (u32, usize) {
    read_char(instance.bytes, position)
}

#[inline]
fn is_first_char(_instance: &Instance, position: usize) -> bool {
    position == 0
}

#[inline]
fn is_end(instance: &Instance, position: usize) -> bool {
    let total_byte_length = instance.bytes.len();
    position >= total_byte_length
}

// #[inline]
// fn is_last_char(instance: &Instance, position: usize) -> bool {
//     let total_byte_length = instance.bytes.len();
//     let (_, last_char_byte_length) = read_previous_char(instance.bytes, total_byte_length);
//     position >= total_byte_length - last_char_byte_length
// }

// fn get_previous_char(instance: &Instance, position: usize) -> u32 {
//     if is_first_char(instance, position) {
//         0
//     } else {
//         let (codepoint, _) = get_char(instance, position - 1);
//         codepoint
//     }
// }

// fn get_next_char(instance: &Instance, position: usize) -> u32 {
//     if is_last_char(instance, position) {
//         0
//     } else {
//         let (_, length) = get_char(instance, position);
//         let (codepoint, _) = get_char(instance, position + length);
//         codepoint
//     }
// }

fn is_word_bound(instance: &Instance, position: usize) -> bool {
    if instance.bytes.len() == 0 {
        false
    } else if position == 0 {
        let (current_char, _) = get_char(instance, position);
        is_word_char(current_char)
    } else if position >= instance.bytes.len() {
        let (previous_char, _) = get_char(instance, position - 1);
        is_word_char(previous_char)
    } else {
        let (current_char, _) = get_char(instance, position);
        let (previous_char, _) = get_char(instance, position - 1);

        if is_word_char(current_char) {
            !is_word_char(previous_char)
        } else {
            is_word_char(previous_char)
        }
    }
}

fn is_word_char(c: u32) -> bool {
    (c >= 'a' as u32 && c <= 'z' as u32)
        || (c >= 'A' as u32 && c <= 'Z' as u32)
        || (c >= '0' as u32 && c <= '9' as u32)
        || (c == '_' as u32)
}

pub enum CheckResult {
    Success(/* position forward */ usize),
    Failure,
}
