// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

mod ast;
mod charposition;
mod commentcleaner;
mod compiler;
mod context;
mod error;
mod errorprinter;
mod lexer;
mod location;
mod macroexpander;
mod normalizer;
mod parser;
mod peekableiter;
mod state;
mod token;
mod transition;

pub use compiler::compile_from_str;