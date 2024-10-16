// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::{error::Error, processor::Processor};

pub struct MatchResult {
    pub groups: Vec<MatchGroup>,
}

pub struct MatchGroup {
    pub text: String,
    pub start: usize, // position included
    pub end: usize,   // position excluded
}

pub fn match_one(text: &str, pattern: &str) -> Result<Option<MatchResult>, Error> {
    let processor = Processor::new(pattern)?;
    let chars: Vec<char> = text.chars().collect();
    let chars_ref = &chars;
    let mut instance = processor.new_instance(chars_ref);

    if let Some(ranges) = instance.exec(0) {
        let groups: Vec<MatchGroup> = ranges
            .iter()
            .map(|item| {
                let sub_string = get_sub_string(chars_ref, item.start, item.end);
                MatchGroup {
                    text: sub_string,
                    start: item.start,
                    end: item.end,
                }
            })
            .collect();

        Ok(Some(MatchResult { groups }))
    } else {
        Ok(None)
    }
}

pub fn match_all(text: &str, pattern: &str) -> Result<Vec<MatchResult>, Error> {
    let mut results = vec![];

    let processor = Processor::new(pattern)?;
    let chars: Vec<char> = text.chars().collect();
    let chars_ref = &chars;
    let mut instance = processor.new_instance(chars_ref);
    let mut start: usize = 0;

    while let Some(ranges) = instance.exec(start) {
        let groups: Vec<MatchGroup> = ranges
            .iter()
            .map(|item| {
                let sub_string = get_sub_string(chars_ref, item.start, item.end);
                MatchGroup {
                    text: sub_string,
                    start: item.start,
                    end: item.end,
                }
            })
            .collect();

        start = groups[0].end;

        results.push(MatchResult { groups });

        if start >= chars_ref.len() {
            break;
        }
    }

    Ok(results)
}

pub fn test(text: &str, pattern: &str) -> Result<bool, Error> {
    let processor = Processor::new(pattern)?;
    let chars: Vec<char> = text.chars().collect();
    let chars_ref = &chars;
    let mut instance = processor.new_instance(chars_ref);

    if let Some(ranges) = instance.exec(0) {
        Ok(true)
    } else {
        Ok(false)
    }
}

fn get_sub_string(text: &[char], start: usize, end: usize) -> String {
    /*
     * convert Vec<char> into String:
     * `let s:String = chars.iter().collect()`
     * or
     * `let s = String::from_iter(&self.chars)`
     */
    let s = &text[start..end];
    String::from_iter(s)
}
