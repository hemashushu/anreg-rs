// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

use std::fmt::Display;

use crate::ast::{
    BackReference, CharRange, CharSet, CharSetElement, Expression, FunctionArgument, FunctionCall,
    FunctionName, Literal, PresetCharSetName, Program,
};

impl Display for PresetCharSetName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name_str = match self {
            PresetCharSetName::CharWord => "char_word",
            PresetCharSetName::CharNotWord => "char_not_word",
            PresetCharSetName::CharDigit => "char_digit",
            PresetCharSetName::CharNotDigit => "char_not_digit",
            PresetCharSetName::CharSpace => "char_space",
            PresetCharSetName::CharNotSpace => "char_not_space",
        };
        f.write_str(name_str)
    }
}

impl Display for FunctionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionName::Optional => f.write_str("optional"),
            FunctionName::OneOrMore => f.write_str("one_or_more"),
            FunctionName::ZeroOrMore => f.write_str("zero_or_more"),
            FunctionName::Repeat => f.write_str("repeat"),
            FunctionName::RepeatRange => f.write_str("repeat_range"),
            FunctionName::RepeatFrom => f.write_str("repeat_from"),
            FunctionName::LazyOptional => f.write_str("lazy_optional"),
            FunctionName::LazyOneOrMore => f.write_str("lazy_one_or_more"),
            FunctionName::LazyZeroOrMore => f.write_str("lazy_zero_or_more"),
            FunctionName::LazyRepeat => f.write_str("lazy_repeat"),
            FunctionName::LazyRepeatRange => f.write_str("lazy_repeat_range"),
            FunctionName::LazyRepeatFrom => f.write_str("lazy_repeat_from"),
            FunctionName::IsBefore => f.write_str("is_before"),
            FunctionName::IsAfter => f.write_str("is_after"),
            FunctionName::IsNotBefore => f.write_str("is_not_before"),
            FunctionName::IsNotAfter => f.write_str("is_not_after"),
            FunctionName::IsStart => f.write_str("is_start"),
            FunctionName::IsEnd => f.write_str("is_end"),
            FunctionName::IsBound => f.write_str("is_bound"),
            FunctionName::IsNotBound => f.write_str("is_not_bound"),
            FunctionName::Index => f.write_str("index"),
            FunctionName::Name => f.write_str("name"),
        }
    }
}

impl Display for CharRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}'..'{}'", self.start, self.end_inclusive)
    }
}

impl Display for CharSetElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CharSetElement::Char(c) => write!(f, "'{}'", c),
            CharSetElement::CharRange(c) => write!(f, "{}", c),
            CharSetElement::PresetCharSet(p) => write!(f, "{}", p),
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
            Literal::AnyChar => write!(f, "char_any"),
            Literal::Char(c) => write!(f, "'{}'", escape_char(*c)),
            Literal::String(s) => write!(f, "\"{}\"", escape_string(s)),
            Literal::CharSet(c) => write!(f, "{}", c),
            Literal::PresetCharSet(p) => write!(f, "{}", p),
        }
    }
}

fn escape_char(c: char) -> String {
    match c {
        '\n' => "\\n".to_string(),
        '\r' => "\\r".to_string(),
        '\t' => "\\t".to_string(),
        '\\' => "\\\\".to_string(),
        '\'' => "\\\'".to_string(),
        '\"' => "\\\"".to_string(),
        _ => c.to_string(),
    }
}

fn escape_string(s: &str) -> String {
    s.chars().map(escape_char).collect()
}

impl Display for FunctionArgument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionArgument::Number(i) => write!(f, "{}", i),
            FunctionArgument::Identifier(name) => write!(f, "{}", name),
            FunctionArgument::Expression(e) => write!(f, "{}", e),
        }
    }
}

impl Display for FunctionCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: Vec<String> = self.args.iter().map(|e| e.to_string()).collect();
        write!(f, "{}({})", self.name, s.join(", "))
    }
}

impl Display for BackReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackReference::Index(index) => write!(f, "^{}", index),
            BackReference::Name(name) => f.write_str(name),
        }
    }
}

impl Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expression::Literal(e) => write!(f, "{}", e),
            Expression::BackReference(e) => write!(f, "{}", e),
            Expression::Group(expressions) => {
                let lines: Vec<String> = expressions.iter().map(|e| e.to_string()).collect();
                write!(f, "({})", lines.join(", "))
            }
            Expression::FunctionCall(e) => write!(f, "{}", e),
            Expression::Or(left, right) => {
                if matches!(left.as_ref(), Expression::Or(_, _)) {
                    write!(f, "({})", left)?;
                } else {
                    write!(f, "{}", left)?;
                }

                write!(f, " || ")?;

                if matches!(right.as_ref(), Expression::Or(_, _)) {
                    write!(f, "({})", right)
                } else {
                    write!(f, "{}", right)
                }
            }
        }
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.expression)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::ast::{Expression, Literal, PresetCharSetName, Program};

    #[test]
    fn test_display() {
        let program = Program {
            expression: Expression::Group(vec![
                Expression::Literal(Literal::Char('a')),
                Expression::Literal(Literal::String("foo".to_string())),
                Expression::Literal(Literal::PresetCharSet(PresetCharSetName::CharWord)),
            ]),
        };

        assert_eq!(program.to_string(), r#"('a', "foo", char_word)"#);
    }
}
