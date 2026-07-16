// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

use crate::{
    ast::{
        BackReference, CharRange, CharSet, CharSetElement, Expression, FunctionArgument,
        FunctionCall, FunctionName, Literal, PresetCharSetName, Program,
    },
    error::AnreError,
    peekable_iter::PeekableIter,
    range::Range,
};

use super::{
    lexer::lex_from_str,
    token::{Repetition, Token, TokenWithRange},
};

pub fn parse_from_str(s: &str) -> Result<Program, AnreError> {
    let tokens = lex_from_str(s)?;
    let mut token_iter = tokens.into_iter();
    let mut peekable_token_iter = PeekableIter::new(&mut token_iter);
    let mut parser = Parser::new(&mut peekable_token_iter);
    parser.parse_program()
}

pub struct Parser<'a> {
    upstream: &'a mut PeekableIter<'a, TokenWithRange>,

    /// Range of the most recently consumed token.
    pub last_range: Range,
}

impl<'a> Parser<'a> {
    fn new(upstream: &'a mut PeekableIter<'a, TokenWithRange>) -> Self {
        Self {
            upstream,
            last_range: Range::default(),
        }
    }

    fn next_token(&mut self) -> Option<Token> {
        match self.next_token_with_range() {
            Some(TokenWithRange { token, range }) => {
                self.last_range = range;
                Some(token)
            }
            None => None,
        }
    }

    fn next_token_with_range(&mut self) -> Option<TokenWithRange> {
        match self.upstream.next() {
            Some(token_with_range) => {
                self.last_range = token_with_range.range;
                Some(token_with_range)
            }
            None => None,
        }
    }

    fn peek_token(&self, offset: usize) -> Option<&Token> {
        match self.upstream.peek(offset) {
            Some(TokenWithRange { token, .. }) => Some(token),
            None => None,
        }
    }

    fn peek_range(&self, offset: usize) -> Option<&Range> {
        match self.upstream.peek(offset) {
            Some(TokenWithRange { range, .. }) => Some(range),
            None => None,
        }
    }

    // Consumes one token and requires it to match `expected_token`.
    fn consume_token_and_assert(
        &mut self,
        expected_token: &Token,
        token_description: &str,
    ) -> Result<(), AnreError> {
        match self.next_token() {
            Some(token) => {
                if &token == expected_token {
                    Ok(())
                } else {
                    Err(AnreError::MessageWithRange(
                        format!("Expected token: {}.", token_description),
                        self.last_range,
                    ))
                }
            }
            None => Err(AnreError::UnexpectedEndOfDocument(format!(
                "Expected token: {}.",
                token_description
            ))),
        }
    }

    // Consumes `]`.
    fn consume_closing_bracket(&mut self) -> Result<(), AnreError> {
        self.consume_token_and_assert(&Token::CharSetEnd, "closing bracket")
    }

    // Consumes `)`.
    fn consume_closing_parenthese(&mut self) -> Result<(), AnreError> {
        self.consume_token_and_assert(&Token::GroupEnd, "closing parenthese \")\"")
    }
}

