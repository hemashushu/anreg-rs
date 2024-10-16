// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::{compiler::compile_from_str, context::Context, error::Error, image::Image};

pub struct Processor {
    image: Image,
}

impl Processor {
    pub fn new(pattern: &str) -> Result<Self, Error> {
        let image = compile_from_str(pattern)?;
        Ok(Processor { image })
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

    pub fn exec(&mut self, start: usize) -> Option<Vec<CaptureRange>> {
        // let capture_ranges : Vec<CaptureRange> =  capture_positions
        // .iter()
        // .map(|i| CaptureRange {
        //     start: i.start,
        //     end: i.end_included + 1,
        // })
        // .collect();

        todo!()
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct CaptureRange {
    pub start: usize, // position included
    pub end: usize,   // position excluded
}
