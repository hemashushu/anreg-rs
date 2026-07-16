// Copyright (c) 2026 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

use crate::{
    anre::macro_expander::expand,
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
    token::{Token, TokenWithRange},
};

pub fn parse_from_str(s: &str) -> Result<Program, AnreError> {
    let tokens = lex_from_str(s)?;
    let expanded_tokens = expand(tokens)?;
    let mut token_iter = expanded_tokens.into_iter();
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

    // Returns `true` when the token at `offset` matches `expected_token`.
    fn peek_token_and_equals(&self, offset: usize, expected_token: &Token) -> bool {
        matches!(
            self.peek_token(offset),
            Some(token) if token == expected_token)
    }

    fn consume_identifier(&mut self) -> Result<String, AnreError> {
        match self.next_token() {
            Some(Token::Identifier(id)) => Ok(id),
            Some(_) => Err(AnreError::MessageWithPosition(
                "Expected an identifier.".to_string(),
                self.last_range.start,
            )),
            None => Err(AnreError::UnexpectedEndOfDocument(
                "Expected an identifier.".to_string(),
            )),
        }
    }

    fn consume_number(&mut self) -> Result<usize, AnreError> {
        match self.next_token() {
            Some(Token::Number(i)) => Ok(i),
            Some(_) => Err(AnreError::MessageWithPosition(
                "Expected a number.".to_string(),
                self.last_range.start,
            )),
            None => Err(AnreError::UnexpectedEndOfDocument(
                "Expected a number.".to_string(),
            )),
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

    // Consumes `(`.
    // fn consume_opening_parenthesis(&mut self) -> Result<(), AnreError> {
    //     self.consume_token_and_assert(&Token::ParenthesisOpen, "opening parenthesis")
    // }

    // Consumes `)`.
    fn consume_closing_parenthesis(&mut self) -> Result<(), AnreError> {
        self.consume_token_and_assert(&Token::ParenthesisClose, "closing parenthesis")
    }

    // Consumes `]`.
    fn consume_closing_bracket(&mut self) -> Result<(), AnreError> {
        self.consume_token_and_assert(&Token::BracketClose, "closing bracket")
    }

    // Consumes `}`.
    fn consume_closing_brace(&mut self) -> Result<(), AnreError> {
        self.consume_token_and_assert(&Token::BraceClose, "closing brace")
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

        // Parsing proceeds from the lowest-precedence form to the highest.
        // Each layer delegates to the next tighter layer and then folds its own
        // operator on top.
        //
        // Lower precedence, parsed later:
        // 1. binary expressions (logic or)
        // 2. named capturing
        // 3. indexed capturing
        // 4. method call and quantifier
        // 5. primary expressions (literal, group, identifier, function call)
        // Higher precedence, parsed first.

        self.parse_logic_or()
    }

    fn parse_logic_or(&mut self) -> Result<Expression, AnreError> {
        // ```diagram
        // expression || expression
        // ```

        let mut left = self.parse_named_capture()?;

        while self.peek_token_and_equals(0, &Token::LogicOr) {
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
            // Or call `parse_named_capture` for left-to-right associative parsing, for example:
            // `let right = self.parse_named_capture()?;`
            //
            // currently right-associative is adopted for efficiency.

            let right = self.parse_expression()?;
            let expression = Expression::Or(Box::new(left), Box::new(right));
            left = expression;
        }

        Ok(left)
    }

    fn parse_named_capture(&mut self) -> Result<Expression, AnreError> {
        // ```diagram
        // expression as identifier
        // ```

        let expression = self.parse_index_capture()?;
        if self.peek_token_and_equals(0, &Token::Keyword("as".to_string())) {
            self.next_token(); // consume "as"

            let name = self.consume_identifier()?;

            if matches!(&expression, Expression::FunctionCall(f) if f.name == FunctionName::Index) {
                let Expression::FunctionCall(mut fc) = expression else {
                    unreachable!();
                };

                // Naming an indexed capturing should not produce `Name(Index(expr))`.
                // Named capturing implies indexed capturing semantics.
                let index_capture_expr = fc.args.remove(0);
                let expression = Expression::FunctionCall(Box::new(FunctionCall {
                    name: FunctionName::Name,
                    args: vec![index_capture_expr, FunctionArgument::Identifier(name)],
                }));
                Ok(expression)
            } else {
                // Wrap the expression in a new named capturing group.
                let index_capture_expr = FunctionArgument::Expression(expression);
                let expression = Expression::FunctionCall(Box::new(FunctionCall {
                    name: FunctionName::Name,
                    args: vec![index_capture_expr, FunctionArgument::Identifier(name)],
                }));
                Ok(expression)
            }
        } else {
            Ok(expression)
        }
    }

    fn parse_index_capture(&mut self) -> Result<Expression, AnreError> {
        // ```diagram
        // # expression
        // ```

        if self.peek_token_and_equals(0, &Token::Hash) {
            self.next_token(); // consume '#'

            let expression = self.parse_notation_and_method_call()?;

            if matches!(&expression, Expression::FunctionCall(f) if f.name == FunctionName::Name) {
                // Named capturing records both the span and the name,
                // so an extra indexed capturing wrapper would be redundant.
                Ok(expression)
            } else {
                // Wrap the expression in an indexed capturing.
                let expression = Expression::FunctionCall(Box::new(FunctionCall {
                    name: FunctionName::Index,
                    args: vec![FunctionArgument::Expression(expression)],
                }));
                Ok(expression)
            }
        } else {
            self.parse_notation_and_method_call()
        }
    }

    fn parse_notation_and_method_call(&mut self) -> Result<Expression, AnreError> {
        // ```diagram
        // expression.identifier(arguments)
        // expression [ "?" | "+" | "*" | "{N}" | "{N..}" | "{N..M}" ]
        // expression [ "??" | "+?" | "*?" | "{N}?" | "{N..}?" | "{N..M}?" ]
        // ```

        let mut expression = self.parse_primary_expression()?;

        while let Some(token) = self.peek_token(0) {
            match token {
                Token::Dot
                    if matches!(self.peek_token(1), Some(Token::Identifier(_)))
                        && matches!(self.peek_token(2), Some(Token::ParenthesisOpen)) =>
                {
                    // method call, for example `foo.bar()`, `foo.bar(arg1, arg2)`, `foo.bar(3)`, etc.

                    let function_call = self.continue_parse_method_call(expression)?;
                    expression = Expression::FunctionCall(Box::new(function_call));
                }
                Token::Optional
                | Token::OneOrMore
                | Token::ZeroOrMore
                | Token::LazyOptional
                | Token::LazyOneOrMore
                | Token::LazyZeroOrMore => {
                    // quantifier, for example `foo?`, `foo+`, `foo*`, `foo??`, `foo+?`, `foo*?`, etc.

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
                Token::BraceOpen => {
                    // repetition, for example `foo{3}`, `foo{3..}`, `foo{3..5}`, `foo{3}?`, `foo{3..}?`, `foo{3..5}?`, etc.

                    let range_start = *self.peek_range(0).unwrap();
                    let (repetition, lazy) = self.continue_parse_repetition()?;
                    let range_end = self.last_range;

                    let mut args = vec![];
                    args.push(FunctionArgument::Expression(expression));

                    let name = match repetition {
                        Repetition::Repeat(n) => {
                            args.push(FunctionArgument::Number(n));

                            if lazy {
                                FunctionName::LazyRepeat
                            } else {
                                FunctionName::Repeat
                            }
                        }
                        Repetition::RepeatFrom(n) => {
                            args.push(FunctionArgument::Number(n));

                            if lazy {
                                FunctionName::LazyRepeatFrom
                            } else {
                                FunctionName::RepeatFrom
                            }
                        }
                        Repetition::RepeatRange(m, n) => {
                            if m > n {
                                let range = Range::merge(&range_start, &range_end);
                                return Err(AnreError::MessageWithRange(
                                    format!(
                                        "Invalid repetition range {{{},{}}}. The start of the range must be less than or equal to the end.",
                                        m, n
                                    ),
                                    range,
                                ));
                            }

                            args.push(FunctionArgument::Number(m));
                            args.push(FunctionArgument::Number(n));

                            if lazy {
                                FunctionName::LazyRepeatRange
                            } else {
                                FunctionName::RepeatRange
                            }
                        }
                    };

                    let function_call = FunctionCall { name, args };
                    expression = Expression::FunctionCall(Box::new(function_call));
                }
                _ => {
                    // other cases, break the loop and return the expression parsed so far.
                    break;
                }
            }
        }

        Ok(expression)
    }

    fn continue_parse_method_call(
        &mut self,
        expression: Expression,
    ) -> Result<FunctionCall, AnreError> {
        // ```diagram
        // "." identifier "(" {args} ")" ?
        // --- ---------- ---            -
        // ^   ^          ^__ validated  ^__ to here
        // |   |__ validated
        // |__ current, validated
        // ```

        self.next_token(); // consume '.'

        let identifier = self.consume_identifier()?; // consume function name
        let name: FunctionName = identifier.as_str().try_into().map_err(|_| {
            AnreError::MessageWithRange(
                format!("Unsupported function \"{}\"", identifier),
                self.last_range,
            )
        })?;

        let mut args = vec![];
        args.push(FunctionArgument::Expression(expression));

        let additional_args = self.continue_parse_function_arguments()?;
        args.extend(additional_args);

        let function_call = FunctionCall { name, args };
        Ok(function_call)
    }

    fn continue_parse_repetition(&mut self) -> Result<(Repetition, /* is_lazy */ bool), AnreError> {
        // ```diagram
        // {m..n}? ?
        // -       -
        // ^       ^__ to here
        // | current, validated
        // ```

        self.next_token(); // consume '{'

        let from = self.consume_number()?;

        let repetition = if self.peek_token_and_equals(0, &Token::Range) {
            // Example:
            // - `{m..}`
            // - `{m..n}`

            self.next_token(); // consume '..'

            if let Some(Token::Number(v)) = self.peek_token(0) {
                let to = *v;
                self.next_token(); // consume number
                Repetition::RepeatRange(from, to)
            } else {
                Repetition::RepeatFrom(from)
            }
        } else {
            // Example:
            // `{m}`
            Repetition::Repeat(from)
        };

        self.consume_closing_brace()?; // consume '}'

        let lazy = if self.peek_token_and_equals(0, &Token::Optional) {
            self.next_token(); // consume trailing '?'
            true
        } else {
            false
        };

        Ok((repetition, lazy))
    }

    fn parse_primary_expression(&mut self) -> Result<Expression, AnreError> {
        // primary expressions:
        // - literal
        // - group
        // - named backreference
        // - indexed backreference
        // - function call

        let expression = match self.peek_token(0) {
            Some(token) => {
                match token {
                    Token::ParenthesisOpen => {
                        // group
                        self.parse_group()?
                    }
                    Token::Identifier(_)
                        if self.peek_token_and_equals(1, &Token::ParenthesisOpen) =>
                    {
                        // function call
                        self.parse_function_call()?
                    }
                    Token::Identifier(id)
                        if id != "char_any"
                            && PresetCharSetName::try_from(id.as_str()).is_err() =>
                    {
                        // A bare identifier that is neither a function call nor a
                        // literal name is treated as a named backreference.
                        let name = id.to_owned();
                        self.next_token(); // consume identifier
                        Expression::BackReference(BackReference::Name(name))
                    }
                    Token::Caret if matches!(self.peek_token(1), Some(Token::Number(_))) => {
                        // numeric backreference, for example `^1`, `^2`, etc.
                        self.next_token(); // consume '^'
                        let index = self.consume_number()?;
                        Expression::BackReference(BackReference::Index(index))
                    }
                    _ => {
                        let literal = self.parse_literal()?;
                        Expression::Literal(literal)
                    }
                }
            }
            None => {
                return Err(AnreError::UnexpectedEndOfDocument(
                    "Expected an expression.".to_string(),
                ));
            }
        };

        Ok(expression)
    }

    fn parse_group(&mut self) -> Result<Expression, AnreError> {
        // ```diagram
        // (expression, ...) ?
        // -                 -
        // ^                 ^__ to here
        // |__ current, validated
        // ```

        self.next_token(); // consume "("
        let mut expressions: Vec<Expression> = vec![];

        while let Some(token) = self.peek_token(0) {
            if token == &Token::ParenthesisClose {
                break;
            }

            let expression = self.parse_expression()?;
            expressions.push(expression);
        }

        self.consume_closing_parenthesis()?; // consume ")"

        // Collapse single-element groups. This keeps macro expansion from
        // introducing extra group nodes that do not affect semantics.
        //
        // - `(foo)` -> `foo`
        // - `((foo, bar))` -> `(foo, bar)`
        if expressions.len() == 1 {
            let expression = expressions.remove(0);
            Ok(expression)
        } else {
            Ok(Expression::Group(expressions))
        }
    }

    fn parse_function_call(&mut self) -> Result<Expression, AnreError> {
        // ```diagram
        // identifier ( args... ) ?
        // ---------- -           -
        // ^          ^           ^__ to here
        // |          |__ validated
        // |__ current, validated
        // ```

        let identifier = self.consume_identifier()?;

        let name = identifier.as_str().try_into().map_err(|_| {
            AnreError::MessageWithRange(
                format!("Unsupported function \"{}\"", identifier),
                self.last_range,
            )
        })?;

        let args = self.continue_parse_function_arguments()?;
        let function_call = FunctionCall { name, args };

        Ok(Expression::FunctionCall(Box::new(function_call)))
    }

    fn continue_parse_function_arguments(&mut self) -> Result<Vec<FunctionArgument>, AnreError> {
        // ```diagram
        // (args...)
        // -       -
        // ^       ^__ to here
        // | current, validated
        // ```

        self.next_token(); // consume '('

        let mut args = vec![];

        while let Some(token) = self.peek_token(0) {
            if token == &Token::ParenthesisClose {
                break;
            }

            match token {
                Token::Number(n) => {
                    let number = *n;
                    self.next_token(); // consume number
                    args.push(FunctionArgument::Number(number));
                }
                Token::Identifier(id)
                    if !self.peek_token_and_equals(1, &Token::ParenthesisOpen)
                        && id != "char_any"
                        && PresetCharSetName::try_from(id.as_str()).is_err() =>
                {
                    let identifier = id.to_owned();
                    self.next_token(); // consume identifier
                    args.push(FunctionArgument::Identifier(identifier));
                }
                _ => {
                    let expression = self.parse_expression()?;
                    args.push(FunctionArgument::Expression(expression));
                }
            }
        }

        self.consume_closing_parenthesis()?; // consume ')'

        Ok(args)
    }

    fn parse_literal(&mut self) -> Result<Literal, AnreError> {
        // literals:
        // - `char_any` (any character)
        // - char
        // - string
        // - charset
        // - preset charset

        let literal = match self.peek_token(0).unwrap() {
            Token::BracketOpen => {
                // charset
                let elements = self.parse_charset_elements()?;
                Literal::CharSet(CharSet {
                    negative: false,
                    elements,
                })
            }
            Token::Not if self.peek_token_and_equals(1, &Token::BracketOpen) => {
                // negative charset
                self.next_token();

                let elements = self.parse_charset_elements()?;
                Literal::CharSet(CharSet {
                    negative: true,
                    elements,
                })
            }
            Token::Char(char_ref) => {
                let c = *char_ref;
                self.next_token(); // consume char
                Literal::Char(c)
            }
            Token::String(string_ref) => {
                let string = string_ref.to_owned();
                self.next_token(); // consume string
                Literal::String(string)
            }
            Token::Identifier(id) if id == "char_any" => {
                self.next_token(); // consume "char_any"
                Literal::AnyChar
            }
            Token::Identifier(preset_charset_name_ref) => {
                let preset_charset_name =
                    PresetCharSetName::try_from(preset_charset_name_ref.as_str()).unwrap();
                self.next_token(); // consume preset charset
                Literal::PresetCharSet(preset_charset_name)
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

    fn parse_charset_elements(&mut self) -> Result<Vec<CharSetElement>, AnreError> {
        // ```diagram
        // [ {char | char_range | preset_charset | char_set} ] ?
        // -                                                   -
        // ^                                                   ^__ to here
        // |__ current, validated
        // ```

        self.next_token(); // consume '['

        let mut elements = vec![];
        while let Some(token) = self.peek_token(0) {
            if token == &Token::BracketClose {
                break;
            }

            let start_range = *self.peek_range(0).unwrap();
            let expression = self.parse_expression()?;

            match expression {
                Expression::Literal(Literal::Char(c)) => {
                    // char
                    if self.peek_token_and_equals(0, &Token::Range) {
                        // character range, for example `['a'..'z']`
                        self.next_token(); // consume '..'

                        if self.peek_token(0).is_none() {
                            return Err(AnreError::UnexpectedEndOfDocument(
                                "Expected a character literal after '..' in a character range."
                                    .to_owned(),
                            ));
                        }

                        let end_range = *self.peek_range(0).unwrap();
                        let end_expression = self.parse_expression()?;

                        if let Expression::Literal(Literal::Char(end_char)) = end_expression {
                            let char_range = CharRange {
                                start: c,
                                end_inclusive: end_char,
                            };
                            elements.push(CharSetElement::CharRange(char_range));
                        } else {
                            let range = Range::merge(&end_range, &self.last_range);
                            return Err(AnreError::MessageWithRange(
                                "Expected a character literal.".to_string(),
                                range,
                            ));
                        }
                    } else {
                        elements.push(CharSetElement::Char(c));
                    }
                }
                Expression::Literal(Literal::PresetCharSet(preset_charset_name)) => {
                    // preset charset
                    elements.push(CharSetElement::PresetCharSet(preset_charset_name));
                }
                Expression::Literal(Literal::CharSet(char_set)) => {
                    // nested charset, for example `['_', ['a'..'f'], ['0'..'9']]`
                    elements.push(CharSetElement::CharSet(Box::new(char_set)));
                }
                _ => {
                    let range = Range::merge(&start_range, &self.last_range);
                    return Err(AnreError::MessageWithRange(
                        "Unsupported character set element.".to_string(),
                        range,
                    ));
                }
            }
        }

        self.consume_closing_bracket()?; // consume ']'
        Ok(elements)
    }
}

enum Repetition {
    Repeat(usize),
    RepeatFrom(usize),
    RepeatRange(usize, usize),
}

impl TryFrom<&str> for PresetCharSetName {
    type Error = ();

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        match name {
            "char_word" => Ok(Self::CharWord),
            "char_not_word" => Ok(Self::CharNotWord),
            "char_digit" => Ok(Self::CharDigit),
            "char_not_digit" => Ok(Self::CharNotDigit),
            "char_space" => Ok(Self::CharSpace),
            "char_not_space" => Ok(Self::CharNotSpace),
            _ => Err(()),
        }
    }
}

impl TryFrom<&str> for FunctionName {
    type Error = ();

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        match name {
            // Greedy Quantifier
            "optional" => Ok(Self::Optional),
            "one_or_more" => Ok(Self::OneOrMore),
            "zero_or_more" => Ok(Self::ZeroOrMore),
            "repeat" => Ok(Self::Repeat),
            "repeat_range" => Ok(Self::RepeatRange),
            "repeat_from" => Ok(Self::RepeatFrom),

            // Lazy Quantifier
            "lazy_optional" => Ok(Self::LazyOptional),
            "lazy_one_or_more" => Ok(Self::LazyOneOrMore),
            "lazy_zero_or_more" => Ok(Self::LazyZeroOrMore),
            "lazy_repeat_range" => Ok(Self::LazyRepeatRange),
            "lazy_repeat_from" => Ok(Self::LazyRepeatFrom),

            // Boundary Assertions
            "is_start" => Ok(Self::IsStart),
            "is_end" => Ok(Self::IsEnd),
            "is_bound" => Ok(Self::IsBound),
            "is_not_bound" => Ok(Self::IsNotBound),

            // Lookahead and Lookbehind Assertions
            "is_before" => Ok(Self::IsBefore),        // lookahead
            "is_after" => Ok(Self::IsAfter),          // lookbehind
            "is_not_before" => Ok(Self::IsNotBefore), // negative lookahead
            "is_not_after" => Ok(Self::IsNotAfter),   // negative lookbehind

            // Capturing Groups
            "index" => Ok(Self::Index),
            "name" => Ok(Self::Name),
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
        let program = parse_from_str(
            r#"
(char_any, 'a', "foo")
    "#,
        )
        .unwrap();

        assert_eq!(
            program,
            Program {
                expression: Expression::Group(vec![
                    Expression::Literal(Literal::AnyChar),
                    Expression::Literal(Literal::Char('a')),
                    Expression::Literal(Literal::String("foo".to_string())),
                ])
            }
        );

        assert_eq!(program.to_string(), r#"(char_any, 'a', "foo")"#);
    }

    #[test]
    fn test_parse_literal_preset_charset() {
        let program = parse_from_str(
            r#"
(
    char_word
    char_not_word
    char_digit
    char_not_digit
    char_space
    char_not_space
)"#,
        )
        .unwrap();

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
        let program = parse_from_str(
            r#"
['a', '0'..'9', char_word]
    "#,
        )
        .unwrap();

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
            parse_from_str(
                r#"
!['a'..'z', char_space]
    "#,
            )
            .unwrap()
            .to_string(),
            r#"!['a'..'z', char_space]"#
        );

        // nested charset
        assert_eq!(
            parse_from_str(
                r#"
['-', ['a'..'f'], ['0'..'9']]
    "#,
            )
            .unwrap()
            .to_string(),
            r#"['-', ['a'..'f'], ['0'..'9']]"#
        );
    }

    #[test]
    fn test_parse_function_call() {
        assert_eq!(
            parse_from_str(
                r#"
(
    optional('a')
    one_or_more('b')
    lazy_zero_or_more('c')
)
    "#,
            )
            .unwrap()
            .to_string(),
            r#"(optional('a'), one_or_more('b'), lazy_zero_or_more('c'))"#
        );

        // multiple args
        assert_eq!(
            parse_from_str(
                r#"
is_after("bar", "foo")
                "#,
            )
            .unwrap()
            .to_string(),
            r#"is_after("bar", "foo")"#
        );

        // numeric args
        assert_eq!(
            parse_from_str(
                r#"
(
    repeat('a' 3)
    repeat_range('b', 5, 7)
    repeat_from(
    'c' 11)
)
    "#,
            )
            .unwrap()
            .to_string(),
            r#"(repeat('a', 3), repeat_range('b', 5, 7), repeat_from('c', 11))"#
        );

        // nested
        assert_eq!(
            parse_from_str(r#"optional(one_or_more('a'))"#)
                .unwrap()
                .to_string(),
            r#"optional(one_or_more('a'))"#
        );
    }

    #[test]
    fn test_parse_method_call() {
        assert_eq!(
            parse_from_str(
                r#"
(
    'a'.optional()
    'b'.one_or_more()
    'c'.lazy_zero_or_more()
)
    "#,
            )
            .unwrap()
            .to_string(),
            r#"(optional('a'), one_or_more('b'), lazy_zero_or_more('c'))"#
        );

        // multiple args
        assert_eq!(
            parse_from_str(
                r#"
"bar".is_after("foo")
    "#,
            )
            .unwrap()
            .to_string(),
            r#"is_after("bar", "foo")"#
        );

        // numeric args
        assert_eq!(
            parse_from_str(
                r#"
(
    'a'.repeat(3)
    'b'.repeat_range(5, 7)
    'c'.repeat_from(11)
)
    "#,
            )
            .unwrap()
            .to_string(),
            r#"(repeat('a', 3), repeat_range('b', 5, 7), repeat_from('c', 11))"#
        );

        // chain method call
        assert_eq!(
            parse_from_str(
                r#"
'a'.one_or_more().optional()
    "#
            )
            .unwrap()
            .to_string(),
            r#"optional(one_or_more('a'))"#
        );
    }

    #[test]
    fn test_parse_quantifier() {
        assert_eq!(
            parse_from_str(
                r#"
(
    'a'?
    'b'+
    'c'*
    'x'??
    'y'+?
    'z'*?
)
    "#,
            )
            .unwrap()
            .to_string(),
            r#"(optional('a'), one_or_more('b'), zero_or_more('c'), lazy_optional('x'), lazy_one_or_more('y'), lazy_zero_or_more('z'))"#
        );

        // Combines function call and quantifier
        assert_eq!(
            parse_from_str(
                r#"
repeat('a',3)?
    "#,
            )
            .unwrap()
            .to_string(),
            r#"optional(repeat('a', 3))"#
        );

        // Combines method call and quantifier
        assert_eq!(
            parse_from_str(
                r#"
'a'.repeat(3)?
    "#,
            )
            .unwrap()
            .to_string(),
            r#"optional(repeat('a', 3))"#
        );
    }

    #[test]
    fn test_parse_repetition() {
        assert_eq!(
            parse_from_str(
                r#"
(
    'a'{3}
    'b'{5..7}
    'c'{11..}
    'x'{3}?
    'y'{5..7}?
    'z'{11..}?
)
    "#,
            )
            .unwrap()
            .to_string(),
            r#"(repeat('a', 3), repeat_range('b', 5, 7), repeat_from('c', 11), lazy_repeat('x', 3), lazy_repeat_range('y', 5, 7), lazy_repeat_from('z', 11))"#
        );

        // Special case: zero repetition and range with the same start and end
        assert_eq!(
            parse_from_str(
                r#"
(
    'a'{0}
    'b'{3..3}
    'c'{5..5}?
)
    "#,
            )
            .unwrap()
            .to_string(),
            r#"(repeat('a', 0), repeat_range('b', 3, 3), lazy_repeat_range('c', 5, 5))"#
        );

        // Combines function call and repetition
        assert_eq!(
            parse_from_str(
                r#"
repeat('a', 3){5..7}
    "#,
            )
            .unwrap()
            .to_string(),
            r#"repeat_range(repeat('a', 3), 5, 7)"#
        );

        // Combines method call and repetition
        assert_eq!(
            parse_from_str(
                r#"
'a'.repeat(3){5..7}
    "#,
            )
            .unwrap()
            .to_string(),
            r#"repeat_range(repeat('a', 3), 5, 7)"#
        );

        // err: invalid repetition range, `{m..n}` is invalid if m > n
        assert!(matches!(
            parse_from_str(r#"'a'{5..3}"#),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 3,
                        line: 0,
                        column: 3
                    },
                    end_inclusive: Position {
                        index: 8,
                        line: 0,
                        column: 8
                    }
                }
            ))
        ));
    }

    #[test]
    fn test_parse_index_capture_and_backreference() {
        assert_eq!(
            parse_from_str(
                r#"
#'a'
    "#,
            )
            .unwrap()
            .to_string(),
            r#"index('a')"#
        );

        assert_eq!(
            parse_from_str(
                r#"
#('a', char_digit)
    "#,
            )
            .unwrap()
            .to_string(),
            r#"index(('a', char_digit))"#
        );

        assert_eq!(
            parse_from_str(
                r#"
(#char_digit+ '.' ^1)
    "#,
            )
            .unwrap()
            .to_string(),
            r#"(index(one_or_more(char_digit)), '.', ^1)"#
        );

        // Operator `#` has lower precedence than method calls
        assert_eq!(
            parse_from_str(
                r#"
#'a'.one_or_more()
    "#,
            )
            .unwrap()
            .to_string(),
            r#"index(one_or_more('a'))"#
        );

        // Operator `#` has lower precedence than notations
        assert_eq!(
            parse_from_str(
                r#"
#'a'?
    "#,
            )
            .unwrap()
            .to_string(),
            r#"index(optional('a'))"#
        );

        // Operator `#` has lower precedence than method calls and notations
        assert_eq!(
            parse_from_str(
                r#"
#'a'.repeat(3)?
    "#,
            )
            .unwrap()
            .to_string(),
            r#"index(optional(repeat('a', 3)))"#
        );

        // Function style indexed capturing
        assert_eq!(
            parse_from_str(
                r#"
index('a')
    "#,
            )
            .unwrap()
            .to_string(),
            r#"index('a')"#
        );

        // Method-like indexed capturing
        assert_eq!(
            parse_from_str(
                r#"
char_digit.index()
    "#,
            )
            .unwrap()
            .to_string(),
            r#"index(char_digit)"#
        );
    }

    #[test]
    fn test_parse_name_capture_and_backreference() {
        assert_eq!(
            parse_from_str(
                r#"
'a' as x
    "#,
            )
            .unwrap()
            .to_string(),
            r#"name('a', x)"#
        );

        assert_eq!(
            parse_from_str(
                r#"
('a', char_digit) as x
    "#,
            )
            .unwrap()
            .to_string(),
            r#"name(('a', char_digit), x)"#
        );

        // named capturing implies indexed capturing
        assert_eq!(
            parse_from_str(
                r#"
#'a' as x
    "#,
            )
            .unwrap()
            .to_string(),
            r#"name('a', x)"#
        );

        // named capturing implies indexed capturing, another case
        assert_eq!(
            parse_from_str(
                r#"
#('a' as x)
    "#,
            )
            .unwrap()
            .to_string(),
            r#"name('a', x)"#
        );

        assert_eq!(
            parse_from_str(
                r#"
(char_digit as a, 'x', a)
    "#,
            )
            .unwrap()
            .to_string(),
            r#"(name(char_digit, a), 'x', a)"#
        );

        // Function style named capturing
        assert_eq!(
            parse_from_str(
                r#"
name('a', x)
    "#,
            )
            .unwrap()
            .to_string(),
            r#"name('a', x)"#
        );

        // Method-like named capturing
        assert_eq!(
            parse_from_str(
                r#"
char_digit.name(x)
    "#,
            )
            .unwrap()
            .to_string(),
            r#"name(char_digit, x)"#
        );
    }

    #[test]
    fn test_parse_logic_or() {
        {
            let program = parse_from_str(
                r#"
'a' || 'b'
    "#,
            )
            .unwrap();

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
            let program = parse_from_str(
                r#"
'a' || 'b' || 'c'
"#,
            )
            .unwrap();

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
            parse_from_str(r#"char_digit+ || [char_word, '-']+"#,)
                .unwrap()
                .to_string(),
            r#"one_or_more(char_digit) || one_or_more([char_word, '-'])"#
        );

        // group + logic or
        assert_eq!(
            parse_from_str(
                r#"
('a' || 'b') || 'c'
"#,
            )
            .unwrap()
            .to_string(),
            r#"('a' || 'b') || 'c'"#
        );

        // group + logic or + group
        assert_eq!(
            parse_from_str(
                r#"
('a', char_word) || ('b', char_digit)
"#,
            )
            .unwrap()
            .to_string(),
            r#"('a', char_word) || ('b', char_digit)"#
        );

        // string + logic or
        assert_eq!(
            parse_from_str(
                r#"
"ab" || "cd"
"#,
            )
            .unwrap()
            .to_string(),
            r#""ab" || "cd""#
        );
    }

    #[test]
    fn test_parse_group() {
        assert_eq!(
            parse_from_str(
                r#"
(
    ("foo", char_digit)
    ('b', ("bar", char_digit))
    is_end()
)
"#,
            )
            .unwrap()
            .to_string(),
            r#"(("foo", char_digit), ('b', ("bar", char_digit)), is_end())"#
        );

        // nested groups
        assert_eq!(
            parse_from_str(
                r#"
(
    repeat(("foo", char_digit), 3)
    ('b', repeat("bar", 5))
    is_end()
)
"#,
            )
            .unwrap()
            .to_string(),
            r#"(repeat(("foo", char_digit), 3), ('b', repeat("bar", 5)), is_end())"#
        );

        // escape nested group
        assert_eq!(
            parse_from_str(
                r#"
(((('a', char_digit, 'b'))))
"#,
            )
            .unwrap()
            .to_string(),
            r#"('a', char_digit, 'b')"#
        );
    }

    #[test]
    fn test_parse_operate_on_group() {
        // Quantifier (function style) on group
        assert_eq!(
            parse_from_str(
                r#"
(
    'a'
    ('b').optional()
    ('x', char_digit).optional()
)
"#,
            )
            .unwrap()
            .to_string(),
            r#"('a', optional('b'), optional(('x', char_digit)))"#
        );

        // Quantifier (notation style) on group
        assert_eq!(
            parse_from_str(
                r#"
(
    'a'
    ('b')?
    ('x', char_digit)?
)
"#,
            )
            .unwrap()
            .to_string(),
            r#"('a', optional('b'), optional(('x', char_digit)))"#
        );

        // Repetition on group
        assert_eq!(
            parse_from_str(
                r#"
(
    'a'
    ('b'){3..5}
    ('x', char_digit){7..11}
)
"#,
            )
            .unwrap()
            .to_string(),
            r#"('a', repeat_range('b', 3, 5), repeat_range(('x', char_digit), 7, 11))"#
        );

        // Quantifier on indexed capturing group
        assert_eq!(
            parse_from_str(
                r#"
(
    'a'
    #('b')?
    #('x', char_digit)?
)
"#,
            )
            .unwrap()
            .to_string(),
            r#"('a', index(optional('b')), index(optional(('x', char_digit))))"#
        );

        // Repetition on indexed capturing group
        assert_eq!(
            parse_from_str(
                r#"
(
    'a'
    #('b'){3..5}
    #('x', char_digit){7..11}
)
"#,
            )
            .unwrap()
            .to_string(),
            r#"('a', index(repeat_range('b', 3, 5)), index(repeat_range(('x', char_digit), 7, 11)))"#
        );

        // Quantifier on name capturing group
        assert_eq!(
            parse_from_str(
                r#"
(
    'a'
    ('b' as foo)?
    (('x', char_digit) as bar)?
)
"#,
            )
            .unwrap()
            .to_string(),
            r#"('a', optional(name('b', foo)), optional(name(('x', char_digit), bar)))"#
        );

        // Repetition on name capturing group
        assert_eq!(
            parse_from_str(
                r#"
(
    'a'
    ('b' as foo){3..5}
    (('x', char_digit) as bar){7..11}
)
"#,
            )
            .unwrap()
            .to_string(),
            r#"('a', repeat_range(name('b', foo), 3, 5), repeat_range(name(('x', char_digit), bar), 7, 11))"#
        );

        // Precedence of group and quantifier
        assert_eq!(
            parse_from_str(
                r#"
(
    (#'x')?
    #'y'?
)
"#,
            )
            .unwrap()
            .to_string(),
            r#"(optional(index('x')), index(optional('y')))"#
        );
    }

    #[test]
    fn test_parse_macro() {
        assert_eq!(
            parse_from_str(
                r#"
define A ("abc")

(is_start(), A, is_end())
"#,
            )
            .unwrap()
            .to_string(),
            r#"(is_start(), "abc", is_end())"#
        );

        assert_eq!(
            parse_from_str(
                r#"
define A ('a')
define B (A, 'b')
define C ([A, 'c'], optional(B), B.one_or_more())
define D (A || B || 'd')

(is_start(), A, B, C, D, is_end())
"#,
            )
            .unwrap()
            .to_string(),
            r#"(is_start(), 'a', ('a', 'b'), (['a', 'c'], optional(('a', 'b')), one_or_more(('a', 'b'))), 'a' || (('a', 'b') || 'd'), is_end())"#
        );
    }

    #[test]
    fn test_parse_examples() {
        assert_eq!(
            parse_from_str(
                r#"
/**
 * Decimal Numbers Regular Expression
 *
 * Examples:
 *
 * - "0"
 * - "123"
 */

char_digit.one_or_more()
"#,
            )
            .unwrap()
            .to_string(),
            "one_or_more(char_digit)"
        );

        assert_eq!(
            parse_from_str(
                r#"
/**
 * Hex Numbers Regular Expression
 *
 * Examples:
 *
 * - "0x0"
 * - "0x123"
 * - "0xabc"
 * - "0xDEADBEEF"
 */

(
    // The prefix "0x"
    "0x"

    // The hex digits
    ['0'..'9', 'a'..'f', 'A'..'F'].one_or_more()
)
"#,
            )
            .unwrap()
            .to_string(),
            "(\"0x\", one_or_more(['0'..'9', 'a'..'f', 'A'..'F']))"
        );

        assert_eq!(
            parse_from_str(
                r#"
/**
 * Email Address Validation Regular Expression
 *
 * Examples:
 *
 * - "abc@example.domain"
 * - "john-smith.new+mailbox-department@example.com"
 *
 * Ref:
 * https://en.wikipedia.org/wiki/Email_address
 */

(
    // Asserts that the current is the first character
    is_start()

    // User name
    [char_word, '.', '-'].one_or_more()

    // Sub-address
    ('+', [char_word, '-'].one_or_more()).optional()

    // The separator
    '@'

    // Domain name
    (
        ['a'..'z', 'A'..'Z', '0'..'9', '-'].one_or_more()
        '.'
    ).one_or_more()

    // Top-level domain
    ['a'..'z'].repeat_from(2)

    // Asserts that the current is the last character
    is_end()
)
"#,
            )
            .unwrap()
            .to_string(),
            "(is_start(), \
one_or_more([char_word, '.', '-']), \
optional(('+', one_or_more([char_word, '-']))), \
'@', \
one_or_more((one_or_more(['a'..'z', 'A'..'Z', '0'..'9', '-']), '.')), \
repeat_from(['a'..'z'], 2), \
is_end())"
        );

        let ipv4_regex = parse_from_str(
            r#"
/**
 * IPv4 Address Validation Regular Expression
 */

define num_25x ("25", ['0'..'5'])
define num_2xx ('2', ['0'..'4'], char_digit)
define num_1xx ('1', char_digit, char_digit)
define num_xx (['1'..'9'], char_digit)
define num_x (char_digit)
define part (num_25x || num_2xx || num_1xx || num_xx || num_x)

(is_start(), (part, '.').repeat(3), part, is_end())
"#,
        )
        .unwrap()
        .to_string();

        let part_str = r#"("25", ['0'..'5']) || (('2', ['0'..'4'], char_digit) || (('1', char_digit, char_digit) || ((['1'..'9'], char_digit) || char_digit)))"#;
        let expected_ipv4_regex = format!(
            "(is_start(), repeat(({}, '.'), 3), {}, is_end())",
            part_str, part_str
        );

        assert_eq!(ipv4_regex, expected_ipv4_regex);

        assert_eq!(
            parse_from_str(
                r#"
/**
 * Simple HTML Tag Regular Expression
 */

(
    '<'                                         // opening tag
    char_word+ as tag_name                      // tag name
    (                                           // attributes
        char_space,
        char_word+,                             // key
        ('=', '"', char_word+, '"').optional()  // value
    )*
    '>'
    char_any+?                                  // text content
    '<', '/', tag_name, '>'                     // closing tag
)
"#,
            )
            .unwrap()
            .to_string(),
            "(\
'<', \
name(one_or_more(char_word), tag_name), \
zero_or_more((char_space, one_or_more(char_word), optional(('=', '\\\"', one_or_more(char_word), '\\\"')))), \
'>', \
lazy_one_or_more(char_any), \
'<', '/', tag_name, '>'\
)"
        );
    }
}