impl Parser<'_> {
    pub fn parse_program(&mut self) -> Result<Program, AnreError> {
        let expression = self.parse_expression()?;

        if self.peek_token(0).is_some() {
            return Err(AnreError::MessageWithRange(
                "Only one top-level expression is allowed. Wrap multiple expressions in a group."
                    .to_owned(),
                *self.peek_range(0).unwrap(),
            ));
        }

        Ok(Program { expression })
    }

    fn parse_expression(&mut self) -> Result<Expression, AnreError> {
        // ```diagram
        // token ...
        // -----
        // ^
        // |__ current, None or Some(...)
        // ```

        self.parse_logic_or()
    }

    fn parse_logic_or(&mut self) -> Result<Expression, AnreError> {
        // ```diagram
        // expression || expression
        // ```

        let mut left = self.parse_consequent_expression()?;

        // In the traditional regular expressions, "groups" are implied
        // on both sides of the "logic or" operator ("|").
        //
        // The | operator has the lowest precedence in a regular expression.
        // If you want to use a disjunction as a part of a bigger pattern,
        // you must group it.
        //
        // For example:
        //
        // "ab|cd" == "(ab)|(cd)" != "a(b|c)d"
        //
        // ref:
        // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Regular_expressions/Disjunction
        while let Some(Token::LogicOr) = self.peek_token(0) {
            self.next_token(); // consume "||"

            // Operator associativity:
            //
            // - https://en.wikipedia.org/wiki/Operator_associativity
            // - https://en.wikipedia.org/wiki/Operators_in_C_and_C%2B%2B#Operator_precedence
            //
            // Representation:
            // - left-associative (left-to-right associative)
            //   `a || b || c -> (a || b) || c`
            // - right-associative (right-to-left associative)
            //   `a || b || c -> a || (b || c)`
            //
            // Call `parse_expression` for right-to-left associative parsing, for example:
            // `let right = self.parse_expression()?;`
            // Or call `parse_consequent_expression` for left-to-right associative parsing, for example:
            // `let right = self.parse_consequent_expression()?;`
            //
            // currently right-associative is adopted for efficiency.

            let right = self.parse_expression()?;
            let expression = Expression::Or(Box::new(left), Box::new(right));
            left = expression;
        }

        Ok(left)
    }

    fn parse_consequent_expression(&mut self) -> Result<Expression, AnreError> {
        // ```diagram
        // token ...
        // -----
        // ^
        // | current, None or Some(...)
        // ```

        // In the traditional regular expressions, consecutive expressions
        // are implicit groups.
        //
        // For example:
        //
        // - `abc` = `('a', 'b', 'c')`
        // - `0[xX][0-9a-fA-F]+` = `('0', ['x', 'X'], one_or_more(['0'..'9', 'a'..'f', 'A'..'F']))`
        // - `ab|cd` = `('a', 'b') || ('c', 'd')`

        let mut expressions = vec![];
        while let Some(token) = self.peek_token(0) {
            match token {
                // collect expressions until the next token is a logic or operator
                // or a group end, which cannot be implicitly grouped with
                // the previous expressions.
                Token::LogicOr | Token::GroupEnd => {
                    break;
                }
                _ => {
                    let expression = self.parse_look_ahead_assertion()?;
                    expressions.push(expression);
                }
            }
        }

        if expressions.is_empty() {
            return Err(AnreError::MessageWithRange(
                "Encountered an empty expression.".to_string(),
                self.last_range,
            ));
        }

        // Merge continuous `Literal::Char` to `Literal::String`
        let mut cursor = expressions.len() - 1;
        while cursor > 0 {
            if matches!(expressions[cursor], Expression::Literal(Literal::Char(_))) {
                // check previous expressions until the first non-`Literal::Char` expression
                let mut begin_index = cursor;
                while begin_index > 0 {
                    if !matches!(
                        expressions[begin_index - 1],
                        Expression::Literal(Literal::Char(_))
                    ) {
                        break;
                    }
                    begin_index -= 1;
                }

                // found continuous chars
                if cursor - begin_index > 0 {
                    let s: String = expressions
                        .drain(begin_index..=cursor)
                        .map(|item| {
                            if let Expression::Literal(Literal::Char(c)) = item {
                                c
                            } else {
                                unreachable!()
                            }
                        })
                        .collect();
                    expressions.insert(begin_index, Expression::Literal(Literal::String(s)));

                    if begin_index == 0 {
                        // reached the beginning of the expression list
                        break;
                    } else {
                        // search for the next continuous chars
                        cursor = begin_index - 1;
                    }
                } else {
                    cursor -= 1;
                }
            } else {
                cursor -= 1;
            }
        }

        // escape the group if it contains only one element
        if expressions.len() == 1 {
            let expression = expressions.remove(0);
            Ok(expression)
        } else {
            // create an implicit group
            Ok(Expression::Group(expressions))
        }
    }

    fn parse_look_ahead_assertion(&mut self) -> Result<Expression, AnreError> {
        // look around assertions:
        // - `(?=...)`  Positive lookahead
        // - `(?!...)`  Negative lookahead
        //
        // For example:
        //
        // `a(?=b)` = `is_before('a', 'b')`, `'a'.is_before('b')`
        //
        // means:
        //
        // "match 'a' only if it's followed by 'b'"

        let mut expression = self.parse_notation()?;

        if let Some(token @ (Token::LookAheadGroupStart | Token::LookAheadNegativeGroupStart)) =
            self.peek_token(0)
        {
            let name = match token {
                Token::LookAheadGroupStart => FunctionName::IsBefore,
                Token::LookAheadNegativeGroupStart => FunctionName::IsNotBefore,
                _ => unreachable!(),
            };

            let mut args = vec![];

            self.next_token(); // consume "(?=" or "(?!"
            let arg0 = self.parse_expression()?;
            self.consume_closing_parenthese()?; // consume ")"

            args.push(FunctionArgument::Expression(expression));
            args.push(FunctionArgument::Expression(arg0));

            let function_call = FunctionCall { name, args };
            expression = Expression::FunctionCall(Box::new(function_call));
        }

        Ok(expression)
    }

    fn parse_notation(&mut self) -> Result<Expression, AnreError> {
        // ```diagram
        // expression [ "?" | "+" | "*" | "{N}" | "{N,}" | "{N,M}" ]
        // expression [ "??" | "+?" | "*?" | "{N}?" | "{N,}?" | "{N,M}?" ]
        // ```
        let mut expression = self.parse_primary_expression()?;

        while let Some(token) = self.peek_token(0) {
            match token {
                Token::Optional
                | Token::LazyOptional
                | Token::OneOrMore
                | Token::LazyOneOrMore
                | Token::ZeroOrMore
                | Token::LazyZeroOrMore => {
                    // quantifier, for example: `foo?`, `foo+`, `foo*`, `foo??`, `foo+?`, `foo*?`, etc.

                    let name = match token {
                        // Greedy quantifier
                        Token::Optional => FunctionName::Optional,
                        Token::OneOrMore => FunctionName::OneOrMore,
                        Token::ZeroOrMore => FunctionName::ZeroOrMore,
                        // Lazy quantifier
                        Token::LazyOptional => FunctionName::LazyOptional,
                        Token::LazyOneOrMore => FunctionName::LazyOneOrMore,
                        Token::LazyZeroOrMore => FunctionName::LazyZeroOrMore,
                        _ => unreachable!(),
                    };

                    let function_call = FunctionCall {
                        name,
                        args: vec![FunctionArgument::Expression(expression)],
                    };
                    expression = Expression::FunctionCall(Box::new(function_call));

                    self.next_token(); // consume notation
                }
                Token::Repetition(repetition, lazy) => {
                    // repetition, for example `foo{3}`, `foo{3,}`, `foo{3,5}`, `foo{3}?`, `foo{3,}?`, `foo{3,5}?`, etc.

                    let mut args = vec![];
                    args.push(FunctionArgument::Expression(expression));

                    let name = match repetition {
                        Repetition::Repeat(n) => {
                            args.push(FunctionArgument::Number(*n));

                            if *lazy {
                                FunctionName::LazyRepeat
                            } else {
                                FunctionName::Repeat
                            }
                        }
                        Repetition::RepeatFrom(n) => {
                            args.push(FunctionArgument::Number(*n));

                            if *lazy {
                                FunctionName::LazyRepeatFrom
                            } else {
                                FunctionName::RepeatFrom
                            }
                        }
                        Repetition::RepeatRange(m, n) => {
                            // `{m..m}` is equivalent to a fixed repetition, so it reuses
                            // the same AST form as `{m}`.
                            if m > n {
                                let range = self.peek_range(0).unwrap();
                                return Err(AnreError::MessageWithRange(
                                    format!(
                                        "Invalid repetition range: {{{},{}}}. The start of the range must be less than or equal to the end.",
                                        m, n
                                    ),
                                    *range,
                                ));
                            }

                            args.push(FunctionArgument::Number(*m));
                            args.push(FunctionArgument::Number(*n));

                            if *lazy {
                                FunctionName::LazyRepeatRange
                            } else {
                                FunctionName::RepeatRange
                            }
                        }
                    };

                    let function_call = FunctionCall { name, args };
                    expression = Expression::FunctionCall(Box::new(function_call));

                    self.next_token(); // consume notation
                }

                _ => {
                    break;
                }
            }
        }

        Ok(expression)
    }

    fn parse_primary_expression(&mut self) -> Result<Expression, AnreError> {
        // primary expressions:
        // - literal
        // - line assertion
        // - boundary assertion
        // - look around assertion
        // - explicit group
        // - back reference
        let expression = match self.peek_token(0).unwrap() {
            Token::LineBoundaryAssertionStart => {
                self.next_token(); // consume '^'
                Expression::FunctionCall(Box::new(FunctionCall {
                    name: FunctionName::IsStart,
                    args: vec![],
                }))
            }
            Token::LineBoundaryAssertionEnd => {
                self.next_token(); // consume '$'
                Expression::FunctionCall(Box::new(FunctionCall {
                    name: FunctionName::IsEnd,
                    args: vec![],
                }))
            }
            Token::WordBoundaryAssertion(b) => {
                let negative = *b;
                self.next_token(); // consume boundary assertion

                if negative {
                    Expression::FunctionCall(Box::new(FunctionCall {
                        name: FunctionName::IsNotBound,
                        args: vec![],
                    }))
                } else {
                    Expression::FunctionCall(Box::new(FunctionCall {
                        name: FunctionName::IsBound,
                        args: vec![],
                    }))
                }
            }
            token @ (Token::LookBehindGroupStart | Token::LookBehindNegativeGroupStart) => {
                // look around assertions:
                // - `(?<=...)` Positive lookbehind
                // - `(?<!...)` Negative lookbehind
                //
                // For example:
                //
                // `(?<=a)b` = `is_after('b', 'a')`, `'b'.is_after('a')`
                //
                // means:
                //
                // "match 'b' only if it's preceded by 'a'"
                let name = match token {
                    Token::LookBehindGroupStart => FunctionName::IsAfter,
                    Token::LookBehindNegativeGroupStart => FunctionName::IsNotAfter,
                    _ => unreachable!(),
                };

                let mut args = vec![];

                self.next_token(); // consume "(?<=" or "(?<!"
                let arg0 = self.parse_expression()?;
                self.consume_closing_parenthese()?; // consume ")"

                let expression = self.parse_expression()?;
                args.push(FunctionArgument::Expression(expression));
                args.push(FunctionArgument::Expression(arg0));

                let function_call = FunctionCall { name, args };
                Expression::FunctionCall(Box::new(function_call))
            }
            Token::GroupStart => {
                // generic group (indexed capture group)
                // `( expression... )`
                self.next_token().unwrap(); // consume "("
                let expression = self.parse_expression()?;
                self.consume_closing_parenthese()?; // consume ")"

                let function_call = FunctionCall {
                    name: FunctionName::Index,
                    args: vec![FunctionArgument::Expression(expression)],
                };
                Expression::FunctionCall(Box::new(function_call))
            }
            Token::NonCaptureGroupStart => {
                // non-capturing group
                // `(?: expression )`
                self.next_token().unwrap(); // consume "("
                let expression = self.parse_expression()?;
                self.consume_closing_parenthese()?; // consume ")"

                expression
            }
            Token::NamedCaptureGroupStart(name_ref) => {
                // named capturing group
                // `(?<...> expression )`
                let name = name_ref.to_owned();
                self.next_token().unwrap(); // consume "("
                let expression = self.parse_expression()?;
                self.consume_closing_parenthese()?; // consume ")"

                let function_call = FunctionCall {
                    name: FunctionName::Name,
                    args: vec![
                        FunctionArgument::Expression(expression),
                        FunctionArgument::Identifier(name),
                    ],
                };
                Expression::FunctionCall(Box::new(function_call))
            }
            Token::BackReferenceNumber(index_ref) => {
                let index = *index_ref;
                self.next_token(); // consume '\num'
                Expression::BackReference(BackReference::Index(index))
            }
            Token::BackReferenceName(name_ref) => {
                let name = name_ref.to_owned();
                self.next_token(); // consume '\k<name>'
                Expression::BackReference(BackReference::Name(name))
            }
            _ => {
                let literal = self.parse_literal()?;
                Expression::Literal(literal)
            }
        };

        Ok(expression)
    }

    fn parse_literal(&mut self) -> Result<Literal, AnreError> {
        // literals:
        // - `.` (any character)
        // - char
        // - charset
        // - preset charset

        let literal = match self.peek_token(0).unwrap() {
            Token::CharSetStart | Token::CharSetStartNegative => {
                let charset = self.parse_charset()?;
                Literal::CharSet(charset)
            }
            Token::Char(char_ref) => {
                let c = *char_ref;
                self.next_token(); // consume char
                Literal::Char(c)
            }
            Token::PresetCharSet(preset_charset_name_ref) => {
                let preset_charset_name =
                    PresetCharSetName::try_from(*preset_charset_name_ref).unwrap();
                self.next_token(); // consume preset charset
                Literal::PresetCharSet(preset_charset_name)
            }
            Token::Dot => {
                self.next_token(); // consume special char
                Literal::AnyChar
            }
            _ => {
                return Err(AnreError::MessageWithRange(
                    "Expected a literal.".to_string(),
                    self.last_range,
                ));
            }
        };

        Ok(literal)
    }

    fn parse_charset(&mut self) -> Result<CharSet, AnreError> {
        // ```diagram
        // [ {^} {char | char_range | preset_charset} ] ?
        // -                                            -
        // ^                                            ^__ to here
        // |__ current, validated
        // ```

        let opening_token = self.next_token().unwrap(); // consume '[' or '[^'
        let negative = matches!(opening_token, Token::CharSetStartNegative);

        let mut elements = vec![];
        while let Some(token) = self.peek_token(0) {
            if token == &Token::CharSetEnd {
                break;
            }

            match token {
                Token::Char(c_ref) => {
                    // char
                    let c = *c_ref;
                    self.next_token(); // consume char
                    elements.push(CharSetElement::Char(c));
                }
                Token::CharRange(from, to) => {
                    // character range
                    let char_range = CharRange {
                        start: *from,
                        end_inclusive: *to,
                    };
                    self.next_token(); // consume char range
                    elements.push(CharSetElement::CharRange(char_range));
                }
                Token::PresetCharSet(preset_charset_name_ref) => {
                    // preset charset
                    let preset_charset_name =
                        PresetCharSetName::try_from(*preset_charset_name_ref).unwrap();
                    self.next_token(); // consume preset charset
                    elements.push(CharSetElement::PresetCharSet(preset_charset_name));
                }
                _ => {
                    return Err(AnreError::MessageWithRange(
                        "Unsupported character set element.".to_string(),
                        *self.peek_range(0).unwrap(),
                    ));
                }
            }
        }

        self.consume_closing_bracket()?;

        let charset = CharSet { negative, elements };

        Ok(charset)
    }
}

