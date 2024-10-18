// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::{
    error::Error,
    process::{MatchGroup, Process},
};

pub fn match_one(pattern: &str, text: &str) -> Result<Option<Vec<MatchGroup>>, Error> {
    let processor = Process::new(pattern)?;
    let chars: Vec<char> = text.chars().collect();
    let chars_ref = &chars;
    let mut instance = processor.new_instance(chars_ref);

    Ok(instance.exec_with_groups(0))
}

pub fn match_all(pattern: &str, text: &str) -> Result<Vec<Vec<MatchGroup>>, Error> {
    let processor = Process::new(pattern)?;
    let chars: Vec<char> = text.chars().collect();
    let mut instance = processor.new_instance(&chars);
    let mut start: usize = 0;

    let mut results = vec![];
    while let Some(result) = instance.exec_with_groups(start) {
        // set up the next position
        start = result[0].end;
        results.push(result);

        if start >= chars.len() {
            break;
        }
    }

    Ok(results)
}

pub fn test(pattern: &str, text: &str) -> Result<bool, Error> {
    let processor = Process::new(pattern)?;
    let chars: Vec<char> = text.chars().collect();
    let mut instance = processor.new_instance(&chars);

    Ok(instance.exec(0).is_some())
}

#[cfg(test)]
mod tests {
    use crate::{matcher::match_all, process::MatchGroup};

    use super::match_one;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_match_one() {
        let r1 = match_one("'a'", "babbaa").unwrap();
        assert_eq!(
            r1.unwrap(),
            vec![MatchGroup::new(None, "a".to_owned(), 1, 2)]
        );

        let r2 = match_one("\"abc\"", "aababcaabc").unwrap();
        assert_eq!(
            r2.unwrap(),
            vec![MatchGroup::new(None, "abc".to_owned(), 3, 6)]
        );
    }

    #[test]
    fn test_match_all() {
        let r1 = match_all("'a'", "babbaa").unwrap();
        assert_eq!(
            r1,
            vec![
                vec![MatchGroup::new(None, "a".to_owned(), 1, 2)],
                vec![MatchGroup::new(None, "a".to_owned(), 4, 5)],
                vec![MatchGroup::new(None, "a".to_owned(), 5, 6)],
            ]
        );

        let r2 = match_all("\"abc\"", "aababcaabc").unwrap();
        assert_eq!(
            r2,
            vec![
                vec![MatchGroup::new(None, "abc".to_owned(), 3, 6)],
                vec![MatchGroup::new(None, "abc".to_owned(), 7, 10)]
            ]
        );
    }
}
