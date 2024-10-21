// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use core::str;

use crate::{
    compiler::compile_from_str,
    error::Error,
    instance::{Instance, MatchRange, Thread},
    route::{Route, MAIN_LINE_INDEX},
    transition::CheckResult,
    utf8reader::read_char,
};

pub struct Process {
    pub route: Route,
}

impl Process {
    pub fn new(pattern: &str) -> Result<Self, Error> {
        let route = compile_from_str(pattern)?;

        // DEBUG::
        println!("{}", route.get_debug_text());

        Ok(Process { route })
    }

    pub fn find<'a, 'b>(&'a self, text: &'b str) -> Option<Match<'a, 'b>> {
        let bytes = text.as_bytes();
        let mut instance = Instance::from_bytes(bytes);
        if !instance.exec(&self.route, 0) {
            return None;
        }

        let groups: Vec<MatchGroup> = instance
            .match_ranges
            .iter()
            .enumerate()
            .map(|(idx, match_range)| {
                MatchGroup::new(
                    match_range.start,
                    match_range.end,
                    self.route.get_capture_group_name_by_index(idx),
                    sub_string(bytes, match_range.start, match_range.end),
                )
            })
            .collect();

        Some(Match { groups })
    }

    pub fn find_iter<'a, 'b>(&'a self, text: &'b str) -> Matches<'a, 'b> {
        let bytes = text.as_bytes();
        let instance = Instance::from_bytes(bytes);
        let matches = Matches::new(&self.route, instance);
        matches
    }

    pub fn test(&self, text: &str) -> bool {
        let bytes = text.as_bytes();
        let mut instance = Instance::from_bytes(bytes);
        instance.exec(&self.route, 0)
    }
}

pub struct Matches<'a, 'b> {
    route: &'a Route,
    instance: Instance<'b>,
    last_position: usize,
}

impl<'a, 'b> Matches<'a, 'b> {
    fn new(route: &'a Route, instance: Instance<'b>) -> Self {
        Matches {
            route,
            instance,
            last_position: 0,
        }
    }
}

impl<'a, 'b> Iterator for Matches<'a, 'b> {
    type Item = Match<'a, 'b>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.instance.exec(self.route, self.last_position) {
            return None;
        }

        let groups: Vec<MatchGroup> = self
            .instance
            .match_ranges
            .iter()
            .enumerate()
            .map(|(idx, match_range)| {
                MatchGroup::new(
                    match_range.start,
                    match_range.end,
                    self.route.get_capture_group_name_by_index(idx),
                    sub_string(self.instance.bytes, match_range.start, match_range.end),
                )
            })
            .collect();

        self.last_position = groups[0].end;

        Some(Match { groups })
    }
}

impl<'a> Instance<'a> {
    pub fn exec(&mut self, route: &Route, start: usize) -> bool {
        start_main_thread(self, route, start, self.bytes.len())
    }

    //     pub fn exec_with_values(&mut self, start: usize) -> Option<Vec<MatchGroup>> {
    //         let capture_names = self.route.get_capture_group_names();
    //         let bytes = self.bytes;
    //
    //         if let Some(match_ranges) = self.exec(start) {
    //             let match_groups: Vec<MatchGroup> = match_ranges
    //                 .iter()
    //                 .zip(capture_names.iter())
    //                 .map(|(range, name_opt)| MatchGroup {
    //                     start: range.start,
    //                     end: range.end,
    //                     name: (*name_opt).map(|item| item.to_owned()),
    //                     value: sub_string(bytes, range.start, range.end).to_owned(),
    //                 })
    //                 .collect();
    //
    //             Some(match_groups)
    //         } else {
    //             None
    //         }
    //     }
    //
    //     pub fn get_group_index_by_name(&self, name: &str) -> Option<usize> {
    //         self.route.get_capture_group_index_by_name(name)
    //     }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Match<'a, 'b> {
    pub groups: Vec<MatchGroup<'a, 'b>>,
}

