// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::{compiler::compile_from_str, context::Context, error::Error, image::Image};

pub struct Process {
    image: Image,
}

impl Process {
    pub fn new(pattern: &str) -> Result<Self, Error> {
        let image = compile_from_str(pattern)?;
        Ok(Process { image })
    }

    pub fn new_instance<'a, 'b: 'a>(&'a self, chars: &'b [char]) -> Instance {
        let number_of_captures = self.image.get_number_of_captures();
        let number_of_counters = self.image.get_number_of_counters();
        Instance::new(
            &self.image,
            Context::new(chars, number_of_captures, number_of_counters),
        )
    }
}

pub struct Instance<'a, 'b> {
    image: &'a Image,
    context: Context<'b>,
}

impl<'a, 'b> Instance<'a, 'b> {
    pub fn new(image: &'a Image, context: Context<'b>) -> Self {
        Instance { image, context }
    }

    pub fn exec(&mut self, start: usize) -> Option<Vec<MatchRange>> {
        self.context.reset(start);

        // do matching

        let match_ranges: Vec<MatchRange> = self
            .context
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
                    value: Self::get_sub_string(&self.context.chars, range.start, range.end),
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
}

pub struct MatchRange {
    pub start: usize, // position included
    pub end: usize,   // position excluded
}

pub struct MatchGroup {
    pub name: Option<String>,
    pub value: String,
    pub start: usize, // position included
    pub end: usize,   // position excluded
}

pub struct MatchResult {
    pub groups: Vec<MatchGroup>,
}
