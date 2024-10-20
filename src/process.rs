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
    route: Route,
}

impl Process {
    pub fn new(pattern: &str) -> Result<Self, Error> {
        let route = compile_from_str(pattern)?;

        // DEBUG::
        println!("{}", route.get_debug_text());

        Ok(Process { route })
    }

    pub fn new_instance<'a, 'b: 'a>(&'a self, text: &'b str) -> Instance {
        let bytes = text.as_bytes();
        Instance::new(&self.route, bytes)
    }
}

impl<'a, 'b> Instance<'a, 'b> {
    pub fn exec(&mut self, start: usize) -> Option<&Vec<MatchRange>> {
        if !start_main_thread(self, start, self.bytes.len()) {
            return None;
        }

        Some(&self.match_ranges)
    }

    pub fn exec_with_values(&mut self, start: usize) -> Option<Vec<MatchGroup>> {
        let capture_names = self.route.get_capture_group_names();
        let bytes = self.bytes;

        if let Some(match_ranges) = self.exec(start) {
            let match_groups: Vec<MatchGroup> = match_ranges
                .iter()
                .zip(capture_names.iter())
                .map(|(range, name_opt)| MatchGroup {
                    start: range.start,
                    end: range.end,
                    name: (*name_opt).map(|item| item.to_owned()),
                    value: sub_string(bytes, range.start, range.end).to_owned(),
                })
                .collect();

            Some(match_groups)
        } else {
            None
        }
    }

    pub fn get_group_index_by_name(&self, name: &str) -> Option<usize> {
        self.route.get_capture_group_index_by_name(name)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct MatchGroup {
    pub name: Option<String>,
    pub value: String, // String,
    pub start: usize,  // position included
    pub end: usize,    // position excluded
}

impl MatchGroup {
    pub fn new(name: Option<String>, value: String, start: usize, end: usize) -> Self {
        MatchGroup {
            name,
            value,
            start,
            end,
        }
    }
}

pub fn get_group_by_name<'a>(groups: &'a [MatchGroup], name: &str) -> Option<&'a MatchGroup> {
    groups.iter().find(|item| match &item.name {
        Some(s) => s == name,
        None => false,
    })
}

pub fn sub_string(bytes: &[u8], start: usize, end_excluded: usize) -> &str {
    /*
     * convert Vec<char> into String:
     * `let s:String = chars.iter().collect()`
     * or
     * `let s = String::from_iter(&chars)`
     */
    let slice = &bytes[start..end_excluded];
    str::from_utf8(slice).unwrap()
}

fn start_main_thread(instance: &mut Instance, mut start: usize, end: usize) -> bool {
    // allocate the vector of 'capture positions' and 'repetition counters'
    let number_of_capture_groups = instance.route.get_number_of_capture_groups();
    let number_of_counters = instance.route.get_number_of_counters();
    let main_thread = Thread::new(start, end, MAIN_LINE_INDEX);

    instance.threads = vec![main_thread];
    instance.match_ranges = vec![MatchRange::default(); number_of_capture_groups];
    instance.counters = vec![0; number_of_counters];
    instance.anchors = vec![vec![]; number_of_counters];

    while start < end {
        if start_thread(instance, start) {
            return true;
        }

        if instance.route.lines[MAIN_LINE_INDEX].fixed_start {
            break;
        }

        // move one character forward and try again
        let (_, byte_length) = read_char(instance.bytes, start);
        start += byte_length;
    }

    false
}