impl<'a, 'b> Match<'a, 'b> {
    pub fn get_group_by_name(&self, name: &str) -> Option<&MatchGroup> {
        self.groups.iter().find(|item| match item.name {
            Some(s) => s == name,
            None => false,
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct MatchGroup<'a, 'b> {
    pub start: usize, // position included
    pub end: usize,   // position excluded
    pub name: Option<&'a String>,
    pub value: &'b str, // String,
}

impl<'a, 'b> MatchGroup<'a, 'b> {
    pub fn new(start: usize, end: usize, name: Option<&'a String>, value: &'b str) -> Self {
        MatchGroup {
            start,
            end,
            name,
            value,
        }
    }
}

fn sub_string(bytes: &[u8], start: usize, end_excluded: usize) -> &str {
    /*
     * convert Vec<char> into String:
     * `let s:String = chars.iter().collect()`
     * or
     * `let s = String::from_iter(&chars)`
     */
    let slice = &bytes[start..end_excluded];
    str::from_utf8(slice).unwrap()
}

fn start_main_thread(instance: &mut Instance, route: &Route, mut start: usize, end: usize) -> bool {
    // allocate the vector of 'capture positions' and 'repetition counters'
    let number_of_capture_groups = route.capture_groups.len();
    // let number_of_counters = instance.route.get_number_of_counters();
    let main_thread = Thread::new(start, end, MAIN_LINE_INDEX);

    instance.threads = vec![main_thread];
    instance.match_ranges = vec![MatchRange::default(); number_of_capture_groups];
    // instance.counters = vec![0; number_of_counters];
    // instance.anchors = vec![vec![]; number_of_counters];

    while start < end {
        if start_thread(instance, route, start) {
            return true;
        }

        if route.lines[MAIN_LINE_INDEX].fixed_start {
            break;
        }

        // move one character forward and try again
        let (_, byte_length) = read_char(instance.bytes, start);
        start += byte_length;
    }

    false
}

fn start_thread(instance: &mut Instance, route: &Route, position: usize) -> bool {
    let (line_index, entry_node_index, exit_node_index) = {
        let thread = instance.get_current_thread_ref();
        let line_index = thread.line_index;
        let line = &route.lines[line_index];
        (line_index, line.start_node_index, line.end_node_index)
    };

    // DEBUG::
    println!(
        ">>THREAD START, line: {}, entry node: {}, position: {}",
        line_index, entry_node_index, position
    );

    // add transitions of the entry node
    instance.append_transition_stack_frames_by_node(route, entry_node_index, position, 0);

    // take the last task
    while let Some(frame) = instance.get_current_thread_ref_mut().transition_stack.pop() {
        // get the transition
        let line = &route.lines[line_index];
        let node = &line.nodes[frame.current_node_index];
        let transition_item = &node.transition_items[frame.transition_index];

        let position = frame.position;
        let last_repetition_count = frame.repetition_count;
        let transition = &transition_item.transition;
        let target_node_index = transition_item.target_node_index;

        // DEBUG::
        println!(
            "> node: {}, position: {}, rep count: {}",
            frame.current_node_index, position, last_repetition_count
        );

        let check_result = transition.check(instance, position, last_repetition_count);
        match check_result {
            CheckResult::Success(move_forward, current_repetition_count) => {
                // DEBUG::
                println!(
                    "  trans: {}, forward: {}, rep count: {} -> node: {}",
                    transition, move_forward, current_repetition_count, target_node_index
                );

                if target_node_index == exit_node_index {
                    println!(
                        "  THREAD FINISH, line: {}, node: {}",
                        line_index, exit_node_index
                    );
                    return true;
                }

                instance.append_transition_stack_frames_by_node(
                    route,
                    target_node_index,
                    position + move_forward,
                    current_repetition_count,
                );
            }
            CheckResult::Failure => {
                // DEBUG::
                println!("  trans: {}, failed", transition);
            }
        }
    }

    // DEBUG::
    println!("  THREAD FAILED, line: {}", line_index);
    false
}

#[cfg(test)]
mod tests {
    use super::{Match, MatchGroup, Process};
    use pretty_assertions::assert_eq;

    fn single_group(start: usize, end: usize, value: &str) -> Match {
        Match {
            groups: vec![MatchGroup::new(start, end, None, value)],
        }
    }

    #[test]
    fn test_process_char() {
        // exists in the middle and at the end of the text
        {
            let process = Process::new("'a'").unwrap();
            let mut matches = process.find_iter("babbaa");

            assert_eq!(matches.next(), Some(single_group(1, 2, "a")));
            assert_eq!(matches.next(), Some(single_group(4, 5, "a")));
            assert_eq!(matches.next(), Some(single_group(5, 6, "a")));
            assert_eq!(matches.next(), None);
        }

        // exists in the middle and at the beginning of the text
        {
            let process = Process::new("'a'").unwrap();
            let mut matches = process.find_iter("abaabb");

            assert_eq!(matches.next(), Some(single_group(0, 1, "a")));
            assert_eq!(matches.next(), Some(single_group(2, 3, "a")));
            assert_eq!(matches.next(), Some(single_group(3, 4, "a")));
            assert_eq!(matches.next(), None);
        }

        // non-existent
        {
            let process = Process::new("'a'").unwrap();
            let mut matches = process.find_iter("xyz");

            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_char_with_utf8() {
        // existent
        {
            let process = Process::new("'文'").unwrap();
            let mut matches = process.find_iter("abc中文字符文字🌏人文");

            assert_eq!(matches.next(), Some(single_group(6, 9, "文")));
            assert_eq!(matches.next(), Some(single_group(15, 18, "文")));
            assert_eq!(matches.next(), Some(single_group(28, 31, "文")));
            assert_eq!(matches.next(), None);
        }

        // non-existent
        {
            let process = Process::new("'文'").unwrap();
            let mut matches = process.find_iter("abc正则表达式🌏改");

            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_string() {
        // existent
        {
            let process = Process::new("\"abc\"").unwrap();
            let text = "ababcbcabc";
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(2, 5, "abc")));
            assert_eq!(matches.next(), Some(single_group(7, 10, "abc")));
            assert_eq!(matches.next(), None);
        }

        // non-existent
        {
            let process = Process::new("\"abc\"").unwrap();
            let text = "uvwxyz";
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_string_with_utf8() {
        {
            let process = Process::new("\"文字\"").unwrap();
            let text = "abc文字文本象形文字🎁表情文字";
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(3, 9, "文字")));
            assert_eq!(matches.next(), Some(single_group(21, 27, "文字")));
            assert_eq!(matches.next(), Some(single_group(37, 43, "文字")));
            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_preset_charset() {
        {
            let process = Process::new("char_word").unwrap();
            let text = "a*1**_ **";
            //               "^ ^  ^   "
            let mut matches = process.find_iter(text);
            assert_eq!(matches.next(), Some(single_group(0, 1, "a")));
            assert_eq!(matches.next(), Some(single_group(2, 3, "1")));
            assert_eq!(matches.next(), Some(single_group(5, 6, "_")));
            assert_eq!(matches.next(), None);
        }

        {
            let process = Process::new("char_not_word").unwrap();
            let text = "!a@12 bc_";
            //               "v v  v   "
            let mut matches = process.find_iter(text);
            assert_eq!(matches.next(), Some(single_group(0, 1, "!")));
            assert_eq!(matches.next(), Some(single_group(2, 3, "@")));
            assert_eq!(matches.next(), Some(single_group(5, 6, " ")));
            assert_eq!(matches.next(), None);
        }

        {
            let process = Process::new("char_digit").unwrap();
            let text = "1a2b_3de*";
            //               "^ ^  ^   "
            let mut matches = process.find_iter(text);
            assert_eq!(matches.next(), Some(single_group(0, 1, "1")));
            assert_eq!(matches.next(), Some(single_group(2, 3, "2")));
            assert_eq!(matches.next(), Some(single_group(5, 6, "3")));
            assert_eq!(matches.next(), None);
        }

        {
            let process = Process::new("char_not_digit").unwrap();
            let text = "a1_23 456";
            //               "v v  v   "
            let mut matches = process.find_iter(text);
            assert_eq!(matches.next(), Some(single_group(0, 1, "a")));
            assert_eq!(matches.next(), Some(single_group(2, 3, "_")));
            assert_eq!(matches.next(), Some(single_group(5, 6, " ")));
            assert_eq!(matches.next(), None);
        }

        {
            let process = Process::new("char_space").unwrap();
            let text = " 1\tab\n_*!";
            //               "^ ^-  ^-   "
            //                012 345 678
            let mut matches = process.find_iter(text);
            assert_eq!(matches.next(), Some(single_group(0, 1, " ")));
            assert_eq!(matches.next(), Some(single_group(2, 3, "\t")));
            assert_eq!(matches.next(), Some(single_group(5, 6, "\n")));
            assert_eq!(matches.next(), None);
        }

        {
            let process = Process::new("char_not_space").unwrap();
            let text = "a\t1\r\n*   ";
            //               "v  v    v   "
            //                01 23 4 5678
            let mut matches = process.find_iter(text);
            assert_eq!(matches.next(), Some(single_group(0, 1, "a")));
            assert_eq!(matches.next(), Some(single_group(2, 3, "1")));
            assert_eq!(matches.next(), Some(single_group(5, 6, "*")));
            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_charset() {
        // chars
        {
            let process = Process::new("['a','b','c']").unwrap();
            let text = "adbefcghi";
            //               "^ ^  ^   "
            let mut matches = process.find_iter(text);
            assert_eq!(matches.next(), Some(single_group(0, 1, "a")));
            assert_eq!(matches.next(), Some(single_group(2, 3, "b")));
            assert_eq!(matches.next(), Some(single_group(5, 6, "c")));
            assert_eq!(matches.next(), None);
        }

        // negative
        {
            let process = Process::new("!['a','b','c']").unwrap();
            let text = "xa1bb*ccc";
            //               "v v  v   "
            let mut matches = process.find_iter(text);
            assert_eq!(matches.next(), Some(single_group(0, 1, "x")));
            assert_eq!(matches.next(), Some(single_group(2, 3, "1")));
            assert_eq!(matches.next(), Some(single_group(5, 6, "*")));
            assert_eq!(matches.next(), None);
        }

        // range
        {
            let process = Process::new("['a'..'c']").unwrap();
            let text = "adbefcghi";
            //               "^ ^  ^   "
            let mut matches = process.find_iter(text);
            assert_eq!(matches.next(), Some(single_group(0, 1, "a")));
            assert_eq!(matches.next(), Some(single_group(2, 3, "b")));
            assert_eq!(matches.next(), Some(single_group(5, 6, "c")));
            assert_eq!(matches.next(), None);
        }

        // negative
        {
            let process = Process::new("!['a'..'c']").unwrap();
            let text = "xa1bb*ccc";
            //               "v v  v   "
            let mut matches = process.find_iter(text);
            assert_eq!(matches.next(), Some(single_group(0, 1, "x")));
            assert_eq!(matches.next(), Some(single_group(2, 3, "1")));
            assert_eq!(matches.next(), Some(single_group(5, 6, "*")));
            assert_eq!(matches.next(), None);
        }

        // ranges
        {
            let process = Process::new("['a'..'f', '0'..'9']").unwrap();
            let text = "am1npfq*_";
            //               "^ ^  ^   "
            let mut matches = process.find_iter(text);
            assert_eq!(matches.next(), Some(single_group(0, 1, "a")));
            assert_eq!(matches.next(), Some(single_group(2, 3, "1")));
            assert_eq!(matches.next(), Some(single_group(5, 6, "f")));
            assert_eq!(matches.next(), None);
        }

        // negative
        {
            let process = Process::new("!['a'..'f', '0'..'9']").unwrap();
            let text = "man12*def";
            //               "v v  v   "
            let mut matches = process.find_iter(text);
            assert_eq!(matches.next(), Some(single_group(0, 1, "m")));
            assert_eq!(matches.next(), Some(single_group(2, 3, "n")));
            assert_eq!(matches.next(), Some(single_group(5, 6, "*")));
            assert_eq!(matches.next(), None);
        }

        // combine range with preset
        {
            let process = Process::new("['a'..'f', char_digit]").unwrap();
            let text = "am1npfq*_";
            //               "^ ^  ^   "
            let mut matches = process.find_iter(text);
            assert_eq!(matches.next(), Some(single_group(0, 1, "a")));
            assert_eq!(matches.next(), Some(single_group(2, 3, "1")));
            assert_eq!(matches.next(), Some(single_group(5, 6, "f")));
            assert_eq!(matches.next(), None);
        }

        // negative
        {
            let process = Process::new("!['a'..'f', char_digit]").unwrap();
            let text = "man12*def";
            //               "v v  v   "
            let mut matches = process.find_iter(text);
            assert_eq!(matches.next(), Some(single_group(0, 1, "m")));
            assert_eq!(matches.next(), Some(single_group(2, 3, "n")));
            assert_eq!(matches.next(), Some(single_group(5, 6, "*")));
            assert_eq!(matches.next(), None);
        }

        // nested
        {
            let process = Process::new("[['a','b','c','d'..'f'], ['0'..'8'], '9']").unwrap();
            let text = "am1npfq*_";
            //               "^ ^  ^   "
            let mut matches = process.find_iter(text);
            assert_eq!(matches.next(), Some(single_group(0, 1, "a")));
            assert_eq!(matches.next(), Some(single_group(2, 3, "1")));
            assert_eq!(matches.next(), Some(single_group(5, 6, "f")));
            assert_eq!(matches.next(), None);
        }

        // negative
        {
            let process = Process::new("![['a','b','c','d'..'f'], ['0'..'8'], '9']").unwrap();
            let text = "man12*def";
            //               "v v  v   "
            let mut matches = process.find_iter(text);
            assert_eq!(matches.next(), Some(single_group(0, 1, "m")));
            assert_eq!(matches.next(), Some(single_group(2, 3, "n")));
            assert_eq!(matches.next(), Some(single_group(5, 6, "*")));
            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_charset_with_utf8() {
        {
            let process = Process::new("['文','字','🍅']").unwrap();
            let text = "abc正文写字🍉宋体字体🍅测试🍋";
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(6, 9, "文")));
            assert_eq!(matches.next(), Some(single_group(12, 15, "字")));
            assert_eq!(matches.next(), Some(single_group(25, 28, "字")));
            assert_eq!(matches.next(), Some(single_group(31, 35, "🍅")));
            assert_eq!(matches.next(), None);
        }

        // negative
        {
            let process = Process::new("!['文','字','🍅']").unwrap();
            let text = "哦字文🍅文噢字🍅文文字字喔";
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(0, 3, "哦")));
            assert_eq!(matches.next(), Some(single_group(16, 19, "噢")));
            assert_eq!(matches.next(), Some(single_group(38, 41, "喔")));
            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_special_char() {
        let process = Process::new("char_any").unwrap();
        let text = "\na\r\n1 \n";
        //               "  ^    ^^  "
        let mut matches = process.find_iter(text);

        assert_eq!(matches.next(), Some(single_group(1, 2, "a")));
        assert_eq!(matches.next(), Some(single_group(4, 5, "1")));
        assert_eq!(matches.next(), Some(single_group(5, 6, " ")));
        assert_eq!(matches.next(), None);
    }

    #[test]
    fn test_process_group() {
        // anreg group = a sequence of patterns
        {
            let process = Process::new("'a', 'b', 'c'").unwrap();
            let text = "ababcbcabc";
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(2, 5, "abc")));
            assert_eq!(matches.next(), Some(single_group(7, 10, "abc")));
            assert_eq!(matches.next(), None);
        }

        {
            let process = Process::new("'%', char_digit").unwrap();
            let text = "0123%567%9";
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(4, 6, "%5")));
            assert_eq!(matches.next(), Some(single_group(8, 10, "%9")));
            assert_eq!(matches.next(), None);
        }

        {
            let process = Process::new("['+','-'], ('%', char_digit)").unwrap();
            let text = "%12+%56-%9";
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(3, 6, "+%5")));
            assert_eq!(matches.next(), Some(single_group(7, 10, "-%9")));
            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_start_and_end_assertion() {
        {
            let process = Process::new("start, 'a'").unwrap();
            let text = "ab";
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(0, 1, "a")));
            assert_eq!(matches.next(), None);
        }

        {
            let process = Process::new("'a', end").unwrap();
            let text = "ab";
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), None);
        }

        {
            let process = Process::new("start, 'a'").unwrap();
            let text = "ba";
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), None);
        }

        {
            let process = Process::new("'a', end").unwrap();
            let text = "ba";
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(1, 2, "a")));
            assert_eq!(matches.next(), None);
        }

        // both 'start' and 'end'
        {
            let process = Process::new("start, 'a', end").unwrap();
            let text = "a";
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(0, 1, "a")));
            assert_eq!(matches.next(), None);
        }

        // both 'start' and 'end' - failed 1
        {
            let process = Process::new("start, 'a', end").unwrap();
            let text = "ab";
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), None);
        }

        // both 'start' and 'end' - failed 2
        {
            let process = Process::new("start, 'a', end").unwrap();
            let text = "ba";
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_boundary_assertion() {
        // matching 'boundary + char'
        {
            let process = Process::new("is_bound, 'a'").unwrap();
            let text = "ab";
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(0, 1, "a")));
            assert_eq!(matches.next(), None);
        }

        {
            let process = Process::new("is_bound, 'a'").unwrap();
            let text = "a";
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(0, 1, "a")));
            assert_eq!(matches.next(), None);
        }

        {
            let process = Process::new("is_bound, 'a'").unwrap();
            let text = " a";
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(1, 2, "a")));
            assert_eq!(matches.next(), None);
        }

        {
            let process = Process::new("is_bound, 'a'").unwrap();
            let text = "ba";
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), None);
        }

        // matching 'char + boundary'
        {
            let process = Process::new("'a', is_bound").unwrap();
            let text = "ba";
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(1, 2, "a")));
            assert_eq!(matches.next(), None);
        }

        {
            let process = Process::new("'a', is_bound").unwrap();
            let text = "a";
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(0, 1, "a")));
            assert_eq!(matches.next(), None);
        }

        {
            let process = Process::new("'a', is_bound").unwrap();
            let text = "a ";
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(0, 1, "a")));
            assert_eq!(matches.next(), None);
        }

        {
            let process = Process::new("'a', is_bound").unwrap();
            let text = "ab";
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_optional() {
        // char optional
        {
            let process = Process::new("'a', 'b'?, 'c'").unwrap();
            let text = "ababccbacabc";
            //               "  ^^^  ^^vvv"
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(2, 5, "abc")));
            assert_eq!(matches.next(), Some(single_group(7, 9, "ac")));
            assert_eq!(matches.next(), Some(single_group(9, 12, "abc")));
            assert_eq!(matches.next(), None);
        }

        // char optional - greedy
        {
            let process = Process::new("'a', 'b', 'c'?").unwrap();
            let text = "abcabx";
            //               "^^^vv"
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(0, 3, "abc")));
            assert_eq!(matches.next(), Some(single_group(3, 5, "ab")));
            assert_eq!(matches.next(), None);
        }

        // char optional - lazy
        {
            let process = Process::new("'a', 'b', 'c'??").unwrap();
            let text = "abcabx";
            //               "^^ ^^ "
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(0, 2, "ab")));
            assert_eq!(matches.next(), Some(single_group(3, 5, "ab")));
            assert_eq!(matches.next(), None);
        }

        // group optional
        {
            let process = Process::new("'a', ('b','c')?, 'd'").unwrap();
            let text = "abcabdacdabcdabacad";
            //               "         ^^^^    ^^"
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(9, 13, "abcd")));
            assert_eq!(matches.next(), Some(single_group(17, 19, "ad")));
            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_repetition_specified() {
        // char repetition
        {
            let process = Process::new("'a'{3}").unwrap();
            let text = "abaabbaaabbbaaaaa";
            //               "      ^^^   ^^^  "
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(6, 9, "aaa")));
            assert_eq!(matches.next(), Some(single_group(12, 15, "aaa")));
            assert_eq!(matches.next(), None);
        }

        // charset repetition
        {
            let process = Process::new("char_digit{3}").unwrap();
            let text = "a1ab12abc123abcd1234";
            //               "         ^^^    ^^^ "
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(9, 12, "123")));
            assert_eq!(matches.next(), Some(single_group(16, 19, "123")));
            assert_eq!(matches.next(), None);
        }

        // group repetition
        {
            let process = Process::new("('a','b'){3}").unwrap();
            let text = "abbaababbaababababab";
            //               "          ^^^^^^    "
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(10, 16, "ababab")));
            assert_eq!(matches.next(), None);
        }

        // repetition + other pattern
        {
            let process = Process::new("'a'{2}, char_digit").unwrap();
            let text = "abaabbaa1bb1aa123bb123a11b11";
            //               "      ^^^   ^^^             "
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(6, 9, "aa1")));
            assert_eq!(matches.next(), Some(single_group(12, 15, "aa1")));
            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_repetition_range() {
        // char repetition
        {
            let process = Process::new("'a'{1,3}").unwrap();
            let text = "abaabbaaabbbaaaabbbb";
            //               "^ ^^  ^^^   ^^^v    "
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(0, 1, "a")));
            assert_eq!(matches.next(), Some(single_group(2, 4, "aa")));
            assert_eq!(matches.next(), Some(single_group(6, 9, "aaa")));
            assert_eq!(matches.next(), Some(single_group(12, 15, "aaa")));
            assert_eq!(matches.next(), Some(single_group(15, 16, "a")));
            assert_eq!(matches.next(), None);
        }

        // char repetition lazy
        {
            let process = Process::new("'a'{1,3}?").unwrap();
            let text = "abaabbaaabbbaaaabbbb";
            //               "^ ^v  ^v^   ^v^v    "
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(0, 1, "a")));
            assert_eq!(matches.next(), Some(single_group(2, 3, "a")));
            assert_eq!(matches.next(), Some(single_group(3, 4, "a")));
            assert_eq!(matches.next(), Some(single_group(6, 7, "a")));
            assert_eq!(matches.next(), Some(single_group(7, 8, "a")));
            // omit the follow up
        }

        // char repetition - to MAX
        {
            let process = Process::new("'a'{2,}").unwrap();
            let text = "abaabbaaabbbaaaabbbb";
            //               "  ^^  ^^^   ^^^^    "
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(2, 4, "aa")));
            assert_eq!(matches.next(), Some(single_group(6, 9, "aaa")));
            assert_eq!(matches.next(), Some(single_group(12, 16, "aaaa")));
            assert_eq!(matches.next(), None);
        }

        // char repetition - to MAX - lazy
        {
            let process = Process::new("'a'{2,}?").unwrap();
            let text = "abaabbaaabbbaaaabbbb";
            //               "  ^^  ^^    ^^vv    "
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(2, 4, "aa")));
            assert_eq!(matches.next(), Some(single_group(6, 8, "aa")));
            assert_eq!(matches.next(), Some(single_group(12, 14, "aa")));
            assert_eq!(matches.next(), Some(single_group(14, 16, "aa")));
            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_optional_and_repetition_range() {
        // implicit
        {
            let process = Process::new("'a', 'b'{0,3}, 'c'").unwrap();
            let text = "acaabcaabbcaabbbcaabbbbc";
            //               "^^ ^^^ ^^^^ ^^^^^       "
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(0, 2, "ac")));
            assert_eq!(matches.next(), Some(single_group(3, 6, "abc")));
            assert_eq!(matches.next(), Some(single_group(7, 11, "abbc")));
            assert_eq!(matches.next(), Some(single_group(12, 17, "abbbc")));
            assert_eq!(matches.next(), None);
        }

        // explicit
        {
            let process = Process::new("'a', ('b'{2,3})?, 'c'").unwrap();
            let text = "acaabcaabbcaabbbcaabbbbc";
            //               "^^     ^^^^ ^^^^^       "
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(0, 2, "ac")));
            assert_eq!(matches.next(), Some(single_group(7, 11, "abbc")));
            assert_eq!(matches.next(), Some(single_group(12, 17, "abbbc")));
            assert_eq!(matches.next(), None);
        }

        // repetition specified
        {
            let process = Process::new("'a', ('b'{2})?, 'c'").unwrap();
            let text = "acaabcaabbcaabbbcaabbbbc";
            //               "^^     ^^^^             "
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(0, 2, "ac")));
            assert_eq!(matches.next(), Some(single_group(7, 11, "abbc")));
            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_repetition_char_any() {
        // repetition specified
        {
            let process = Process::new("char_any{3}").unwrap();
            let text = "abcdefgh";
            //               "^^^vvv  "
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(0, 3, "abc")));
            assert_eq!(matches.next(), Some(single_group(3, 6, "def")));
            assert_eq!(matches.next(), None);
        }

        // repetition range - to MAX
        {
            let process = Process::new("char_any+").unwrap();
            let text = "abcdefg";
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(0, 7, "abcdefg")));
            assert_eq!(matches.next(), None);
        }
    }

    #[test]
    fn test_process_repetition_backtracking() {
        // backtracking
        {
            let process = Process::new("start, 'a', char_any+, 'c'").unwrap();
            let text = "abbcmn";
            //               "^^^^  "
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(0, 4, "abbc")));
        }

        // backtracking - failed
        // because there is no char between 'a' and 'c'
        {
            let process = Process::new("start, 'a', char_any+, 'c'").unwrap();
            let text = "acmn";
            let mut matches = process.find_iter(text);
            assert_eq!(matches.next(), None);
        }

        // backtracking - failed
        // because there is not enough char between 'a' and 'c'
        {
            let process = Process::new("start, 'a', char_any{3,}, 'c'").unwrap();
            let text = "abbcmn";
            let mut matches = process.find_iter(text);
            assert_eq!(matches.next(), None);
        }

        // lazy repetition - no backtracking
        {
            let process = Process::new("'a', char_any+?, 'c'").unwrap();
            let text = "abbcmn";
            //               "^^^^  "
            let mut matches = process.find_iter(text);

            assert_eq!(matches.next(), Some(single_group(0, 4, "abbc")));
        }

        // nested backtracking
        // todo!()
    }

    #[test]
    fn test_process_capture() {
        //
    }
}