impl TryFrom<char> for PresetCharSetName {
    type Error = ();

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            'w' => Ok(PresetCharSetName::CharWord),
            'W' => Ok(PresetCharSetName::CharNotWord),
            's' => Ok(PresetCharSetName::CharSpace),
            'S' => Ok(PresetCharSetName::CharNotSpace),
            'd' => Ok(PresetCharSetName::CharDigit),
            'D' => Ok(PresetCharSetName::CharNotDigit),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        ast::{
            CharRange, CharSet, CharSetElement, Expression, Literal, PresetCharSetName, Program,
        },
        error::AnreError,
        position::Position,
        range::Range,
    };

    use super::parse_from_str;

    #[test]
    fn test_parse_literal() {
        {
            let program = parse_from_str(r#"a"#).unwrap();

            assert_eq!(
                program,
                Program {
                    expression: Expression::Literal(Literal::Char('a')),
                }
            );

            assert_eq!(program.to_string(), r#"'a'"#);
        }

        // Merge continuous chars.
        {
            let program = parse_from_str(r#".foo"#).unwrap();

            assert_eq!(
                program,
                Program {
                    expression: Expression::Group(vec![
                        Expression::Literal(Literal::AnyChar),
                        Expression::Literal(Literal::String("foo".to_string())),
                    ])
                }
            );

            assert_eq!(program.to_string(), r#"(char_any, "foo")"#);
        }
    }

    #[test]
    fn test_parse_literal_preset_charset() {
        let program = parse_from_str(r#"\w\W\d\D\s\S"#).unwrap();

        assert_eq!(
            program,
            Program {
                expression: Expression::Group(vec![
                    Expression::Literal(Literal::PresetCharSet(PresetCharSetName::CharWord)),
                    Expression::Literal(Literal::PresetCharSet(PresetCharSetName::CharNotWord)),
                    Expression::Literal(Literal::PresetCharSet(PresetCharSetName::CharDigit)),
                    Expression::Literal(Literal::PresetCharSet(PresetCharSetName::CharNotDigit)),
                    Expression::Literal(Literal::PresetCharSet(PresetCharSetName::CharSpace)),
                    Expression::Literal(Literal::PresetCharSet(PresetCharSetName::CharNotSpace)),
                ])
            }
        );

        assert_eq!(
            program.to_string(),
            r#"(char_word, char_not_word, char_digit, char_not_digit, char_space, char_not_space)"#
        );
    }

    #[test]
    fn test_parse_literal_charset() {
        let program = parse_from_str(r#"[a0-9\w]"#).unwrap();

        assert_eq!(
            program,
            Program {
                expression: Expression::Literal(Literal::CharSet(CharSet {
                    negative: false,
                    elements: vec![
                        CharSetElement::Char('a'),
                        CharSetElement::CharRange(CharRange {
                            start: '0',
                            end_inclusive: '9'
                        }),
                        CharSetElement::PresetCharSet(PresetCharSetName::CharWord),
                    ]
                }))
            }
        );

        assert_eq!(program.to_string(), r#"['a', '0'..'9', char_word]"#);

        // negative charset
        assert_eq!(
            parse_from_str(r#"[^a-z\s]"#,).unwrap().to_string(),
            r#"!['a'..'z', char_space]"#
        );

        assert_eq!(
            parse_from_str(r#"[-a-f0-9]"#,).unwrap().to_string(),
            r#"['-', 'a'..'f', '0'..'9']"#
        );
    }

    #[test]
    fn test_parse_quantifier() {
        assert_eq!(
            parse_from_str(r#"a?b+c*x??y+?z*?"#,).unwrap().to_string(),
            r#"(optional('a'), one_or_more('b'), zero_or_more('c'), lazy_optional('x'), lazy_one_or_more('y'), lazy_zero_or_more('z'))"#
        );
    }

    #[test]
    fn test_parse_repetition() {
        assert_eq!(
            parse_from_str(r#"a{3}b{5,7}c{11,}x{3}?y{5,7}?z{11,}?"#,)
                .unwrap()
                .to_string(),
            r#"(repeat('a', 3), repeat_range('b', 5, 7), repeat_from('c', 11), lazy_repeat('x', 3), lazy_repeat_range('y', 5, 7), lazy_repeat_from('z', 11))"#
        );

        // Special case: zero repetition and range with the same start and end
        assert_eq!(
            parse_from_str(r#"a{0}b{3,3}c{5,5}?"#,).unwrap().to_string(),
            r#"(repeat('a', 0), repeat_range('b', 3, 3), lazy_repeat_range('c', 5, 5))"#
        );

        // err: invalid repetition range, `{m,n}` is invalid if m > n
        assert!(matches!(
            parse_from_str(r#"a{5,3}"#,),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 1,
                        line: 0,
                        column: 1
                    },
                    end_inclusive: Position {
                        index: 5,
                        line: 0,
                        column: 5
                    }
                }
            ))
        ));
    }

    #[test]
    fn test_parse_line_assertions() {
        assert_eq!(
            parse_from_str(r#"^a$"#,).unwrap().to_string(),
            r#"(is_start(), 'a', is_end())"#
        );
    }

    #[test]
    fn test_parse_boundary_assertions() {
        assert_eq!(
            parse_from_str(r#"\ba\B"#,).unwrap().to_string(),
            r#"(is_bound(), 'a', is_not_bound())"#
        );
    }

    #[test]
    fn test_parse_index_capture_and_backreference() {
        assert_eq!(
            parse_from_str(r#"(a)"#,).unwrap().to_string(),
            r#"index('a')"#
        );

        assert_eq!(
            parse_from_str(r#"(a\d)"#,).unwrap().to_string(),
            r#"index(('a', char_digit))"#
        );

        assert_eq!(
            parse_from_str(r#"(\d+)\.\1"#,).unwrap().to_string(),
            r#"(index(one_or_more(char_digit)), '.', ^1)"#
        );
    }

    #[test]
    fn test_parse_name_capture_and_backreference() {
        assert_eq!(
            parse_from_str(r#"(?<x>a)"#,).unwrap().to_string(),
            r#"name('a', x)"#
        );

        assert_eq!(
            parse_from_str(r#"(?<x>a\d)"#,).unwrap().to_string(),
            r#"name(('a', char_digit), x)"#
        );

        assert_eq!(
            parse_from_str(r#"(?<a>\d)x\k<a>"#,).unwrap().to_string(),
            r#"(name(char_digit, a), 'x', a)"#
        );
    }

    #[test]
    fn test_parse_logic_or() {
        {
            let program = parse_from_str(r#"a|b"#).unwrap();

            assert_eq!(
                program,
                Program {
                    expression: Expression::Or(
                        Box::new(Expression::Literal(Literal::Char('a'))),
                        Box::new(Expression::Literal(Literal::Char('b'))),
                    )
                }
            );

            assert_eq!(program.to_string(), r#"'a' || 'b'"#);
        }

        // multiple operands
        {
            let program = parse_from_str(r#"a|b|c"#).unwrap();

            assert_eq!(
                program,
                Program {
                    expression: Expression::Or(
                        Box::new(Expression::Literal(Literal::Char('a'))),
                        Box::new(Expression::Or(
                            Box::new(Expression::Literal(Literal::Char('b'))),
                            Box::new(Expression::Literal(Literal::Char('c'))),
                        )),
                    )
                }
            );

            assert_eq!(program.to_string(), r#"'a' || ('b' || 'c')"#);
        }

        // expression as operand
        assert_eq!(
            parse_from_str(r#"\d+|[\w-]+"#,).unwrap().to_string(),
            r#"one_or_more(char_digit) || one_or_more([char_word, '-'])"#
        );

        // group + logic or
        assert_eq!(
            parse_from_str(r#"(a|b)|c"#,).unwrap().to_string(),
            r#"index('a' || 'b') || 'c'"#
        );

        // group + logic or + group
        assert_eq!(
            parse_from_str(r#"(a\w)|(b\d)"#,).unwrap().to_string(),
            r#"index(('a', char_word)) || index(('b', char_digit))"#
        );

        // string + logic or
        assert_eq!(
            parse_from_str(r#"ab|cd"#,).unwrap().to_string(),
            r#""ab" || "cd""#
        );
    }

    #[test]
    fn test_parse_group() {
        // groups are indexed capturing groups by default

        assert_eq!(
            parse_from_str(r#"(?:foo\d)(?:b(?:bar\d))$"#,)
                .unwrap()
                .to_string(),
            r#"(("foo", char_digit), ('b', ("bar", char_digit)), is_end())"#
        );

        // nested groups
        assert_eq!(
            parse_from_str(r#"(?:foo\d){3}(?:b(?:bar){5})$"#,)
                .unwrap()
                .to_string(),
            r#"(repeat(("foo", char_digit), 3), ('b', repeat("bar", 5)), is_end())"#
        );

        // escape nested groups
        assert_eq!(
            parse_from_str(r#"(?:(?:a\db))"#,).unwrap().to_string(),
            r#"('a', char_digit, 'b')"#
        );
    }

    #[test]
    fn test_parse_operate_on_group() {
        // Quantifier on group
        assert_eq!(
            parse_from_str(r#"a(?:b)?(?:x\d)?"#,).unwrap().to_string(),
            r#"('a', optional('b'), optional(('x', char_digit)))"#
        );

        // Repetition on group
        assert_eq!(
            parse_from_str(r#"a(?:b){3,5}(?:x\d){7,11}"#,)
                .unwrap()
                .to_string(),
            r#"('a', repeat_range('b', 3, 5), repeat_range(('x', char_digit), 7, 11))"#
        );

        // Quantifier on indexed capturing group
        assert_eq!(
            parse_from_str(r#"a(b?)((?:x\d)?)"#,).unwrap().to_string(),
            r#"('a', index(optional('b')), index(optional(('x', char_digit))))"#
        );

        // Repetition on indexed capturing group
        assert_eq!(
            parse_from_str(r#"a(b{3,5})((?:x\d){7,11})"#,)
                .unwrap()
                .to_string(),
            r#"('a', index(repeat_range('b', 3, 5)), index(repeat_range(('x', char_digit), 7, 11)))"#
        );

        // Quantifier on named capturing group
        assert_eq!(
            parse_from_str(r#"a(?<foo>b)?(?<bar>x\d)?"#,)
                .unwrap()
                .to_string(),
            r#"('a', optional(name('b', foo)), optional(name(('x', char_digit), bar)))"#
        );

        // Repetition on named capturing group
        assert_eq!(
            parse_from_str(r#"a(?<foo>b){3,5}(?<bar>x\d){7,11}"#,)
                .unwrap()
                .to_string(),
            r#"('a', repeat_range(name('b', foo), 3, 5), repeat_range(name(('x', char_digit), bar), 7, 11))"#
        );

        // Precedence of group and quantifier
        assert_eq!(
            parse_from_str(r#"(x)?(y?)"#,).unwrap().to_string(),
            r#"(optional(index('x')), index(optional('y')))"#
        );
    }

    #[test]
    fn test_parse_lookaround_assertion() {
        assert_eq!(
            parse_from_str(r#"(?<=a)b"#,).unwrap().to_string(),
            r#"is_after('b', 'a')"#
        );

        assert_eq!(
            parse_from_str(r#"(?<!a)b"#,).unwrap().to_string(),
            r#"is_not_after('b', 'a')"#
        );

        assert_eq!(
            parse_from_str(r#"a(?=b)"#,).unwrap().to_string(),
            r#"is_before('a', 'b')"#
        );

        assert_eq!(
            parse_from_str(r#"a(?!b)"#,).unwrap().to_string(),
            r#"is_not_before('a', 'b')"#
        );
    }

    #[test]
    fn test_parse_examples() {
        assert_eq!(
            parse_from_str(r#"\d+"#,).unwrap().to_string(),
            "one_or_more(char_digit)"
        );

        assert_eq!(
            parse_from_str(r#"0x[0-9a-fA-F]+"#,).unwrap().to_string(),
            "(\"0x\", one_or_more(['0'..'9', 'a'..'f', 'A'..'F']))"
        );

        assert_eq!(
            parse_from_str(r#"^[\w.-]+(\+[\w-]+)?@([a-zA-Z0-9-]+\.)+[a-z]{2,}$"#,)
                .unwrap()
                .to_string(),
            "(is_start(), \
one_or_more([char_word, '.', '-']), \
optional(index(('+', one_or_more([char_word, '-'])))), \
'@', \
one_or_more(index((one_or_more(['a'..'z', 'A'..'Z', '0'..'9', '-']), '.'))), \
repeat_from(['a'..'z'], 2), \
is_end())"
        );

        let ipv4_regex = parse_from_str(
            r#"^((25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)\.){3}(25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)$"#,
        )
        .unwrap()
        .to_string();

        let expected_part_str = r#"index(("25", ['0'..'5']) || (('2', ['0'..'4'], char_digit) || (('1', char_digit, char_digit) || ((['1'..'9'], char_digit) || char_digit))))"#;
        let expected_str = format!(
            "(is_start(), repeat(index(({}, '.')), 3), {}, is_end())",
            expected_part_str, expected_part_str
        );

        assert_eq!(ipv4_regex, expected_str);

        assert_eq!(
            parse_from_str(r#"<(?<tag_name>\w+)(\s\w+(="\w+")?)*>.+?</\k<tag_name>>"#,)
                .unwrap()
                .to_string(),
            "(\
'<', \
name(one_or_more(char_word), tag_name), \
zero_or_more(index((char_space, one_or_more(char_word), optional(index((\"=\\\"\", one_or_more(char_word), '\\\"')))))), \
'>', \
lazy_one_or_more(char_any), \
\"</\", tag_name, '>'\
)"
        );
    }
}
