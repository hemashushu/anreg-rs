// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::{
    compiler::compile_from_str,
    error::Error,
    image::{Image, MAIN_STATESET_INDEX},
    instance::{Instance, MatchRange, Thread},
    transition::CheckResult,
};

pub struct Process {
    image: Image,
}

impl Process {
    pub fn new(pattern: &str) -> Result<Self, Error> {
        let image = compile_from_str(pattern)?;

        // DEBUG::
        println!("{}", image.get_image_text());

        Ok(Process { image })
    }

    pub fn new_instance<'a, 'b: 'a>(&'a self, chars: &'b [char]) -> Instance {
        Instance::new(&self.image, chars)
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

    pub fn exec_with_values(&mut self, start: usize) -> Option<Vec<MatchGroup>> {
        let capture_names = self.image.get_capture_group_names();
        let chars = self.chars;

        if let Some(match_ranges) = self.exec(start) {
            let match_groups: Vec<MatchGroup> = match_ranges
                .iter()
                .zip(capture_names.iter())
                .map(|(range, name_opt)| MatchGroup {
                    start: range.start,
                    end: range.end,
                    name: if let Some(n) = *name_opt {
                        Some(n.to_owned())
                    } else {
                        None
                    },
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
    let number_of_captures = instance.image.get_number_of_capture_groups();
    let number_of_counters = instance.image.get_number_of_counters();
    let main_thread = Thread::new(start, end, MAIN_STATESET_INDEX);
    instance.threads = vec![main_thread];
    instance.match_ranges = vec![MatchRange::default(); number_of_captures];
    instance.counters = vec![0; number_of_counters];
    instance.anchors = vec![vec![]; number_of_counters];

    while start < end {
        if start_thread(instance, start) {
            return true;
        }

        if instance.image.statesets[MAIN_STATESET_INDEX].fixed_start {
            break;
        }

        // move forward and try again
        start += 1;
    }

    false
}

fn start_thread(instance: &mut Instance, position: usize) -> bool {
    let (stateset_index, entry_state_index, exit_state_index) = {
        let thread = instance.get_current_thread_ref();
        let stateset_index = thread.stateset_index;
        let stateset = &instance.image.statesets[stateset_index];
        (
            stateset_index,
            stateset.start_node_index,
            stateset.end_node_index,
        )
    };

    // DEBUG::
    println!(
        "🔶 THREAD START, state set: {}, entry state: {}, position: {}",
        stateset_index, entry_state_index, position
    );

    // add transitions of the entry state
    instance.append_tasks_by_state(entry_state_index, position);

    while !instance.get_current_thread_ref_mut().stack.is_empty() {
        // take the last task
        let task = instance.get_current_thread_ref_mut().stack.pop().unwrap();

        // get the transition
        let stateset = &instance.image.statesets[stateset_index];
        let state = &stateset.states[task.state_index];
        let transition_node = &state.transitions[task.transition_index];

        // DEBUG::
        println!("> state: {}, position: {}", task.state_index, task.position);

        let position = task.position;
        let transition = &transition_node.transition;
        let target_state_index = transition_node.target_state_index;

        let check_result = transition.check(instance, position);
        match check_result {
            CheckResult::Success(forward) => {
                // DEBUG::
                println!(
                    "  trans: {}, forward: {}, -> state: {}",
                    transition, forward, target_state_index
                );

                if target_state_index == exit_state_index {
                    println!(
                        "  THREAD FINISH, state set: {}, state: {}",
                        stateset_index, exit_state_index
                    );
                    return true;
                }

                instance.append_tasks_by_state(target_state_index, position + forward);
            }
            CheckResult::Failure => {
                // DEBUG::
                println!("  trans: {}, failed", transition);
            }
        }
    }

    // DEBUG::
    println!("  THREAD FAILED, state set: {}", stateset_index);
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
            let chars = "a*1**_***".chars().collect::<Vec<char>>();
            let mut instance = process.new_instance(&chars);
            assert_eq!(instance.exec(0), Some(vec![MatchRange::new(0, 1)]));
        }
    }
}
