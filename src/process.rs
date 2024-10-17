// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::{
    compiler::compile_from_str,
    error::Error,
    image::{Image, MAIN_STATESET_INDEX},
    instance::{CapturePosition, Instance, Thread},
    transition::{CheckResult, Transition},
};

pub struct Process {
    image: Image,
}

impl Process {
    pub fn new(pattern: &str) -> Result<Self, Error> {
        let image = compile_from_str(pattern)?;
        Ok(Process { image })
    }

    pub fn new_instance<'a, 'b: 'a>(&'a self, chars: &'b [char]) -> Instance {
        Instance::new(&self.image, chars) // , number_of_captures, number_of_counters)
    }
}

impl<'a, 'b> Instance<'a, 'b> {
    pub fn exec(&mut self, start: usize) -> Option<Vec<MatchRange>> {
        // do matching
        if !start_main_thread(self, start, self.chars.len()) {
            return None;
        }

        let match_ranges: Vec<MatchRange> = self
            .capture_positions
            .iter()
            .map(|i| MatchRange {
                start: i.start,
                end: i.end_included + 1,
            })
            .collect();

        Some(match_ranges)
    }

    pub fn exec_with_result(&mut self, start: usize) -> Option<MatchResult> {
        if let Some(match_ranges) = self.exec(start) {
            let match_groups: Vec<MatchGroup> = match_ranges
                .iter()
                .zip(self.image.get_capture_names().iter())
                .map(|(range, name_opt)| MatchGroup {
                    start: range.start,
                    end: range.end,
                    name: if let Some(n) = *name_opt {
                        Some(n.to_owned())
                    } else {
                        None
                    },
                    value: get_sub_string(&self.chars, range.start, range.end),
                })
                .collect();

            let match_result = MatchResult {
                groups: match_groups,
            };
            Some(match_result)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct MatchRange {
    pub start: usize, // position included
    pub end: usize,   // position excluded
}

#[derive(Debug)]
pub struct MatchGroup {
    pub name: Option<String>,
    pub value: String,
    pub start: usize, // position included
    pub end: usize,   // position excluded
}

#[derive(Debug)]
pub struct MatchResult {
    pub groups: Vec<MatchGroup>,
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
    // DEBUG
    println!("{}", instance.image.get_image_text());

    // allocate the vector of 'capture positions' and 'repetition counters'
    let number_of_captures = instance.image.get_number_of_captures();
    let number_of_counters = instance.image.get_number_of_counters();
    let main_thread = Thread::new(start, end, MAIN_STATESET_INDEX);
    instance.threads = vec![main_thread];
    instance.capture_positions = vec![CapturePosition::default(); number_of_captures];
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

    println!("THREAD START----------------- position: {}", position);

    // add transitions of the entry state
    instance.append_tasks_by_state(entry_state_index, position);

    while !instance.get_current_thread_ref_mut().stack.is_empty() {
        // take the last task
        let task = instance.get_current_thread_ref_mut().stack.pop().unwrap();

        // get the transition
        let stateset = &instance.image.statesets[stateset_index];
        let state = &stateset.states[task.state_index];
        let transition_node = &state.transitions[task.transition_index];

        println!("> state: {}, position: {}", task.state_index, task.position);

        let position = task.position;
        let transition = &transition_node.transition;
        let target_state_index = transition_node.target_state_index;

        let check_result = transition.check(instance, position);
        match  check_result {
            CheckResult::Success(forward) => {
                println!("  trans: {}, forward: {}, --> state: {}", transition, forward, target_state_index);

                if target_state_index == exit_state_index {
                    println!("FINISH! < {}", exit_state_index);
                    return true;
                }

                instance.append_tasks_by_state(target_state_index, position + forward);
            },
            CheckResult::Failure => {
                println!("  trans: {}, X", transition);
            },
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::Process;

    #[test]
    fn test_process_raw_char() {
        let process = Process::new("'a'").unwrap();
        let chars: Vec<char> = "babbaabbb".chars().collect();
        let mut instance = process.new_instance(&chars);
        let ranges_opt0 = instance.exec(0);
        println!("{:?}", ranges_opt0)
    }
}
