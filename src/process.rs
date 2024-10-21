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

    // pub fn new_instance<'a, 'b: 'a>(&'a self, text: &'b str) -> Instance {
    //     let bytes = text.as_bytes();
    //     Instance::new(&self.route, bytes)
    // }
}

impl<'a> Instance<'a> {
    pub fn exec(&mut self, route: &Route, start: usize) -> Option<&Vec<MatchRange>> {
        if !start_main_thread(self, route, start, self.bytes.len()) {
            return None;
        }

        Some(&self.match_ranges)
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

// #[derive(Debug, PartialEq, Clone)]
// pub struct MatchGroup {
//     pub name: Option<String>,
//     pub value: String, // String,
//     pub start: usize,  // position included
//     pub end: usize,    // position excluded
// }
//
// impl MatchGroup {
//     pub fn new(name: Option<String>, value: String, start: usize, end: usize) -> Self {
//         MatchGroup {
//             name,
//             value,
//             start,
//             end,
//         }
//     }
// }
//
// pub fn get_group_by_name<'a>(groups: &'a [MatchGroup], name: &str) -> Option<&'a MatchGroup> {
//     groups.iter().find(|item| match &item.name {
//         Some(s) => s == name,
//         None => false,
//     })
// }
//
// pub fn sub_string(bytes: &[u8], start: usize, end_excluded: usize) -> &str {
//     /*
//      * convert Vec<char> into String:
//      * `let s:String = chars.iter().collect()`
//      * or
//      * `let s = String::from_iter(&chars)`
//      */
//     let slice = &bytes[start..end_excluded];
//     str::from_utf8(slice).unwrap()
// }

fn start_main_thread(instance: &mut Instance, route: &Route, mut start: usize, end: usize) -> bool {
    // allocate the vector of 'capture positions' and 'repetition counters'
    let number_of_capture_groups = route.get_number_of_capture_groups();
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
    use crate::instance::{Instance, MatchRange};

    use super::Process;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_process_char() {
        // exists in the middle and at the end of the text
        {
            let process = Process::new("'a'").unwrap();
            let mut instance = Instance::new("babbaa");

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(1, 2)])
            );
            assert_eq!(
                instance.exec(&process.route, 1),
                Some(&vec![MatchRange::new(1, 2)])
            );

            assert_eq!(
                instance.exec(&process.route, 2),
                Some(&vec![MatchRange::new(4, 5)])
            );
            assert_eq!(
                instance.exec(&process.route, 3),
                Some(&vec![MatchRange::new(4, 5)])
            );
            assert_eq!(
                instance.exec(&process.route, 4),
                Some(&vec![MatchRange::new(4, 5)])
            );

            assert_eq!(
                instance.exec(&process.route, 5),
                Some(&vec![MatchRange::new(5, 6)])
            );

            // exceed the length of text
            assert_eq!(instance.exec(&process.route, 6), None);
        }

        // exists in the middle and at the beginning of the text
        {
            let process = Process::new("'a'").unwrap();
            let mut instance = Instance::new("abaabb");

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );

            assert_eq!(
                instance.exec(&process.route, 1),
                Some(&vec![MatchRange::new(2, 3)])
            );
            assert_eq!(
                instance.exec(&process.route, 2),
                Some(&vec![MatchRange::new(2, 3)])
            );

            assert_eq!(
                instance.exec(&process.route, 3),
                Some(&vec![MatchRange::new(3, 4)])
            );

            assert_eq!(instance.exec(&process.route, 4), None);
            assert_eq!(instance.exec(&process.route, 5), None);

