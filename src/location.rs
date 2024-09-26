// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Location {
    pub unit: usize,   // the index of source file
    pub index: usize,  // character index
    pub line: usize,   // line index
    pub column: usize, // column index
    pub length: usize, // text length, 0 for position
}

impl Location {
    pub fn new_position(unit: usize, index: usize, line: usize, column: usize) -> Self {
        Self {
            unit,
            index,
            line,
            column,
            length: 0,
        }
    }

    pub fn new_range(unit: usize, index: usize, line: usize, column: usize, length: usize) -> Self {
        Self {
            unit,
            index,
            line,
            column,
            length,
        }
    }

    /// Build Range with Position and length
    pub fn from_position_and_length(position: &Location, length: usize) -> Self {
        Self::new_range(
            position.unit,
            position.index,
            position.line,
            position.column,
            length,
        )
    }

    /// Convert two Positions to Range
    pub fn from_position_pair(position_start: &Location, position_end: &Location) -> Self {
        Self::new_range(
            position_start.unit,
            position_start.index,
            position_start.line,
            position_start.column,
            position_end.index - position_start.index,
        )
    }

    /// Convert two Positions to Range
    pub fn from_position_pair_with_end_included(
        position_start: &Location,
        position_end_included: &Location,
    ) -> Self {
        Self::new_range(
            position_start.unit,
            position_start.index,
            position_start.line,
            position_start.column,
            position_end_included.index - position_start.index + 1,
        )
    }

    /// Combine two ranges into a new range
    pub fn from_range_pair(range_start: &Location, range_end: &Location) -> Self {
        Self::new_range(
            range_start.unit,
            range_start.index,
            range_start.line,
            range_start.column,
            range_end.index - range_start.index + range_end.length,
        )
    }

    /// Convert Range to Position
    pub fn from_range_start(range: &Location) -> Self {
        Self::new_position(range.unit, range.index, range.line, range.column)
    }

    /// Convert Range to Position
    pub fn from_range_end(range: &Location) -> Self {
        let index = range.index + range.length;
        let column = range.column + range.length;
        Self::new_position(range.unit, index, range.line, column)
    }

    pub fn move_position_forward(&self) -> Self {
        Self {
            index: self.index + 1,
            column: self.column + 1,
            ..*self
        }
    }

    //     pub fn move_position_forward_by_line(&self) -> Self {
    //         Self {
    //             index: self.index + 1,
    //             line: self.line + 1,
    //             column: 0,
    //             ..*self
    //         }
    //     }
}
