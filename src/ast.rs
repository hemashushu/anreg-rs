// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::fmt::Display;

#[derive(Debug, PartialEq)]
pub struct Program {
    pub expressions: Vec<Expression>,
}

#[derive(Debug, PartialEq)]
pub enum Expression {
    Literal(Literal),
    Identifier(String),
    Assertion(String),

    /**
     * the _group_ of ANREG is different from the _group_ of
     * oridinary regular expressions.
     * ANREG's group is just a series of patterns, which will not
     * be captured unless enclosed by the function 'name' or 'capture'.
     * e.g.
     * ANREG group `('a', 'b', char_word+)` is equivalent to oridinary regex `ab\w+`
     */
    Group(Vec<Expression>),

    FunctionCall(Box<FunctionCall>),

    /**
     * Disjunction
     * https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Regular_expressions/Disjunction
     */
    Or(Box<Expression>, Box<Expression>),
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
    Expression(Box<Expression>),
}

#[derive(Debug, PartialEq)]
pub enum Literal {
    Char(char),
    String(String),
    Special(String),
    CharSet(CharSet),
    PresetCharSet(String),
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
    CharSet(Box<CharSet>)
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

    // Assertions ("判定")
    IsBefore,    // lookahead
    IsAfter,     // lookbehind
    IsNotBefore, // negative lookahead
    IsNotAfter,  // negative lookbehind

    // Capture
    Name,
    Capture,
}

impl Display for FunctionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionName::Optional => f.write_str("optional"),
            FunctionName::OneOrMore => f.write_str("one_or_more"),
            FunctionName::ZeroOrMore => f.write_str("zero_or_more"),
            FunctionName::Repeat => f.write_str("repeat"),
            FunctionName::RepeatRange => f.write_str("repeat_range"),
            FunctionName::AtLeast => f.write_str("at_least"),
            FunctionName::OptionalLazy => f.write_str("optional_lazy"),
            FunctionName::OneOrMoreLazy => f.write_str("one_or_more_lazy"),
            FunctionName::ZeroOrMoreLazy => f.write_str("zero_or_more_lazy"),
            FunctionName::RepeatLazy => f.write_str("repeat_lazy"),
            FunctionName::RepeatRangeLazy => f.write_str("repeat_range_lazy"),
            FunctionName::AtLeastLazy => f.write_str("at_least_lazy"),
            FunctionName::IsBefore => f.write_str("is_before"),
            FunctionName::IsAfter => f.write_str("is_after"),
            FunctionName::IsNotBefore => f.write_str("is_not_before"),
            FunctionName::IsNotAfter => f.write_str("is_not_after"),
            FunctionName::Name => f.write_str("name"),
            FunctionName::Capture => f.write_str("capture"),
        }
    }
}

impl Display for CharRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}'..'{}'", self.start, self.end_included)
    }
}

impl Display for CharSetElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CharSetElement::Char(c) => write!(f, "'{}'", c),
            CharSetElement::CharRange(c) => write!(f, "{}", c),
            CharSetElement::PresetCharSet(p) => f.write_str(p),
            CharSetElement::CharSet(c) => write!(f, "{}", c),
        }
    }
}

impl Display for CharSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: Vec<String> = self.elements.iter().map(|e| e.to_string()).collect();
        if self.negative {
            write!(f, "![{}]", s.join(", "))
        } else {
            write!(f, "[{}]", s.join(", "))
        }
    }
}

impl Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Literal::Char(c) => write!(f, "'{}'", c),
            Literal::String(s) => write!(f, "\"{}\"", s),
            Literal::CharSet(c) => write!(f, "{}", c),
            Literal::PresetCharSet(p) => f.write_str(p),
            Literal::Special(s) => write!(f, "{}", s),
        }
    }
}

impl Display for FunctionCallArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionCallArg::Number(n) => write!(f, "{}", n),
            FunctionCallArg::Identifier(i) => write!(f, "{}", i),
            FunctionCallArg::Expression(e) => write!(f, "{}", e),
        }
    }
}

impl Display for FunctionCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.args.is_empty() {
            write!(f, "{}({})", self.name, self.expression)
        } else {
            let s: Vec<String> = self.args.iter().map(|e| e.to_string()).collect();
            write!(f, "{}({}, {})", self.name, self.expression, s.join(", "))
        }
    }
}

impl Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expression::Literal(l) => write!(f, "{}", l),
            Expression::Identifier(id) => f.write_str(id),
            Expression::Assertion(s) => f.write_str(s),
            Expression::Group(g) => {
                let s: Vec<String> = g.iter().map(|e| e.to_string()).collect();
                write!(f, "({})", s.join(", "))
            }
            Expression::FunctionCall(fc) => write!(f, "{}", fc),
            Expression::Or(left, right) => write!(f, "{} || {}", left, right),
        }
    }
}

impl Display for Program {
    // for debug
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut exp_strings: Vec<String> = vec![];
        for (idx, expression) in self.expressions.iter().enumerate() {
            match expression {
                Expression::FunctionCall(function_call) => {
                    if idx != 0 {
                        // replace the last ',' with '\n'
                        exp_strings.pop();
                        exp_strings.push("\n".to_owned());
                    }
                    exp_strings.push(function_call.to_string());
                    exp_strings.push("\n".to_owned());
                }
                _ => {
                    exp_strings.push(expression.to_string());
                    exp_strings.push(", ".to_owned());
                }
            }
        }

        if !exp_strings.is_empty() {
            exp_strings.pop(); // remove the last ',' or '\n'
            write!(f, "{}", exp_strings.join(""))
        } else {
            f.write_str("")
        }
    }
}
