// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::{
    error::Error,
    process::{MatchResult, Process},
};

pub fn match_one(text: &str, pattern: &str) -> Result<Option<MatchResult>, Error> {
    let processor = Process::new(pattern)?;
    let chars: Vec<char> = text.chars().collect();
    let chars_ref = &chars;
    let mut instance = processor.new_instance(chars_ref);

    Ok(instance.exec_with_result(0))
}

pub fn match_all(text: &str, pattern: &str) -> Result<Vec<MatchResult>, Error> {
    let processor = Process::new(pattern)?;
    let chars: Vec<char> = text.chars().collect();
    let mut instance = processor.new_instance(&chars);
    let mut start: usize = 0;

    let mut results = vec![];
    while let Some(result) = instance.exec_with_result(start) {
        start = result.groups[0].end; // next start position
        results.push(result);
        if start >= chars.len() {
            break;
        }
    }

    Ok(results)
}

pub fn test(text: &str, pattern: &str) -> Result<bool, Error> {
    let processor = Process::new(pattern)?;
    let chars: Vec<char> = text.chars().collect();
    let mut instance = processor.new_instance(&chars);

    Ok(instance.exec(0).is_some())
}
