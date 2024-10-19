// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::{
    compiler::compile_from_str,
    error::Error,
    instance::{Instance, MatchRange, Thread},
    route::{Route, MAIN_LINE_INDEX},
    transition::CheckResult,
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

    pub fn new_instance<'a, 'b: 'a>(&'a self, chars: &'b [char]) -> Instance {
        Instance::new(&self.route, chars)
    }
}

impl<'a, 'b> Instance<'a, 'b> {
    pub fn exec(&mut self, start: usize) -> Option<Vec<MatchRange>> {
        // do matching
        if !start_main_thread(self, start, self.chars.len()) {
            return None;
        }

        Some(
            self.match_ranges
                .iter()
                .map(|item| item.to_owned())
                .collect(),
        )
    }

    pub fn exec_with_groups(&mut self, start: usize) -> Option<Vec<MatchGroup>> {
        let capture_names = self.route.get_capture_group_names();
        let chars = self.chars;

        if let Some(match_ranges) = self.exec(start) {
            let match_groups: Vec<MatchGroup> = match_ranges
                .iter()
                .zip(capture_names.iter())
                .map(|(range, name_opt)| MatchGroup {
                    start: range.start,
                    end: range.end,
                    name: (*name_opt).map(|n| n.to_owned()),
                    value: get_sub_string(chars, range.start, range.end),
                })
                .collect();

            Some(match_groups)
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct MatchGroup {
    pub name: Option<String>,
    pub value: String,
    pub start: usize, // position included
    pub end: usize,   // position excluded
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

pub fn get_group<'a>(groups: &'a [MatchGroup], name: &str) -> Option<&'a MatchGroup> {
    groups.iter().find(|item| match &item.name {
        Some(s) => s == name,
        None => false,
    })
}

fn get_sub_string(chars: &[char], start: usize, end: usize) -> String {
    /*
     * convert Vec<char> into String:
     * `let s:String = chars.iter().collect()`
     * or
     * `let s = String::from_iter(&chars)`
     */
    let slice = &chars[start..end];
    String::from_iter(slice)
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
        start += 1;
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
    use crate::instance::MatchRange;

    use super::Process;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_process_char() {
        let process = Process::new("'a'").unwrap();

        {
            let chars: Vec<char> = "babbaa".chars().collect();
            let mut instance = process.new_instance(&chars);

            // match 1
            assert_eq!(instance.exec(0), Some(vec![MatchRange::new(1, 2)]));
            assert_eq!(instance.exec(1), Some(vec![MatchRange::new(1, 2)]));

            // match 2
            assert_eq!(instance.exec(2), Some(vec![MatchRange::new(4, 5)]));
            assert_eq!(instance.exec(3), Some(vec![MatchRange::new(4, 5)]));
            assert_eq!(instance.exec(4), Some(vec![MatchRange::new(4, 5)]));

            // match 3
            assert_eq!(instance.exec(5), Some(vec![MatchRange::new(5, 6)]));

            // exceed the length of chars
            assert_eq!(instance.exec(6), None);
        }

        {
            let chars: Vec<char> = "abaabb".chars().collect();
            let mut instance = process.new_instance(&chars);

            assert_eq!(instance.exec(0), Some(vec![MatchRange::new(0, 1)]));

            assert_eq!(instance.exec(1), Some(vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(2), Some(vec![MatchRange::new(2, 3)]));

            assert_eq!(instance.exec(3), Some(vec![MatchRange::new(3, 4)]));

            assert_eq!(instance.exec(4), None);
            assert_eq!(instance.exec(5), None);

            // exceed the length of chars
            assert_eq!(instance.exec(6), None);
        }

        {
            let chars: Vec<char> = "xyz".chars().collect();
            let mut instance = process.new_instance(&chars);

            assert_eq!(instance.exec(0), None);
            assert_eq!(instance.exec(1), None);
            assert_eq!(instance.exec(2), None);

            // exceed the length of chars
            assert_eq!(instance.exec(3), None);
        }
    }

    #[test]
    fn test_process_string() {
        let process = Process::new("\"abc\"").unwrap();

        {
            let chars: Vec<char> = "ababcbcabc".chars().collect();
            let mut instance = process.new_instance(&chars);

            assert_eq!(instance.exec(0), Some(vec![MatchRange::new(2, 5)]));
            assert_eq!(instance.exec(1), Some(vec![MatchRange::new(2, 5)]));
            assert_eq!(instance.exec(2), Some(vec![MatchRange::new(2, 5)]));

            assert_eq!(instance.exec(3), Some(vec![MatchRange::new(7, 10)]));
            assert_eq!(instance.exec(5), Some(vec![MatchRange::new(7, 10)]));
            assert_eq!(instance.exec(7), Some(vec![MatchRange::new(7, 10)]));

            assert_eq!(instance.exec(8), None);

            // exceed the length of chars
            assert_eq!(instance.exec(10), None);
        }

        {
            let chars: Vec<char> = "uvwxyz".chars().collect();
            let mut instance = process.new_instance(&chars);

            assert_eq!(instance.exec(0), None);
            assert_eq!(instance.exec(1), None);
            assert_eq!(instance.exec(5), None);

            assert_eq!(instance.exec(6), None);
        }
    }

    #[test]
    fn test_preset_charset() {
        {
            let process = Process::new("char_word").unwrap();
            let chars = "a*1**_ **".chars().collect::<Vec<char>>();
            //                      ^ ^  ^
            let mut instance = process.new_instance(&chars);
            assert_eq!(instance.exec(0), Some(vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        {
            let process = Process::new("char_not_word").unwrap();
            let chars = "!a@12 bc_".chars().collect::<Vec<char>>();
            //                      v v  v
            let mut instance = process.new_instance(&chars);
            assert_eq!(instance.exec(0), Some(vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        {
            let process = Process::new("char_digit").unwrap();
            let chars = "1a2b_3de*".chars().collect::<Vec<char>>();
            //                      ^ ^  ^
            let mut instance = process.new_instance(&chars);
            assert_eq!(instance.exec(0), Some(vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        {
            let process = Process::new("char_not_digit").unwrap();
            let chars = "a1_23 456".chars().collect::<Vec<char>>();
            //                      v v  v
            let mut instance = process.new_instance(&chars);
            assert_eq!(instance.exec(0), Some(vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        {
            let process = Process::new("char_space").unwrap();
            let chars = " 1\tab\n_*!".chars().collect::<Vec<char>>();
            //                      ^ ^^  ^^
            //                      012 345 678
            let mut instance = process.new_instance(&chars);
            assert_eq!(instance.exec(0), Some(vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        {
            let process = Process::new("char_not_space").unwrap();
            let chars = "a\t1\n *   ".chars().collect::<Vec<char>>();
            //                      v  v   v
            //                      01 23 45678
            let mut instance = process.new_instance(&chars);
            assert_eq!(instance.exec(0), Some(vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }
    }

    #[test]
    fn test_charset() {
        {
            let process = Process::new("['a','b','c']").unwrap();
            let chars = "adbefcghi".chars().collect::<Vec<char>>();
            //                      ^ ^  ^
            let mut instance = process.new_instance(&chars);
            assert_eq!(instance.exec(0), Some(vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        // negative
        {
            let process = Process::new("!['a','b','c']").unwrap();
            let chars = "xa1bb*ccc".chars().collect::<Vec<char>>();
            //                      v v  v
            let mut instance = process.new_instance(&chars);
            assert_eq!(instance.exec(0), Some(vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        {
            let process = Process::new("['a'..'c']").unwrap();
            let chars = "adbefcghi".chars().collect::<Vec<char>>();
            //                      ^ ^  ^
            let mut instance = process.new_instance(&chars);
            assert_eq!(instance.exec(0), Some(vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        // negative
        {
            let process = Process::new("!['a'..'c']").unwrap();
            let chars = "xa1bb*ccc".chars().collect::<Vec<char>>();
            //                      v v  v
            let mut instance = process.new_instance(&chars);
            assert_eq!(instance.exec(0), Some(vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        {
            let process = Process::new("['a'..'f', '0'..'9']").unwrap();
            let chars = "am1npfq*_".chars().collect::<Vec<char>>();
            //                      ^ ^  ^
            let mut instance = process.new_instance(&chars);
            assert_eq!(instance.exec(0), Some(vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        // negative
        {
            let process = Process::new("!['a'..'f', '0'..'9']").unwrap();
            let chars = "man12*def".chars().collect::<Vec<char>>();
            //                      v v  v
            let mut instance = process.new_instance(&chars);
            assert_eq!(instance.exec(0), Some(vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        // combine with preset
        {
            let process = Process::new("['a'..'f', char_digit]").unwrap();
            let chars = "am1npfq*_".chars().collect::<Vec<char>>();
            //                      ^ ^  ^
            let mut instance = process.new_instance(&chars);
            assert_eq!(instance.exec(0), Some(vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        // negative
        {
            let process = Process::new("!['a'..'f', char_digit]").unwrap();
            let chars = "man12*def".chars().collect::<Vec<char>>();
            //                      v v  v
            let mut instance = process.new_instance(&chars);
            assert_eq!(instance.exec(0), Some(vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        // nested
        {
            let process = Process::new("[['a','b','c','d'..'f'], ['0'..'8'], '9']").unwrap();
            let chars = "am1npfq*_".chars().collect::<Vec<char>>();
            //                      ^ ^  ^
            let mut instance = process.new_instance(&chars);
            assert_eq!(instance.exec(0), Some(vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }

        // negative
        {
            let process = Process::new("![['a','b','c','d'..'f'], ['0'..'8'], '9']").unwrap();
            let chars = "man12*def".chars().collect::<Vec<char>>();
            //                      v v  v
            let mut instance = process.new_instance(&chars);
            assert_eq!(instance.exec(0), Some(vec![MatchRange::new(0, 1)]));
            assert_eq!(instance.exec(1), Some(vec![MatchRange::new(2, 3)]));
            assert_eq!(instance.exec(3), Some(vec![MatchRange::new(5, 6)]));
            assert_eq!(instance.exec(6), None);
        }
    }
}
