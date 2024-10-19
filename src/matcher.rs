// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::process::{MatchGroup, Process};

pub fn match_one(pattern: &str, text: &str) -> Option<Vec<MatchGroup>> {
    let processor = Process::new(pattern).unwrap();
    let mut instance = processor.new_instance(text);
    instance.exec_with_owned_values(0)
}

pub fn match_all(pattern: &str, text: &str) -> Vec<Vec<MatchGroup>> {
    let processor = Process::new(pattern).unwrap();
    let mut instance = processor.new_instance(text);
    let mut start: usize = 0;
    let length = text.as_bytes().len();

    let mut results = vec![];
    while let Some(result) = instance.exec_with_owned_values(start) {
        // set up the next position
        start = result[0].end;
        results.push(result);

        if start >= length {
            break;
        }
    }

    results
}

pub fn test(pattern: &str, text: &str) -> bool {
    let processor = Process::new(pattern).unwrap();
    let mut instance = processor.new_instance(text);
    instance.exec(0).is_some()
}

#[cfg(test)]
mod tests {
    use crate::{matcher::match_all, process::MatchGroup};

    use super::match_one;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_match_one() {
        let r1 = match_one("'a'", "babbaa").unwrap();
        assert_eq!(r1, vec![MatchGroup::new(None, "a".to_owned(), 1, 2)]);

        let r2 = match_one("\"abc\"", "aababcaabc").unwrap();
        assert_eq!(r2, vec![MatchGroup::new(None, "abc".to_owned(), 3, 6)]);
    }

    #[test]
    fn test_match_all() {
        let r1 = match_all("'a'", "babbaa");
        assert_eq!(
            r1,
            vec![
                vec![MatchGroup::new(None, "a".to_owned(), 1, 2)],
                vec![MatchGroup::new(None, "a".to_owned(), 4, 5)],
                vec![MatchGroup::new(None, "a".to_owned(), 5, 6)],
            ]
        );

        let r2 = match_all("\"abc\"", "aababcaabc");
        assert_eq!(
            r2,
            vec![
                vec![MatchGroup::new(None, "abc".to_owned(), 3, 6)],
                vec![MatchGroup::new(None, "abc".to_owned(), 7, 10)]
            ]
        );
    }

    #[test]
    fn test_test() {
        // todo
    }
}