fn start_thread(instance: &mut Instance, position: usize) -> bool {
    let (line_index, entry_node_index, exit_node_index) = {
        let thread = instance.get_current_thread_ref();
        let line_index = thread.line_index;
        let line = &instance.route.lines[line_index];
        (line_index, line.start_node_index, line.end_node_index)
    };

    // DEBUG::
    println!(
        ">>THREAD START, line: {}, entry node: {}, position: {}",
        line_index, entry_node_index, position
    );

    // add transitions of the entry node
    instance.append_tasks_by_node(entry_node_index, position);

    // take the last task
    while let Some(task) = instance.get_current_thread_ref_mut().stack.pop() {
        // get the transition
        let line = &instance.route.lines[line_index];
        let node = &line.nodes[task.node_index];
        let transition_item = &node.transition_items[task.transition_index];

        // DEBUG::
        println!("> node: {}, position: {}", task.node_index, task.position);

        let position = task.position;
        let transition = &transition_item.transition;
        let target_node_index = transition_item.target_node_index;

        let check_result = transition.check(instance, position);
        match check_result {
            CheckResult::Success(forward) => {
                // DEBUG::
                println!(
                    "  trans: {}, forward: {}, -> node: {}",
                    transition, forward, target_node_index
                );

                if target_node_index == exit_node_index {
                    println!(
                        "  THREAD FINISH, line: {}, node: {}",
                        line_index, exit_node_index
                    );
                    return true;
                }

                instance.append_tasks_by_node(target_node_index, position + forward);
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
    use crate::{instance::MatchRange, process::MatchGroup};

    use super::Process;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_process_char() {
        // exists in the middle and at the end of the text
        {
            let process = Process::new("'a'").unwrap();
            let mut instance = process.new_instance("babbaa");

            assert_eq!(instance.exec(0), Some(&vec![MatchRange::new(1, 2)]));
            assert_eq!(instance.exec(1), Some(&vec![MatchRange::new(1, 2)]));

            assert_eq!(instance.exec(2), Some(&vec![MatchRange::new(4, 5)]));
            assert_eq!(instance.exec(3), Some(&vec![MatchRange::new(4, 5)]));
            assert_eq!(instance.exec(4), Some(&vec![MatchRange::new(4, 5)]));

            assert_eq!(instance.exec(5), Some(&vec![MatchRange::new(5, 6)]));

            // exceed the length of text
            assert_eq!(instance.exec(6), None);
        }

        // exists in the middle and at the beginning of the text
        {
            let process = Process::new("'a'").unwrap();
            let mut instance = process.new_instance("abaabb");

            assert_eq!(instance.exec(0), Some(&vec![MatchRange::new(0, 1)]));

            assert_eq!(instance.exec(1), Some(&vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(2), Some(&vec![MatchRange::new(2, 3)]));

            assert_eq!(instance.exec(3), Some(&vec![MatchRange::new(3, 4)]));

            assert_eq!(instance.exec(4), None);
            assert_eq!(instance.exec(5), None);

            // exceed the length of text
            assert_eq!(instance.exec(6), None);
        }

        // non-existent
        {
            let process = Process::new("'a'").unwrap();
            let mut instance = process.new_instance("xyz");

            assert_eq!(instance.exec(0), None);
            assert_eq!(instance.exec(1), None);
            assert_eq!(instance.exec(2), None);

            // exceed the length of text
            assert_eq!(instance.exec(3), None);
        }

        // exec_with_values
        {
            let process = Process::new("'a'").unwrap();
            let mut instance = process.new_instance("babbaa");

            assert_eq!(
                instance.exec_with_values(0).unwrap(),
                vec![MatchGroup::new(None, "a".to_owned(), 1, 2),]
            );

            assert_eq!(
                instance.exec_with_values(2).unwrap(),
                vec![MatchGroup::new(None, "a".to_owned(), 4, 5),]
            );

            assert_eq!(
                instance.exec_with_values(5).unwrap(),
                vec![MatchGroup::new(None, "a".to_owned(), 5, 6),]
            );

            assert_eq!(instance.exec_with_values(6), None);
        }
    }

    #[test]
    fn test_process_char_with_utf8() {
        // existent
        {
            let process = Process::new("'文'").unwrap();
            let mut instance = process.new_instance("abc中文字符文字🌏人文");

            assert_eq!(instance.exec(0), Some(&vec![MatchRange::new(6, 9)]));
            assert_eq!(instance.exec(9), Some(&vec![MatchRange::new(15, 18)]));
            assert_eq!(instance.exec(18), Some(&vec![MatchRange::new(28, 31)]));
            assert_eq!(instance.exec(31), None);
        }

        // exec_with_values
        {
            let process = Process::new("'文'").unwrap();
            let mut instance = process.new_instance("abc中文字符文字🌏人文");

            assert_eq!(
                instance.exec_with_values(0).unwrap(),
                vec![MatchGroup::new(None, "文".to_owned(), 6, 9),]
            );

            assert_eq!(
                instance.exec_with_values(9).unwrap(),
                vec![MatchGroup::new(None, "文".to_owned(), 15, 18),]
            );

            assert_eq!(
                instance.exec_with_values(18).unwrap(),
                vec![MatchGroup::new(None, "文".to_owned(), 28, 31),]
            );

            assert_eq!(instance.exec_with_values(31), None);
        }
    }

    #[test]
    fn test_process_string() {
        // existent
        {
            let process = Process::new("\"abc\"").unwrap();
            let text = "ababcbcabc";
            let mut instance = process.new_instance(text);

            assert_eq!(instance.exec(0), Some(&vec![MatchRange::new(2, 5)]));
            assert_eq!(instance.exec(1), Some(&vec![MatchRange::new(2, 5)]));
            assert_eq!(instance.exec(2), Some(&vec![MatchRange::new(2, 5)]));

            assert_eq!(instance.exec(3), Some(&vec![MatchRange::new(7, 10)]));
            assert_eq!(instance.exec(5), Some(&vec![MatchRange::new(7, 10)]));
            assert_eq!(instance.exec(7), Some(&vec![MatchRange::new(7, 10)]));

            assert_eq!(instance.exec(8), None);

            // exceed the length of text
            assert_eq!(instance.exec(10), None);
        }

        // non-existent
        {
            let process = Process::new("\"abc\"").unwrap();
            let text = "uvwxyz";
            let mut instance = process.new_instance(text);

            assert_eq!(instance.exec(0), None);
            assert_eq!(instance.exec(1), None);
            assert_eq!(instance.exec(5), None);

            assert_eq!(instance.exec(6), None);
        }

        // exec_with_values
        {
            let process = Process::new("\"abc\"").unwrap();
            let text = "ababcbcabc";
            let mut instance = process.new_instance(text);

            assert_eq!(
                instance.exec_with_values(0).unwrap(),
                vec![MatchGroup::new(None, "abc".to_owned(), 2, 5),]
            );

            assert_eq!(
                instance.exec_with_values(5).unwrap(),
                vec![MatchGroup::new(None, "abc".to_owned(), 7, 10),]
            );

            assert_eq!(instance.exec_with_values(10), None);
        }
    }

    #[test]
    fn test_process_string_with_utf8() {
        {
            let process = Process::new("\"文字\"").unwrap();
            let text = "abc文字文本象形文字🎁表情文字";
            let mut instance = process.new_instance(text);

            assert_eq!(instance.exec(0), Some(&vec![MatchRange::new(3, 9)]));
            assert_eq!(instance.exec(9), Some(&vec![MatchRange::new(21, 27)]));
            assert_eq!(instance.exec(27), Some(&vec![MatchRange::new(37, 43)]));

            // exceed the length of text
            assert_eq!(instance.exec(43), None);
        }

        // exec_with_values
        {
            let process = Process::new("\"文字\"").unwrap();
            let text = "abc文字文本象形文字🎁表情文字";
            let mut instance = process.new_instance(text);

            assert_eq!(
                instance.exec_with_values(0).unwrap(),
                vec![MatchGroup::new(None, "文字".to_owned(), 3, 9),]
            );

            assert_eq!(
                instance.exec_with_values(9).unwrap(),
                vec![MatchGroup::new(None, "文字".to_owned(), 21, 27),]
            );

            assert_eq!(
                instance.exec_with_values(27).unwrap(),
                vec![MatchGroup::new(None, "文字".to_owned(), 37, 43),]
            );

            assert_eq!(instance.exec_with_values(43), None);
        }
    }

    #[test]
    fn test_process_preset_charset() {
        {
            let process = Process::new("char_word").unwrap();
            let text = "a*1**_ **";
            //               "^ ^  ^   "
            let mut instance = process.new_instance(text);
            assert_eq!(instance.exec(0), Some(&vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(&vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(&vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        {
            let process = Process::new("char_not_word").unwrap();
            let text = "!a@12 bc_";
            //               "v v  v   "
            let mut instance = process.new_instance(text);
            assert_eq!(instance.exec(0), Some(&vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(&vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(&vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        {
            let process = Process::new("char_digit").unwrap();
            let text = "1a2b_3de*";
            //               "^ ^  ^   "
            let mut instance = process.new_instance(text);
            assert_eq!(instance.exec(0), Some(&vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(&vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(&vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        {
            let process = Process::new("char_not_digit").unwrap();
            let text = "a1_23 456";
            //               "v v  v   "
            let mut instance = process.new_instance(text);
            assert_eq!(instance.exec(0), Some(&vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(&vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(&vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        {
            let process = Process::new("char_space").unwrap();
            let text = " 1\tab\n_*!";
            //               "^ ^-  ^-   "
            //                012 345 678
            let mut instance = process.new_instance(text);
            assert_eq!(instance.exec(0), Some(&vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(&vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(&vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        {
            let process = Process::new("char_not_space").unwrap();
            let text = "a\t1\r\n*   ";
            //               "v  v    v   "
            //                01 23 4 5678
            let mut instance = process.new_instance(text);
            assert_eq!(instance.exec(0), Some(&vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(&vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(&vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        // exec_with_values
        {
            let process = Process::new("char_word").unwrap();
            let text = "a*1**_ **";
            //               "^ ^  ^   "
            let mut instance = process.new_instance(text);

            assert_eq!(
                instance.exec_with_values(0).unwrap(),
                vec![MatchGroup::new(None, "a".to_owned(), 0, 1),]
            );

            assert_eq!(
                instance.exec_with_values(1).unwrap(),
                vec![MatchGroup::new(None, "1".to_owned(), 2, 3),]
            );

            assert_eq!(
                instance.exec_with_values(3).unwrap(),
                vec![MatchGroup::new(None, "_".to_owned(), 5, 6),]
            );

            assert_eq!(instance.exec_with_values(6), None);
        }
    }

    #[test]
    fn test_process_charset() {
        // chars
        {
            let process = Process::new("['a','b','c']").unwrap();
            let text = "adbefcghi";
            //               "^ ^  ^   "
            let mut instance = process.new_instance(text);
            assert_eq!(instance.exec(0), Some(&vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(&vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(&vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        // negative
        {
            let process = Process::new("!['a','b','c']").unwrap();
            let text = "xa1bb*ccc";
            //               "v v  v   "
            let mut instance = process.new_instance(text);
            assert_eq!(instance.exec(0), Some(&vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(&vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(&vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        // range
        {
            let process = Process::new("['a'..'c']").unwrap();
            let text = "adbefcghi";
            //               "^ ^  ^   "
            let mut instance = process.new_instance(text);
            assert_eq!(instance.exec(0), Some(&vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(&vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(&vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        // negative
        {
            let process = Process::new("!['a'..'c']").unwrap();
            let text = "xa1bb*ccc";
            //               "v v  v   "
            let mut instance = process.new_instance(text);
            assert_eq!(instance.exec(0), Some(&vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(&vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(&vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        // ranges
        {
            let process = Process::new("['a'..'f', '0'..'9']").unwrap();
            let text = "am1npfq*_";
            //               "^ ^  ^   "
            let mut instance = process.new_instance(text);
            assert_eq!(instance.exec(0), Some(&vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(&vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(&vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        // negative
        {
            let process = Process::new("!['a'..'f', '0'..'9']").unwrap();
            let text = "man12*def";
            //               "v v  v   "
            let mut instance = process.new_instance(text);
            assert_eq!(instance.exec(0), Some(&vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(&vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(&vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        // combine range with preset
        {
            let process = Process::new("['a'..'f', char_digit]").unwrap();
            let text = "am1npfq*_";
            //               "^ ^  ^   ""
            let mut instance = process.new_instance(text);
            assert_eq!(instance.exec(0), Some(&vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(&vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(&vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        // negative
        {
            let process = Process::new("!['a'..'f', char_digit]").unwrap();
            let text = "man12*def";
            //               "v v  v   "
            let mut instance = process.new_instance(text);
            assert_eq!(instance.exec(0), Some(&vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(&vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(&vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        // nested
        {
            let process = Process::new("[['a','b','c','d'..'f'], ['0'..'8'], '9']").unwrap();
            let text = "am1npfq*_";
            //               "^ ^  ^   "
            let mut instance = process.new_instance(text);
            assert_eq!(instance.exec(0), Some(&vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(&vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(&vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        // negative
        {
            let process = Process::new("![['a','b','c','d'..'f'], ['0'..'8'], '9']").unwrap();
            let text = "man12*def";
            //               "v v  v   "
            let mut instance = process.new_instance(text);
            assert_eq!(instance.exec(0), Some(&vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(&vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(&vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        // chars - exec_with_values
        {
            let process = Process::new("['a','b','c']").unwrap();
            let text = "adbefcghi";
            let mut instance = process.new_instance(text);

            assert_eq!(
                instance.exec_with_values(0).unwrap(),
                vec![MatchGroup::new(None, "a".to_owned(), 0, 1),]
            );

            assert_eq!(
                instance.exec_with_values(2).unwrap(),
                vec![MatchGroup::new(None, "b".to_owned(), 2, 3),]
            );

            assert_eq!(
                instance.exec_with_values(3).unwrap(),
                vec![MatchGroup::new(None, "c".to_owned(), 5, 6),]
            );

            assert_eq!(instance.exec_with_values(6), None);
        }
    }

    #[test]
    fn test_process_charset_with_utf8() {
        {
            let process = Process::new("['文','字','🍅']").unwrap();
            let text = "abc正文写字🍉宋体字体🍅测试🍋";
            let mut instance = process.new_instance(text);

            assert_eq!(instance.exec(0), Some(&vec![MatchRange::new(6, 9)])); // '文'
            assert_eq!(instance.exec(9), Some(&vec![MatchRange::new(12, 15)])); // '字'
            assert_eq!(instance.exec(15), Some(&vec![MatchRange::new(25, 28)])); // '字'
            assert_eq!(instance.exec(28), Some(&vec![MatchRange::new(31, 35)])); // '🍅'
            assert_eq!(instance.exec(35), None);
        }

        // negative
        {
            let process = Process::new("!['文','字','🍅']").unwrap();
            let text = "哦字文🍅文噢字🍅文文字字喔";
            let mut instance = process.new_instance(text);

            assert_eq!(instance.exec(0), Some(&vec![MatchRange::new(0, 3)])); // '哦'
            assert_eq!(instance.exec(3), Some(&vec![MatchRange::new(16, 19)])); // '噢'
            assert_eq!(instance.exec(19), Some(&vec![MatchRange::new(38, 41)])); // '喔'
            assert_eq!(instance.exec(41), None);
        }

        // exec_with_values
        {
            let process = Process::new("['文','字','🍅']").unwrap();
            let text = "abc正文写字🍉宋体字体🍅测试🍋";
            let mut instance = process.new_instance(text);

            assert_eq!(
                instance.exec_with_values(0).unwrap(),
                vec![MatchGroup::new(None, "文".to_owned(), 6, 9)]
            );
            assert_eq!(
                instance.exec_with_values(9).unwrap(),
                vec![MatchGroup::new(None, "字".to_owned(), 12, 15)]
            );
            assert_eq!(
                instance.exec_with_values(15).unwrap(),
                vec![MatchGroup::new(None, "字".to_owned(), 25, 28)]
            );
            assert_eq!(
                instance.exec_with_values(28).unwrap(),
                vec![MatchGroup::new(None, "🍅".to_owned(), 31, 35)]
            );
            assert_eq!(instance.exec_with_values(35), None);
        }

        // exec_with_values - negative
        {
            let process = Process::new("!['文','字','🍅']").unwrap();
            let text = "哦字文🍅文噢字🍅文文字字喔";
            let mut instance = process.new_instance(text);

            assert_eq!(
                instance.exec_with_values(0).unwrap(),
                vec![MatchGroup::new(None, "哦".to_owned(), 0, 3)]
            );
            assert_eq!(
                instance.exec_with_values(3).unwrap(),
                vec![MatchGroup::new(None, "噢".to_owned(), 16, 19)]
            );
            assert_eq!(
                instance.exec_with_values(19).unwrap(),
                vec![MatchGroup::new(None, "喔".to_owned(), 38, 41)]
            );
            assert_eq!(instance.exec_with_values(41), None);
        }
    }
}
