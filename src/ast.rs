// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

#[derive(Debug)]
pub struct CharRange {
    pub start: char,
    pub end_included: char,
}

#[derive(Debug)]
pub enum CharSetElement {
    Char(char),
    CharRange(CharRange),
    Preset(String),
}

#[derive(Debug)]
pub struct CharSet {
    items: Vec<CharSetElement>,
}

#[derive(Debug)]
pub enum Literal {
    Char(char),
    String(String),
    CharSet(CharSet),
    PresetCharSet(String),
}

#[derive(Debug)]
pub enum Arg {
    Expression(Expression),
    Number(u32),
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub expression: Box<Expression>,
    pub args: Vec<Arg>,
}

#[derive(Debug)]
pub enum Expression {
    Literal(Literal),
    Identifier(String),
    Group(Vec<Expression>),
    Alternation(Box<Expression>, Box<Expression>),
    Function(Box<Function>),
}

#[derive(Debug)]
pub struct Definition {
    pub name: String,
    pub expression: Box<Expression>
}

#[derive(Debug)]
pub struct Program {
    pub definitions: Vec<Expression>,
    pub expressions: Vec<Expression>,
}
