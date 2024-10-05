// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

#[derive(Debug, PartialEq)]
pub struct Program {
    pub definitions: Vec<Definition>,
    pub expressions: Vec<Expression>,
}

#[derive(Debug, PartialEq)]
pub struct Definition {
    pub expression: Box<Expression>,
    pub identifier: String,
}

#[derive(Debug, PartialEq)]
pub enum Expression {
    Literal(Literal),
    Identifier(String),
    Group(Vec<Expression>),
    FunctionCall(Box<FunctionCall>),
    Alternation(Box<Expression>, Box<Expression>),
}

#[derive(Debug, PartialEq)]
pub struct FunctionCall {
    pub name: FunctionName,
    pub expression: Box<Expression>,
    pub args: Vec<FunctionCallArg>,
}

#[derive(Debug, PartialEq)]
pub enum FunctionCallArg {
    Number(u32),
    Identifier(String),
}

#[derive(Debug, PartialEq)]
pub enum Literal {
    Char(char),
    String(String),
    CharSet(CharSet),
    PresetCharSet(String),
    Symbol(String),
}

#[derive(Debug, PartialEq)]
pub struct CharSet {
    pub negative: bool,
    pub elements: Vec<CharSetElement>,
}

#[derive(Debug, PartialEq)]
pub enum CharSetElement {
    Char(char),
    CharRange(CharRange),
    PresetCharSet(String),
    Symbol(String),
}

#[derive(Debug, PartialEq)]
pub struct CharRange {
    pub start: char,
    pub end_included: char,
}

#[derive(Debug, PartialEq)]
pub enum FunctionName {
    // Greedy quantifier
    Optional,
    OneOrMore,
    ZeroOrMore,
    Repeat,
    RepeatRange,
    AtLeast,

    // Lazy quantifier
    OptionalLazy,
    OneOrMoreLazy,
    ZeroOrMoreLazy,
    RepeatLazy,
    RepeatRangeLazy,
    AtLeastLazy,

    // Assertions
    IsBefore,    // lookahead
    IsAfter,     // lookbehind
    IsNotBefore, // negative lookahead
    IsNotAfter,  // negative lookbehind

    // Others
    Name,
}