            // exceed the length of text
            assert_eq!(instance.exec(&process.route, 6), None);
        }

        // non-existent
        {
            let process = Process::new("'a'").unwrap();
            let mut instance = Instance::new("xyz");

            assert_eq!(instance.exec(&process.route, 0), None);
            assert_eq!(instance.exec(&process.route, 1), None);
            assert_eq!(instance.exec(&process.route, 2), None);

            // exceed the length of text
            assert_eq!(instance.exec(&process.route, 3), None);
        }

        //         // exec_with_values
        //         {
        //             let process = Process::new("'a'").unwrap();
        //             let mut instance = Instance::new("babbaa");
        //
        //             assert_eq!(
        //                 instance.exec_with_values(0).unwrap(),
        //                 vec![MatchGroup::new(None, "a".to_owned(), 1, 2),]
        //             );
        //
        //             assert_eq!(
        //                 instance.exec_with_values(2).unwrap(),
        //                 vec![MatchGroup::new(None, "a".to_owned(), 4, 5),]
        //             );
        //
        //             assert_eq!(
        //                 instance.exec_with_values(5).unwrap(),
        //                 vec![MatchGroup::new(None, "a".to_owned(), 5, 6),]
        //             );
        //
        //             assert_eq!(instance.exec_with_values(6), None);
        //         }
    }

    #[test]
    fn test_process_char_with_utf8() {
        // existent
        {
            let process = Process::new("'文'").unwrap();
            let mut instance = Instance::new("abc中文字符文字🌏人文");

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(6, 9)])
            );
            assert_eq!(
                instance.exec(&process.route, 9),
                Some(&vec![MatchRange::new(15, 18)])
            );
            assert_eq!(
                instance.exec(&process.route, 18),
                Some(&vec![MatchRange::new(28, 31)])
            );
            assert_eq!(instance.exec(&process.route, 31), None);
        }

        //         // exec_with_values
        //         {
        //             let process = Process::new("'文'").unwrap();
        //             let mut instance = Instance::new("abc中文字符文字🌏人文");
        //
        //             assert_eq!(
        //                 instance.exec_with_values(0).unwrap(),
        //                 vec![MatchGroup::new(None, "文".to_owned(), 6, 9),]
        //             );
        //
        //             assert_eq!(
        //                 instance.exec_with_values(9).unwrap(),
        //                 vec![MatchGroup::new(None, "文".to_owned(), 15, 18),]
        //             );
        //
        //             assert_eq!(
        //                 instance.exec_with_values(18).unwrap(),
        //                 vec![MatchGroup::new(None, "文".to_owned(), 28, 31),]
        //             );
        //
        //             assert_eq!(instance.exec_with_values(31), None);
        //         }
    }

    #[test]
    fn test_process_string() {
        // existent
        {
            let process = Process::new("\"abc\"").unwrap();
            let text = "ababcbcabc";
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(2, 5)])
            );
            assert_eq!(
                instance.exec(&process.route, 1),
                Some(&vec![MatchRange::new(2, 5)])
            );
            assert_eq!(
                instance.exec(&process.route, 2),
                Some(&vec![MatchRange::new(2, 5)])
            );

            assert_eq!(
                instance.exec(&process.route, 3),
                Some(&vec![MatchRange::new(7, 10)])
            );
            assert_eq!(
                instance.exec(&process.route, 5),
                Some(&vec![MatchRange::new(7, 10)])
            );
            assert_eq!(
                instance.exec(&process.route, 7),
                Some(&vec![MatchRange::new(7, 10)])
            );

            assert_eq!(instance.exec(&process.route, 8), None);

            // exceed the length of text
            assert_eq!(instance.exec(&process.route, 10), None);
        }

        // non-existent
        {
            let process = Process::new("\"abc\"").unwrap();
            let text = "uvwxyz";
            let mut instance = Instance::new(text);

            assert_eq!(instance.exec(&process.route, 0), None);
            assert_eq!(instance.exec(&process.route, 1), None);
            assert_eq!(instance.exec(&process.route, 5), None);

            assert_eq!(instance.exec(&process.route, 6), None);
        }

        //         // exec_with_values
        //         {
        //             let process = Process::new("\"abc\"").unwrap();
        //             let text = "ababcbcabc";
        //             let mut instance = Instance::new(text);
        //
        //             assert_eq!(
        //                 instance.exec_with_values(0).unwrap(),
        //                 vec![MatchGroup::new(None, "abc".to_owned(), 2, 5),]
        //             );
        //
        //             assert_eq!(
        //                 instance.exec_with_values(5).unwrap(),
        //                 vec![MatchGroup::new(None, "abc".to_owned(), 7, 10),]
        //             );
        //
        //             assert_eq!(instance.exec_with_values(10), None);
        //         }
    }

    #[test]
    fn test_process_string_with_utf8() {
        {
            let process = Process::new("\"文字\"").unwrap();
            let text = "abc文字文本象形文字🎁表情文字";
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(3, 9)])
            );
            assert_eq!(
                instance.exec(&process.route, 9),
                Some(&vec![MatchRange::new(21, 27)])
            );
            assert_eq!(
                instance.exec(&process.route, 27),
                Some(&vec![MatchRange::new(37, 43)])
            );

            // exceed the length of text
            assert_eq!(instance.exec(&process.route, 43), None);
        }

        //         // exec_with_values
        //         {
        //             let process = Process::new("\"文字\"").unwrap();
        //             let text = "abc文字文本象形文字🎁表情文字";
        //             let mut instance = Instance::new(text);
        //
        //             assert_eq!(
        //                 instance.exec_with_values(0).unwrap(),
        //                 vec![MatchGroup::new(None, "文字".to_owned(), 3, 9),]
        //             );
        //
        //             assert_eq!(
        //                 instance.exec_with_values(9).unwrap(),
        //                 vec![MatchGroup::new(None, "文字".to_owned(), 21, 27),]
        //             );
        //
        //             assert_eq!(
        //                 instance.exec_with_values(27).unwrap(),
        //                 vec![MatchGroup::new(None, "文字".to_owned(), 37, 43),]
        //             );
        //
        //             assert_eq!(instance.exec_with_values(43), None);
        //         }
    }

    #[test]
    fn test_process_preset_charset() {
        {
            let process = Process::new("char_word").unwrap();
            let text = "a*1**_ **";
            //               "^ ^  ^   "
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );
            assert_eq!(
                instance.exec(&process.route, 1),
                Some(&vec![MatchRange::new(2, 3)])
            );
            assert_eq!(
                instance.exec(&process.route, 3),
                Some(&vec![MatchRange::new(5, 6)])
            );
            assert_eq!(instance.exec(&process.route, 6), None);
        }

        {
            let process = Process::new("char_not_word").unwrap();
            let text = "!a@12 bc_";
            //               "v v  v   "
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );
            assert_eq!(
                instance.exec(&process.route, 1),
                Some(&vec![MatchRange::new(2, 3)])
            );
            assert_eq!(
                instance.exec(&process.route, 3),
                Some(&vec![MatchRange::new(5, 6)])
            );
            assert_eq!(instance.exec(&process.route, 6), None);
        }

        {
            let process = Process::new("char_digit").unwrap();
            let text = "1a2b_3de*";
            //               "^ ^  ^   "
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );
            assert_eq!(
                instance.exec(&process.route, 1),
                Some(&vec![MatchRange::new(2, 3)])
            );
            assert_eq!(
                instance.exec(&process.route, 3),
                Some(&vec![MatchRange::new(5, 6)])
            );
            assert_eq!(instance.exec(&process.route, 6), None);
        }

        {
            let process = Process::new("char_not_digit").unwrap();
            let text = "a1_23 456";
            //               "v v  v   "
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );
            assert_eq!(
                instance.exec(&process.route, 1),
                Some(&vec![MatchRange::new(2, 3)])
            );
            assert_eq!(
                instance.exec(&process.route, 3),
                Some(&vec![MatchRange::new(5, 6)])
            );
            assert_eq!(instance.exec(&process.route, 6), None);
        }

        {
            let process = Process::new("char_space").unwrap();
            let text = " 1\tab\n_*!";
            //               "^ ^-  ^-   "
            //                012 345 678
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );
            assert_eq!(
                instance.exec(&process.route, 1),
                Some(&vec![MatchRange::new(2, 3)])
            );
            assert_eq!(
                instance.exec(&process.route, 3),
                Some(&vec![MatchRange::new(5, 6)])
            );
            assert_eq!(instance.exec(&process.route, 6), None);
        }

        {
            let process = Process::new("char_not_space").unwrap();
            let text = "a\t1\r\n*   ";
            //               "v  v    v   "
            //                01 23 4 5678
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );
            assert_eq!(
                instance.exec(&process.route, 1),
                Some(&vec![MatchRange::new(2, 3)])
            );
            assert_eq!(
                instance.exec(&process.route, 3),
                Some(&vec![MatchRange::new(5, 6)])
            );
            assert_eq!(instance.exec(&process.route, 6), None);
        }

        //         // exec_with_values
        //         {
        //             let process = Process::new("char_word").unwrap();
        //             let text = "a*1**_ **";
        //             //               "^ ^  ^   "
        //             let mut instance = Instance::new(text);
        //
        //             assert_eq!(
        //                 instance.exec_with_values(0).unwrap(),
        //                 vec![MatchGroup::new(None, "a".to_owned(), 0, 1),]
        //             );
        //
        //             assert_eq!(
        //                 instance.exec_with_values(1).unwrap(),
        //                 vec![MatchGroup::new(None, "1".to_owned(), 2, 3),]
        //             );
        //
        //             assert_eq!(
        //                 instance.exec_with_values(3).unwrap(),
        //                 vec![MatchGroup::new(None, "_".to_owned(), 5, 6),]
        //             );
        //
        //             assert_eq!(instance.exec_with_values(6), None);
        //         }
    }

    #[test]
    fn test_process_charset() {
        // chars
        {
            let process = Process::new("['a','b','c']").unwrap();
            let text = "adbefcghi";
            //               "^ ^  ^   "
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );
            assert_eq!(
                instance.exec(&process.route, 1),
                Some(&vec![MatchRange::new(2, 3)])
            );
            assert_eq!(
                instance.exec(&process.route, 3),
                Some(&vec![MatchRange::new(5, 6)])
            );
            assert_eq!(instance.exec(&process.route, 6), None);
        }

        // negative
        {
            let process = Process::new("!['a','b','c']").unwrap();
            let text = "xa1bb*ccc";
            //               "v v  v   "
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );
            assert_eq!(
                instance.exec(&process.route, 1),
                Some(&vec![MatchRange::new(2, 3)])
            );
            assert_eq!(
                instance.exec(&process.route, 3),
                Some(&vec![MatchRange::new(5, 6)])
            );
            assert_eq!(instance.exec(&process.route, 6), None);
        }

        // range
        {
            let process = Process::new("['a'..'c']").unwrap();
            let text = "adbefcghi";
            //               "^ ^  ^   "
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );
            assert_eq!(
                instance.exec(&process.route, 1),
                Some(&vec![MatchRange::new(2, 3)])
            );
            assert_eq!(
                instance.exec(&process.route, 3),
                Some(&vec![MatchRange::new(5, 6)])
            );
            assert_eq!(instance.exec(&process.route, 6), None);
        }

        // negative
        {
            let process = Process::new("!['a'..'c']").unwrap();
            let text = "xa1bb*ccc";
            //               "v v  v   "
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );
            assert_eq!(
                instance.exec(&process.route, 1),
                Some(&vec![MatchRange::new(2, 3)])
            );
            assert_eq!(
                instance.exec(&process.route, 3),
                Some(&vec![MatchRange::new(5, 6)])
            );
            assert_eq!(instance.exec(&process.route, 6), None);
        }

        // ranges
        {
            let process = Process::new("['a'..'f', '0'..'9']").unwrap();
            let text = "am1npfq*_";
            //               "^ ^  ^   "
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );
            assert_eq!(
                instance.exec(&process.route, 1),
                Some(&vec![MatchRange::new(2, 3)])
            );
            assert_eq!(
                instance.exec(&process.route, 3),
                Some(&vec![MatchRange::new(5, 6)])
            );
            assert_eq!(instance.exec(&process.route, 6), None);
        }

        // negative
        {
            let process = Process::new("!['a'..'f', '0'..'9']").unwrap();
            let text = "man12*def";
            //               "v v  v   "
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );
            assert_eq!(
                instance.exec(&process.route, 1),
                Some(&vec![MatchRange::new(2, 3)])
            );
            assert_eq!(
                instance.exec(&process.route, 3),
                Some(&vec![MatchRange::new(5, 6)])
            );
            assert_eq!(instance.exec(&process.route, 6), None);
        }

        // combine range with preset
        {
            let process = Process::new("['a'..'f', char_digit]").unwrap();
            let text = "am1npfq*_";
            //               "^ ^  ^   ""
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );
            assert_eq!(
                instance.exec(&process.route, 1),
                Some(&vec![MatchRange::new(2, 3)])
            );
            assert_eq!(
                instance.exec(&process.route, 3),
                Some(&vec![MatchRange::new(5, 6)])
            );
            assert_eq!(instance.exec(&process.route, 6), None);
        }

        // negative
        {
            let process = Process::new("!['a'..'f', char_digit]").unwrap();
            let text = "man12*def";
            //               "v v  v   "
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );
            assert_eq!(
                instance.exec(&process.route, 1),
                Some(&vec![MatchRange::new(2, 3)])
            );
            assert_eq!(
                instance.exec(&process.route, 3),
                Some(&vec![MatchRange::new(5, 6)])
            );
            assert_eq!(instance.exec(&process.route, 6), None);
        }

        // nested
        {
            let process = Process::new("[['a','b','c','d'..'f'], ['0'..'8'], '9']").unwrap();
            let text = "am1npfq*_";
            //               "^ ^  ^   "
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );
            assert_eq!(
                instance.exec(&process.route, 1),
                Some(&vec![MatchRange::new(2, 3)])
            );
            assert_eq!(
                instance.exec(&process.route, 3),
                Some(&vec![MatchRange::new(5, 6)])
            );
            assert_eq!(instance.exec(&process.route, 6), None);
        }

        // negative
        {
            let process = Process::new("![['a','b','c','d'..'f'], ['0'..'8'], '9']").unwrap();
            let text = "man12*def";
            //               "v v  v   "
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );
            assert_eq!(
                instance.exec(&process.route, 1),
                Some(&vec![MatchRange::new(2, 3)])
            );
            assert_eq!(
                instance.exec(&process.route, 3),
                Some(&vec![MatchRange::new(5, 6)])
            );
            assert_eq!(instance.exec(&process.route, 6), None);
        }

        //         // chars - exec_with_values
        //         {
        //             let process = Process::new("['a','b','c']").unwrap();
        //             let text = "adbefcghi";
        //             let mut instance = Instance::new(text);
        //
        //             assert_eq!(
        //                 instance.exec_with_values(0).unwrap(),
        //                 vec![MatchGroup::new(None, "a".to_owned(), 0, 1),]
        //             );
        //
        //             assert_eq!(
        //                 instance.exec_with_values(2).unwrap(),
        //                 vec![MatchGroup::new(None, "b".to_owned(), 2, 3),]
        //             );
        //
        //             assert_eq!(
        //                 instance.exec_with_values(3).unwrap(),
        //                 vec![MatchGroup::new(None, "c".to_owned(), 5, 6),]
        //             );
        //
        //             assert_eq!(instance.exec_with_values(6), None);
        //         }
    }

    #[test]
    fn test_process_charset_with_utf8() {
        {
            let process = Process::new("['文','字','🍅']").unwrap();
            let text = "abc正文写字🍉宋体字体🍅测试🍋";
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(6, 9)])
            ); // '文'
            assert_eq!(
                instance.exec(&process.route, 9),
                Some(&vec![MatchRange::new(12, 15)])
            ); // '字'
            assert_eq!(
                instance.exec(&process.route, 15),
                Some(&vec![MatchRange::new(25, 28)])
            ); // '字'
            assert_eq!(
                instance.exec(&process.route, 28),
                Some(&vec![MatchRange::new(31, 35)])
            ); // '🍅'
            assert_eq!(instance.exec(&process.route, 35), None);
        }

        // negative
        {
            let process = Process::new("!['文','字','🍅']").unwrap();
            let text = "哦字文🍅文噢字🍅文文字字喔";
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 3)])
            ); // '哦'
            assert_eq!(
                instance.exec(&process.route, 3),
                Some(&vec![MatchRange::new(16, 19)])
            ); // '噢'
            assert_eq!(
                instance.exec(&process.route, 19),
                Some(&vec![MatchRange::new(38, 41)])
            ); // '喔'
            assert_eq!(instance.exec(&process.route, 41), None);
        }

        //         // exec_with_values
        //         {
        //             let process = Process::new("['文','字','🍅']").unwrap();
        //             let text = "abc正文写字🍉宋体字体🍅测试🍋";
        //             let mut instance = Instance::new(text);
        //
        //             assert_eq!(
        //                 instance.exec_with_values(0).unwrap(),
        //                 vec![MatchGroup::new(None, "文".to_owned(), 6, 9)]
        //             );
        //             assert_eq!(
        //                 instance.exec_with_values(9).unwrap(),
        //                 vec![MatchGroup::new(None, "字".to_owned(), 12, 15)]
        //             );
        //             assert_eq!(
        //                 instance.exec_with_values(15).unwrap(),
        //                 vec![MatchGroup::new(None, "字".to_owned(), 25, 28)]
        //             );
        //             assert_eq!(
        //                 instance.exec_with_values(28).unwrap(),
        //                 vec![MatchGroup::new(None, "🍅".to_owned(), 31, 35)]
        //             );
        //             assert_eq!(instance.exec_with_values(35), None);
        //         }

        //         // exec_with_values - negative
        //         {
        //             let process = Process::new("!['文','字','🍅']").unwrap();
        //             let text = "哦字文🍅文噢字🍅文文字字喔";
        //             let mut instance = Instance::new(text);
        //
        //             assert_eq!(
        //                 instance.exec_with_values(0).unwrap(),
        //                 vec![MatchGroup::new(None, "哦".to_owned(), 0, 3)]
        //             );
        //             assert_eq!(
        //                 instance.exec_with_values(3).unwrap(),
        //                 vec![MatchGroup::new(None, "噢".to_owned(), 16, 19)]
        //             );
        //             assert_eq!(
        //                 instance.exec_with_values(19).unwrap(),
        //                 vec![MatchGroup::new(None, "喔".to_owned(), 38, 41)]
        //             );
        //             assert_eq!(instance.exec_with_values(41), None);
        //         }
    }

    #[test]
    fn test_process_special_char() {
        let process = Process::new("char_any").unwrap();
        let text = "\n \r\n  \n";
        //               "  ^    ^^  "
        let mut instance = Instance::new(text);

        assert_eq!(
            instance.exec(&process.route, 0),
            Some(&vec![MatchRange::new(1, 2)])
        );
        assert_eq!(
            instance.exec(&process.route, 2),
            Some(&vec![MatchRange::new(4, 5)])
        );
        assert_eq!(
            instance.exec(&process.route, 5),
            Some(&vec![MatchRange::new(5, 6)])
        );
        assert_eq!(instance.exec(&process.route, 6), None);
    }

    #[test]
    fn test_process_group() {
        // anreg group = a sequence of patterns
        {
            let process = Process::new("'a', 'b', 'c'").unwrap();
            let text = "ababcbcabc";
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(2, 5)])
            );
            assert_eq!(
                instance.exec(&process.route, 5),
                Some(&vec![MatchRange::new(7, 10)])
            );
            assert_eq!(instance.exec(&process.route, 10), None);
        }

        {
            let process = Process::new("'%', char_digit").unwrap();
            let text = "0123%567%9";
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(4, 6)])
            );
            assert_eq!(
                instance.exec(&process.route, 6),
                Some(&vec![MatchRange::new(8, 10)])
            );
            assert_eq!(instance.exec(&process.route, 10), None);
        }

        {
            let process = Process::new("['+','-'], ('%', char_digit)").unwrap();
            let text = "%12+%56-%9";
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(3, 6)])
            );
            assert_eq!(
                instance.exec(&process.route, 6),
                Some(&vec![MatchRange::new(7, 10)])
            );
            assert_eq!(instance.exec(&process.route, 10), None);
        }
    }

    #[test]
    fn test_process_start_and_end_assertion() {
        {
            let process = Process::new("start, 'a'").unwrap();
            let text = "ab";
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );
            assert_eq!(instance.exec(&process.route, 1), None);
        }

        {
            let process = Process::new("'a', end").unwrap();
            let text = "ab";
            let mut instance = Instance::new(text);
            assert_eq!(instance.exec(&process.route, 0), None);
        }

        {
            let process = Process::new("start, 'a'").unwrap();
            let text = "ba";
            let mut instance = Instance::new(text);
            assert_eq!(instance.exec(&process.route, 0), None);
        }

        {
            let process = Process::new("'a', end").unwrap();
            let text = "ba";
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(1, 2)])
            );
            assert_eq!(instance.exec(&process.route, 2), None);
        }

        // both 'start' and 'end'
        {
            let process = Process::new("start, 'a', end").unwrap();
            let text = "a";
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );
        }

        // both 'start' and 'end' - failed 1
        {
            let process = Process::new("start, 'a', end").unwrap();
            let text = "ab";
            let mut instance = Instance::new(text);
            assert_eq!(instance.exec(&process.route, 0), None);
        }

        // both 'start' and 'end' - failed 2
        {
            let process = Process::new("start, 'a', end").unwrap();
            let text = "ba";
            let mut instance = Instance::new(text);
            assert_eq!(instance.exec(&process.route, 0), None);
        }
    }

    #[test]
    fn test_process_boundary_assertion() {
        // matching 'boundary + char'
        {
            let process = Process::new("is_bound, 'a'").unwrap();
            let text = "ab";
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );
            assert_eq!(instance.exec(&process.route, 1), None);
        }

        {
            let process = Process::new("is_bound, 'a'").unwrap();
            let text = "a";
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );
            assert_eq!(instance.exec(&process.route, 1), None);
        }

        {
            let process = Process::new("is_bound, 'a'").unwrap();
            let text = " a";
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(1, 2)])
            );
            assert_eq!(instance.exec(&process.route, 2), None);
        }

        {
            let process = Process::new("is_bound, 'a'").unwrap();
            let text = "ba";
            let mut instance = Instance::new(text);
            assert_eq!(instance.exec(&process.route, 0), None);
        }

        // matching 'char + boundary'
        {
            let process = Process::new("'a', is_bound").unwrap();
            let text = "ba";
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(1, 2)])
            );
            assert_eq!(instance.exec(&process.route, 2), None);
        }

        {
            let process = Process::new("'a', is_bound").unwrap();
            let text = "a";
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );
            assert_eq!(instance.exec(&process.route, 1), None);
        }

        {
            let process = Process::new("'a', is_bound").unwrap();
            let text = "a ";
            let mut instance = Instance::new(text);
            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );
            assert_eq!(instance.exec(&process.route, 1), None);
        }

        {
            let process = Process::new("'a', is_bound").unwrap();
            let text = "ab";
            let mut instance = Instance::new(text);
            assert_eq!(instance.exec(&process.route, 0), None);
        }
    }

    #[test]
    fn test_process_optional() {
        // char optional
        {
            let process = Process::new("'a', 'b'?, 'c'").unwrap();
            let text = "ababccbacabc";
            //               "  ^^^  ^^vvv"
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(2, 5)])
            ); // "abc"
            assert_eq!(
                instance.exec(&process.route, 5),
                Some(&vec![MatchRange::new(7, 9)])
            ); // "ac"
            assert_eq!(
                instance.exec(&process.route, 9),
                Some(&vec![MatchRange::new(9, 12)])
            ); // "abc"
            assert_eq!(instance.exec(&process.route, 12), None);
        }

        // char optional - greedy
        {
            let process = Process::new("'a', 'b', 'c'?").unwrap();
            let text = "abcabx";
            //               "^^^vv"
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 3)])
            );
            assert_eq!(
                instance.exec(&process.route, 3),
                Some(&vec![MatchRange::new(3, 5)])
            );
            assert_eq!(instance.exec(&process.route, 5), None);
        }

        // char optional - lazy
        {
            let process = Process::new("'a', 'b', 'c'??").unwrap();
            let text = "abcabx";
            //               "^^ ^^ "
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 2)])
            );
            assert_eq!(
                instance.exec(&process.route, 2),
                Some(&vec![MatchRange::new(3, 5)])
            );
            assert_eq!(instance.exec(&process.route, 5), None);
        }

        // group optional
        {
            let process = Process::new("'a', ('b','c')?, 'd'").unwrap();
            let text = "abcabdacdabcdabacad";
            //               "         ^^^^    ^^"
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(9, 13)])
            );
            assert_eq!(
                instance.exec(&process.route, 13),
                Some(&vec![MatchRange::new(17, 19)])
            );
            assert_eq!(instance.exec(&process.route, 19), None);
        }
    }

    #[test]
    fn test_process_repetition_specified() {
        // char repetition
        {
            let process = Process::new("'a'{3}").unwrap();
            let text = "abaabbaaabbbaaaaa";
            //               "      ^^^   ^^^  "
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(6, 9)])
            );
            assert_eq!(
                instance.exec(&process.route, 9),
                Some(&vec![MatchRange::new(12, 15)])
            );
            assert_eq!(instance.exec(&process.route, 15), None);
        }

        // charset repetition
        {
            let process = Process::new("char_digit{3}").unwrap();
            let text = "a1ab12abc123abcd1234";
            //               "         ^^^    ^^^^"
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(9, 12)])
            );
            assert_eq!(
                instance.exec(&process.route, 12),
                Some(&vec![MatchRange::new(16, 19)])
            );
            assert_eq!(instance.exec(&process.route, 19), None);
        }

        // group repetition
        {
            let process = Process::new("('a','b'){3}").unwrap();
            let text = "abbaababbaababababab";
            //               "          ^^^^^^    "
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(10, 16)])
            );
            assert_eq!(instance.exec(&process.route, 16), None);
        }

        // repetition + other pattern
        {
            let process = Process::new("'a'{2}, char_digit").unwrap();
            let text = "abaabbaa1bb1aa123bb123a11b11";
            //               "      ^^^   ^^^             "
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(6, 9)])
            );
            assert_eq!(
                instance.exec(&process.route, 9),
                Some(&vec![MatchRange::new(12, 15)])
            );
            assert_eq!(instance.exec(&process.route, 15), None);
        }
    }

    #[test]
    fn test_process_repetition_range() {
        // char repetition
        {
            let process = Process::new("'a'{1,3}").unwrap();
            let text = "abaabbaaabbbaaaabbbb";
            //               "^ ^^  ^^^   ^vvv    "
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );
            assert_eq!(
                instance.exec(&process.route, 1),
                Some(&vec![MatchRange::new(2, 4)])
            );
            assert_eq!(
                instance.exec(&process.route, 4),
                Some(&vec![MatchRange::new(6, 9)])
            );
            assert_eq!(
                instance.exec(&process.route, 9),
                Some(&vec![MatchRange::new(12, 15)])
            );
            assert_eq!(
                instance.exec(&process.route, 15),
                Some(&vec![MatchRange::new(15, 16)])
            );
            assert_eq!(instance.exec(&process.route, 16), None);
        }

        // char repetition lazy
        {
            let process = Process::new("'a'{1,3}?").unwrap();
            let text = "abaabbaaabbbaaaabbbb";
            //               "^ ^v  ^v^   ^v^v    "
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 1)])
            );
            assert_eq!(
                instance.exec(&process.route, 1),
                Some(&vec![MatchRange::new(2, 3)])
            );
            assert_eq!(
                instance.exec(&process.route, 3),
                Some(&vec![MatchRange::new(3, 4)])
            );
            assert_eq!(
                instance.exec(&process.route, 4),
                Some(&vec![MatchRange::new(6, 7)])
            );
            assert_eq!(
                instance.exec(&process.route, 7),
                Some(&vec![MatchRange::new(7, 8)])
            );
            // omit the follow up
        }

        // char repetition - to MAX
        {
            let process = Process::new("'a'{2,}").unwrap();
            let text = "abaabbaaabbbaaaabbbb";
            //               "  ^^  ^^^   ^^^^    "
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(2, 4)])
            );
            assert_eq!(
                instance.exec(&process.route, 4),
                Some(&vec![MatchRange::new(6, 9)])
            );
            assert_eq!(
                instance.exec(&process.route, 9),
                Some(&vec![MatchRange::new(12, 16)])
            );
            assert_eq!(instance.exec(&process.route, 16), None);
        }

        // char repetition - to MAX - lazy
        {
            let process = Process::new("'a'{2,}?").unwrap();
            let text = "abaabbaaabbbaaaabbbb";
            //               "  ^^  ^^    ^^vv    "
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(2, 4)])
            );
            assert_eq!(
                instance.exec(&process.route, 4),
                Some(&vec![MatchRange::new(6, 8)])
            );
            assert_eq!(
                instance.exec(&process.route, 8),
                Some(&vec![MatchRange::new(12, 14)])
            );
            assert_eq!(
                instance.exec(&process.route, 14),
                Some(&vec![MatchRange::new(14, 16)])
            );
            assert_eq!(instance.exec(&process.route, 16), None);
        }
    }

    #[test]
    fn test_process_optional_and_repetition_range() {
        // implicit
        {
            let process = Process::new("'a', 'b'{0,3}, 'c'").unwrap();
            let text = "acaabcaabbcaabbbcaabbbbc";
            //               "^^ ^^^ ^^^^ ^^^^^       "
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 2)])
            );
            assert_eq!(
                instance.exec(&process.route, 2),
                Some(&vec![MatchRange::new(3, 6)])
            );
            assert_eq!(
                instance.exec(&process.route, 6),
                Some(&vec![MatchRange::new(7, 11)])
            );
            assert_eq!(
                instance.exec(&process.route, 11),
                Some(&vec![MatchRange::new(12, 17)])
            );
            assert_eq!(instance.exec(&process.route, 17), None);
        }

        // explicit
        {
            let process = Process::new("'a', ('b'{2,3})?, 'c'").unwrap();
            let text = "acaabcaabbcaabbbcaabbbbc";
            //               "^^     ^^^^ ^^^^^       "
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 2)])
            );
            assert_eq!(
                instance.exec(&process.route, 2),
                Some(&vec![MatchRange::new(7, 11)])
            );
            assert_eq!(
                instance.exec(&process.route, 11),
                Some(&vec![MatchRange::new(12, 17)])
            );
            assert_eq!(instance.exec(&process.route, 17), None);
        }

        // repetition specified
        {
            let process = Process::new("'a', ('b'{2})?, 'c'").unwrap();
            let text = "acaabcaabbcaabbbcaabbbbc";
            //               "^^     ^^^^             "
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 2)])
            );
            assert_eq!(
                instance.exec(&process.route, 2),
                Some(&vec![MatchRange::new(7, 11)])
            );
            assert_eq!(instance.exec(&process.route, 17), None);
        }
    }

    #[test]
    fn test_process_repetition_char_any() {
        // repetition specified
        {
            let process = Process::new("char_any{3}").unwrap();
            let text = "abcdefghijkl";
            //               "^^^vvv^^^vvv"
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 3)])
            );
            assert_eq!(
                instance.exec(&process.route, 3),
                Some(&vec![MatchRange::new(3, 6)])
            );
            assert_eq!(
                instance.exec(&process.route, 6),
                Some(&vec![MatchRange::new(6, 9)])
            );
            assert_eq!(
                instance.exec(&process.route, 9),
                Some(&vec![MatchRange::new(9, 12)])
            );
            assert_eq!(instance.exec(&process.route, 12), None);
        }

        // repetition range - to MAX
        {
            let process = Process::new("char_any+").unwrap();
            let text = "abcdefghijkl";
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 12)])
            );
            assert_eq!(instance.exec(&process.route, 12), None);
        }
    }

    #[test]
    fn test_process_repetition_backtracking() {
        // backtracking
        {
            let process = Process::new("start, 'a', char_any+, 'c'").unwrap();
            let text = "abbcmn";
            //               "^^^^  "
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 4)])
            );
        }

        // backtracking - failed
        // because there is no char between 'a' and 'c'
        {
            let process = Process::new("start, 'a', char_any+, 'c'").unwrap();
            let text = "acmn";
            let mut instance = Instance::new(text);
            assert_eq!(instance.exec(&process.route, 0), None);
        }

        // backtracking - failed
        // because there is not enough char between 'a' and 'c'
        {
            let process = Process::new("start, 'a', char_any{3,}, 'c'").unwrap();
            let text = "abbcmn";
            let mut instance = Instance::new(text);
            assert_eq!(instance.exec(&process.route, 0), None);
        }

        // lazy repetition - no backtracking
        {
            let process = Process::new("'a', char_any+?, 'c'").unwrap();
            let text = "abbcmn";
            //               "^^^^  "
            let mut instance = Instance::new(text);

            assert_eq!(
                instance.exec(&process.route, 0),
                Some(&vec![MatchRange::new(0, 4)])
            );
        }
    }
}
