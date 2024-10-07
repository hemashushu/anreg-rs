// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::{
    ast::{
        CharRange, CharSet, CharSetElement, Expression, FunctionCall, FunctionCallArg,
        FunctionName, Literal, Program,
    },
    charposition::CharsWithPositionIter,
    commentcleaner::clean,
    error::Error,
    lexer::Lexer,
    location::Location,
    macroexpander::expand,
    normalizer::normalize,
    peekableiter::PeekableIter,
    token::{Token, TokenWithRange},
};

pub struct Parser<'a> {
    upstream: &'a mut PeekableIter<'a, TokenWithRange>,
    last_range: Location,
}

impl<'a> Parser<'a> {
    fn new(upstream: &'a mut PeekableIter<'a, TokenWithRange>) -> Self {
        Self {
            upstream,
            last_range: Location::new_range(0, 0, 0, 0, 0),
        }
    }

    fn next_token(&mut self) -> Option<Token> {
        match self.upstream.next() {
            Some(TokenWithRange { token, range }) => {
                self.last_range = range;
                Some(token)
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

    fn peek_token_and_equals(&self, offset: usize, expected_token: &Token) -> bool {
        matches!(
            self.upstream.peek(offset),
            Some(TokenWithRange { token, .. }) if token == expected_token)
    }

    fn peek_range(&self, offset: usize) -> Option<&Location> {
        match self.upstream.peek(offset) {
            Some(TokenWithRange { range, .. }) => Some(range),
            None => None,
        }
    }

    // consume '\n' if it exists.
    fn consume_new_line_if_exist(&mut self) -> bool {
        match self.peek_token(0) {
            Some(Token::NewLine) => {
                self.next_token();
                true
            }
            _ => false,
        }
    }

    // consume '\n' or ',' if they exist.
    fn consume_new_line_or_comma_if_exist(&mut self) -> bool {
        match self.peek_token(0) {
            Some(Token::NewLine | Token::Comma) => {
                self.next_token();
                true
            }
            _ => false,
        }
    }

    fn expect_token(&mut self, expected_token: &Token) -> Result<(), Error> {
        match self.next_token() {
            Some(token) => {
                if &token == expected_token {
                    Ok(())
                } else {
                    Err(Error::MessageWithLocation(
                        format!("Expect token: {}.", expected_token.get_description()),
                        self.last_range.get_position_by_range_start(),
                    ))
                }
            }
            None => Err(Error::UnexpectedEndOfDocument(format!(
                "Expect token: {}.",
                expected_token.get_description()
            ))),
        }
    }

    fn expect_new_line_or_comma(&mut self) -> Result<(), Error> {
        match self.peek_token(0) {
            Some(Token::NewLine | Token::Comma) => {
                self.next_token();
                Ok(())
            }
            Some(_) => Err(Error::MessageWithLocation(
                "Expect a comma or new-line.".to_owned(),
                self.peek_range(0).unwrap().get_position_by_range_start(),
            )),
            None => Err(Error::UnexpectedEndOfDocument(
                "Expect a comma or new-line.".to_owned(),
            )),
        }
    }

    fn expect_identifier(&mut self) -> Result<String, Error> {
        match self.peek_token(0) {
            Some(Token::Identifier(s)) => {
                let id = s.to_owned();
                self.next_token();
                Ok(id)
            }
            Some(_) => Err(Error::MessageWithLocation(
                "Expect an identifier.".to_owned(),
                self.last_range.get_position_by_range_start(),
            )),
            None => Err(Error::UnexpectedEndOfDocument(
                "Expect an identifier.".to_owned(),
            )),
        }
    }

    fn expect_number(&mut self) -> Result<u32, Error> {
        match self.peek_token(0) {
            Some(Token::Number(n)) => {
                let num = *n;
                self.next_token();
                Ok(num)
            }
            Some(_) => Err(Error::MessageWithLocation(
                "Expect a number.".to_owned(),
                self.last_range.get_position_by_range_start(),
            )),
            None => Err(Error::UnexpectedEndOfDocument(
                "Expect a number.".to_owned(),
            )),
        }
    }

    fn expect_char(&mut self) -> Result<char, Error> {
        match self.peek_token(0) {
            Some(Token::Char(c)) => {
                let ch = *c;
                self.next_token();
                Ok(ch)
            }
            Some(_) => Err(Error::MessageWithLocation(
                "Expect a char.".to_owned(),
                self.last_range.get_position_by_range_start(),
            )),
            None => Err(Error::UnexpectedEndOfDocument("Expect a char.".to_owned())),
        }
    }
}

impl<'a> Parser<'a> {
    pub fn parse_program(&mut self) -> Result<Program, Error> {
        // let mut definitions = vec![];
        let mut expressions = vec![];

        while let Some(token) = self.peek_token(0) {
            // match token {
            //     Token::Identifier(id) if id == "define" => {
            //         let definition = self.parse_definition_statement()?;
            //         definitions.push(definition);
            //     }
            //     _ => {
            //         let expression = self.parse_expression()?;
            //         expressions.push(expression);
            //     }
            // }

            let expression = self.parse_expression()?;
            expressions.push(expression);

            // consume separator
            let found_sep = self.consume_new_line_or_comma_if_exist();
            if !found_sep {
                break;
            }
        }

        let program = Program {
            // definitions,
            expressions,
        };

        Ok(program)
    }

    //     fn parse_definition_statement(&mut self) -> Result<Definition, Error> {
    //         // "define" "(" identifier "," expression ")" ?
    //         // --------                                -
    //         // ^                                       ^-- to here
    //         // | current, validated
    //
    //         self.next_token(); // consume "define"
    //         self.expect_token(&Token::LeftParen)?; // consume '('
    //         self.consume_new_line_if_exist(); // consume trailing new-line
    //
    //         let identifier = self.expect_identifier()?;
    //         self.expect_new_line_or_comma()?; // consume arg separator
    //
    //         let expression = self.parse_expression()?;
    //         self.consume_new_line_if_exist(); // consume trailing new-line
    //
    //         self.expect_token(&Token::RightParen)?; // consume ')'
    //
    //         Ok(Definition {
    //             expression: Box::new(expression),
    //             identifier,
    //         })
    //     }

    fn parse_expression(&mut self) -> Result<Expression, Error> {
        // token ...
        // -----
        // ^
        // | current, not None

        // the expression parsing order:
        // 1. binary expressions
        // 2. unary expressions
        // 3. base expression

        self.parse_alternation()
    }

    // binary expression
    fn parse_alternation(&mut self) -> Result<Expression, Error> {
        // token ... [ "||" expression ]
        // -----
        // ^
        // | current, not None

        let mut left = self.parse_simple_expression()?;

        while let Some(Token::LogicOr) = self.peek_token(0) {
            self.next_token(); // consume "||"
            self.consume_new_line_if_exist(); // consume trailing new-line

            // note:
            // using `parse_expression` for right-to-left precedence, e.g.
            // `let right = self.parse_expression()?;`
            // and using `parse_notation_and_rear_call` for
            // left-to-right precedence.

            let right = self.parse_simple_expression()?;
            let expression = Expression::Alternation(Box::new(left), Box::new(right));
            left = expression;
        }

        Ok(left)
    }

    // post unary expression
    fn parse_simple_expression(&mut self) -> Result<Expression, Error> {
        // token ...
        // -----
        // ^
        // | current, may be None

        self.parse_notation_and_rear_function_call()
    }

    fn parse_notation_and_rear_function_call(&mut self) -> Result<Expression, Error> {
        // token ... [ notation | "." identifier "("]
        // -----
        // ^
        // | current, may be None

        let mut left = self.parse_base_expression()?;

        while let Some(token) = self.peek_token(0) {
            match token {
                Token::Question
                | Token::Plus
                | Token::Asterisk
                | Token::QuestionLazy
                | Token::PlusLazy
                | Token::AsteriskLazy => {
                    let name = function_name_from_notation_token(&token, &self.last_range)?;
                    let function_call = FunctionCall {
                        name,
                        expression: Box::new(left),
                        args: vec![],
                    };
                    left = Expression::FunctionCall(Box::new(function_call));

                    self.next_token(); // consume notation
                }
                Token::LeftBrace => {
                    let (notation_quantifier, lazy) = self.continue_parse_notation_quantifier()?;

                    let mut args = vec![];

                    let name = match notation_quantifier {
                        NotationQuantifier::Repeat(n) => {
                            args.push(FunctionCallArg::Number(n));
                            if lazy {
                                FunctionName::RepeatLazy
                            } else {
                                FunctionName::Repeat
                            }
                        }
                        NotationQuantifier::RepeatRange(m, n) => {
                            args.push(FunctionCallArg::Number(m));
                            args.push(FunctionCallArg::Number(n));
                            if lazy {
                                FunctionName::RepeatRangeLazy
                            } else {
                                FunctionName::RepeatRange
                            }
                        }
                        NotationQuantifier::AtLeast(n) => {
                            args.push(FunctionCallArg::Number(n));
                            if lazy {
                                FunctionName::AtLeastLazy
                            } else {
                                FunctionName::AtLeast
                            }
                        }
                    };

                    let function_call = FunctionCall {
                        name,
                        expression: Box::new(left),
                        args,
                    };
                    left = Expression::FunctionCall(Box::new(function_call));
                }
                Token::Dot
                    if matches!(self.peek_token(1), Some(Token::Identifier(_)))
                        && self.peek_token_and_equals(2, &Token::LeftParen) =>
                {
                    let function_call = self.continue_parse_rear_function_call(left)?;
                    left = Expression::FunctionCall(Box::new(function_call));
                }
                _ => {
                    break;
                }
            }
        }

        Ok(left)
    }

    fn continue_parse_notation_quantifier(&mut self) -> Result<(NotationQuantifier, bool), Error> {
        // {m, n}? ?
        // -       -
        // ^       ^__ to here
        // | current, validated

        self.next_token(); // consume '{'
        self.consume_new_line_if_exist(); // consume trailing new-line

        let from = self.expect_number()?;

        // the comma that follows the first number is NOT a separator, it
        // can not be replaced by a newline like a normal comma,
        // its presence indicates that there is a second number, or that
        // the value of the second number is infinite.

        let (dual, to_optional) = if self.peek_token_and_equals(0, &Token::Comma) {
            // example:
            //
            // `{m,}` `{m,n}`
            //
            // ```
            // {
            //     m,
            // }
            // ```
            //
            // ```
            // {
            //     m, n
            // }
            // ```
            self.next_token(); // consume ','

            let to_optional = if let Some(Token::Number(to_ref)) = self.peek_token(0) {
                let to = *to_ref;
                self.next_token(); // consume number
                Some(to)
            } else {
                None
            };

            (true, to_optional)
        } else if self.peek_token_and_equals(0, &Token::NewLine)
            && matches!(self.peek_token(1), Some(Token::Number(_)))
        {
            // example:
            //
            // ```
            // {
            //     m
            //     n
            // }
            // ```

            self.next_token(); // consume new-line

            let to_optional = if let Some(Token::Number(to)) = self.next_token() {
                Some(to)
            } else {
                unreachable!()
            };

            (true, to_optional)
        } else {
            // example:
            //
            // `{m}`
            //
            // ```
            // {
            //     m
            // }
            // ```
            (false, None)
        };

        self.consume_new_line_if_exist();
        self.expect_token(&Token::RightBrace)?; // consume '}'

        let lazy = if self.peek_token_and_equals(0, &Token::Question) {
            self.next_token(); // consume trailing '?'
            true
        } else {
            false
        };

        let quantifier = if dual {
            if let Some(to) = to_optional {
                NotationQuantifier::RepeatRange(from, to)
            } else {
                NotationQuantifier::AtLeast(from)
            }
        } else {
            NotationQuantifier::Repeat(from)
        };

        Ok((quantifier, lazy))
    }

    fn continue_parse_rear_function_call(
        &mut self,
        expression: Expression,
    ) -> Result<FunctionCall, Error> {
        // "." identifier "(" {args} ")" ?
        // --- ---------- ---            -
        // ^   ^          ^__ validated  ^ to here
        // |   |__ validated
        // | current, validated

        self.next_token(); // consume '.'

        let name_string = self.expect_identifier()?; // consume function name
        let name = function_name_from_str(&name_string, &self.last_range)?;

        self.next_token(); // consume '('
        self.consume_new_line_if_exist(); // consume trailing new-line

        let mut args = vec![];

        while let Some(token) = self.peek_token(0) {
            if token == &Token::RightParen {
                break;
            }

            match token {
                Token::Number(num_ref) => {
                    let num = *num_ref;
                    self.next_token(); // consume number
                    args.push(FunctionCallArg::Number(num));
                }
                Token::Identifier(id_ref) => {
                    let id = id_ref.to_owned();
                    self.next_token(); // consume identifier
                    args.push(FunctionCallArg::Identifier(id));
                }
                _ => {
                    return Err(Error::MessageWithLocation(
                        "Unsupported argument value.".to_owned(),
                        self.last_range,
                    ));
                }
            }

            let found_sep = self.consume_new_line_or_comma_if_exist();
            if !found_sep {
                break;
            }
        }

        self.expect_token(&Token::RightParen)?; // consume ')'

        let function_call = FunctionCall {
            name,
            expression: Box::new(expression),
            args,
        };

        Ok(function_call)
    }

    fn parse_base_expression(&mut self) -> Result<Expression, Error> {
        // token ...
        // ---------
        // ^
        // | current, may be None

        // base expression:
        // - literal
        // - identifier
        // - group
        // - function call
        let expression = match self.peek_token(0) {
            Some(token) => {
                match token {
                    Token::LeftParen => {
                        // group
                        self.parse_group()?
                    }
                    Token::Identifier(_) if self.peek_token_and_equals(1, &Token::LeftParen) => {
                        // function call
                        self.parse_function_call()?
                    }
                    Token::Identifier(_) => {
                        // identifier
                        let id = self.expect_identifier()?;
                        Expression::Identifier(id)
                    }
                    _ => {
                        let literal = self.parse_literal()?;
                        Expression::Literal(literal)
                    }
                }
            }
            None => {
                return Err(Error::UnexpectedEndOfDocument(
                    "Expect an expression.".to_owned(),
                ));
            }
        };

        Ok(expression)
    }

    fn parse_group(&mut self) -> Result<Expression, Error> {
        // "(" {expression} ")" ?
        // ---                  -
        // ^                    ^-- to here
        // | current, validated

        self.expect_token(&Token::LeftParen)?; // consume "("
        self.consume_new_line_if_exist(); // consume trailing new-line

        let mut expressions: Vec<Expression> = vec![];

        while let Some(token) = self.peek_token(0) {
            if token == &Token::RightParen {
                break;
            }

            let expression = self.parse_expression()?;
            expressions.push(expression);

            let found_sep = self.consume_new_line_or_comma_if_exist();
            if !found_sep {
                break;
            }
        }

        self.expect_token(&Token::RightParen)?; // consume ")"

        Ok(Expression::Group(expressions))
    }

    fn parse_function_call(&mut self) -> Result<Expression, Error> {
        // identifier "(" expression ["," args... ] ")" ?
        // ---------- ---                               -
        // ^          ^__ validated                     ^__ to here
        // | current, validated

        let name_string = self.expect_identifier()?;
        let name = function_name_from_str(&name_string, &self.last_range)?;

        self.next_token(); // consume '('
        self.consume_new_line_if_exist(); // consume trailing new-line

        let expression = self.parse_expression()?;
        self.consume_new_line_or_comma_if_exist(); // consume trailing new-line

        let mut args = vec![];
        while let Some(token) = self.peek_token(0) {
            if token == &Token::RightParen {
                break;
            }

            match token {
                Token::Number(num_ref) => {
                    let num = *num_ref;
                    self.next_token(); // consume number
                    args.push(FunctionCallArg::Number(num));
                }
                Token::Identifier(id_ref) => {
                    let id = id_ref.to_owned();
                    self.next_token(); // consume identifier
                    args.push(FunctionCallArg::Identifier(id));
                }
                _ => {
                    return Err(Error::MessageWithLocation(
                        "Unsupported argument value.".to_owned(),
                        self.last_range,
                    ));
                }
            }

            let found_sep = self.consume_new_line_or_comma_if_exist();
            if !found_sep {
                break;
            }
        }

        self.expect_token(&Token::RightParen)?; // consume ')'

        let function_call = FunctionCall {
            name,
            expression: Box::new(expression),
            args,
        };

        Ok(Expression::FunctionCall(Box::new(function_call)))
    }

    fn parse_literal(&mut self) -> Result<Literal, Error> {
        // token ...
        // -----
        // ^
        // | current, not None

        // literal:
        //   - char
        //   - string
        //   - charset
        //   - preset_charset
        //   - symbol

        match self.peek_token(0) {
            Some(token) => {
                let literal = match token {
                    Token::LeftBracket => {
                        let elements = self.parse_charset()?;
                        Literal::CharSet(CharSet {
                            negative: false,
                            elements,
                        })
                    }
                    Token::Exclamation if self.peek_token_and_equals(1, &Token::LeftBracket) => {
                        // negative charset
                        self.next_token();

                        let elements = self.parse_charset()?;
                        Literal::CharSet(CharSet {
                            negative: true,
                            elements,
                        })
                    }
                    Token::Char(c_ref) => {
                        let c = *c_ref;
                        self.next_token(); // consume char
                        Literal::Char(c)
                    }
                    Token::String(string_ref) => {
                        let string = string_ref.to_owned();
                        self.next_token(); // consume string
                        Literal::String(string)
                    }
                    Token::PresetCharSet(preset_charset_ref) => {
                        let preset_charset = preset_charset_ref.to_owned();
                        self.next_token(); // consume preset charset
                        Literal::PresetCharSet(preset_charset)
                    }
                    Token::Symbol(symbol_ref) => {
                        let symbol = symbol_ref.to_owned();
                        self.next_token(); // consume symbol
                        Literal::Symbol(symbol)
                    }
                    _ => {
                        return Err(Error::MessageWithLocation(
                            "Expect a literal.".to_owned(),
                            self.last_range,
                        ));
                    }
                };

                Ok(literal)
            }
            None => {
                unreachable!()
            }
        }
    }

    fn parse_charset(&mut self) -> Result<Vec<CharSetElement>, Error> {
        // "[" {char | char_range | preset_charset | symbol} "]" ?
        // ---                                                   -
        // ^                                                     ^__ to here
        // | current, validated

        self.next_token(); // consume '['
        self.consume_new_line_if_exist(); // consume trailing new-line

        let mut elements = vec![];
        while let Some(token) = self.peek_token(0) {
            if token == &Token::RightBracket {
                break;
            }

            match token {
                Token::Char(_)
                    if self.peek_token_and_equals(1, &Token::Interval)
                        || (self.peek_token_and_equals(1, &Token::NewLine)
                            && self.peek_token_and_equals(2, &Token::Interval)) =>
                {
                    // char range
                    let char_range = self.parse_char_range()?;
                    elements.push(CharSetElement::CharRange(char_range));
                }
                Token::Char(c_ref) => {
                    // char
                    let c = *c_ref;
                    self.next_token(); // consume char
                    elements.push(CharSetElement::Char(c));
                }
                Token::PresetCharSet(preset_charset_ref) => {
                    // preset char set
                    let preset_charset = preset_charset_ref.to_owned();
                    self.next_token(); // consume preset charset
                    elements.push(CharSetElement::PresetCharSet(preset_charset));
                }
                Token::Symbol(symbol_ref) => {
                    // symbol, such as "start", "end", "bound"
                    let symbol = symbol_ref.to_owned();
                    self.next_token(); // consume symbol
                    elements.push(CharSetElement::Symbol(symbol));
                }
                _ => {
                    return Err(Error::MessageWithLocation(
                        "Unexpected char set element.".to_owned(),
                        self.last_range,
                    ));
                }
            }

            let found_sep = self.consume_new_line_or_comma_if_exist();
            if !found_sep {
                break;
            }
        }

        self.expect_token(&Token::RightBracket)?;

        Ok(elements)
    }

    fn parse_char_range(&mut self) -> Result<CharRange, Error> {
        // 'c' [new-line] '..' 'c' ?
        // ---  --------  ----     -
        // ^    ^         ^        ^__ to here
        // |    | vali..  | validated
        // | current, validated

        let char_start = self.expect_char()?; // consume start char
        self.consume_new_line_if_exist();

        self.next_token(); // consume '..'
        self.consume_new_line_if_exist();

        let char_end = self.expect_char()?; // consume end char

        Ok(CharRange {
            start: char_start,
            end_included: char_end,
        })
    }
}

enum NotationQuantifier {
    Repeat(u32),
    RepeatRange(u32, u32),
    AtLeast(u32),
}

fn function_name_from_str(name_str: &str, range: &Location) -> Result<FunctionName, Error> {
    let name = match name_str {
        // Greedy quantifier
        "optional" => FunctionName::Optional,
        "one_or_more" => FunctionName::OneOrMore,
        "zero_or_more" => FunctionName::ZeroOrMore,
        "repeat" => FunctionName::Repeat,
        "repeat_range" => FunctionName::RepeatRange,
        "at_least" => FunctionName::AtLeast,

        // Lazy quantifier
        "optional_lazy" => FunctionName::OptionalLazy,
        "one_or_more_lazy" => FunctionName::OneOrMoreLazy,
        "zero_or_more_lazy" => FunctionName::ZeroOrMoreLazy,
        "repeat_lazy" => FunctionName::RepeatLazy,
        "repeat_range_lazy" => FunctionName::RepeatRangeLazy,
        "at_least_lazy" => FunctionName::AtLeastLazy,

        // Assertions
        "is_before" => FunctionName::IsBefore, // lookahead
        "is_after" => FunctionName::IsAfter,   // lookbehind
        "is_not_before" => FunctionName::IsNotBefore, // negative lookahead
        "is_not_after" => FunctionName::IsNotAfter, // negative lookbehind

        // Others
        "name" => FunctionName::Name,
        // Unexpect
        _ => {
            return Err(Error::MessageWithLocation(
                format!("Unexpect function name: \"{}\"", name_str),
                range.to_owned(),
            ))
        }
    };

    Ok(name)
}

fn function_name_from_notation_token(
    token: &Token,
    range: &Location,
) -> Result<FunctionName, Error> {
    let name = match token {
        // Greedy quantifier
        Token::Question => FunctionName::Optional,
        Token::Plus => FunctionName::OneOrMore,
        Token::Asterisk => FunctionName::ZeroOrMore,

        // Lazy quantifier
        Token::QuestionLazy => FunctionName::OptionalLazy,
        Token::PlusLazy => FunctionName::OneOrMoreLazy,
        Token::AsteriskLazy => FunctionName::ZeroOrMoreLazy,

        // Unexpect
        _ => {
            return Err(Error::MessageWithLocation(
                "Expect a function name.".to_owned(),
                range.to_owned(),
            ))
        }
    };

    Ok(name)
}

pub fn parse_from_str(s: &str) -> Result<Program, Error> {
    let mut chars = s.chars();
    let mut char_position_iter = CharsWithPositionIter::new(0, &mut chars);
    let mut peekable_char_position_iter = PeekableIter::new(&mut char_position_iter, 3);
    let mut lexer = Lexer::new(&mut peekable_char_position_iter);
    let tokens = lexer.lex()?;
    let clean_tokens = clean(tokens);
    let normalized_tokens = normalize(clean_tokens);
    let expanded_tokens = expand(normalized_tokens)?;
    let final_tokens = normalize(expanded_tokens);
    let mut final_token_iter = final_tokens.into_iter();
    let mut peekable_final_token_iter = PeekableIter::new(&mut final_token_iter, 3);
    let mut parser = Parser::new(&mut peekable_final_token_iter);
    parser.parse_program()
}

#[cfg(test)]
mod tests {

    use pretty_assertions::assert_eq;

    use crate::ast::{CharRange, CharSet, CharSetElement, Expression, Literal, Program};

    use super::parse_from_str;

    #[test]
    fn test_parse_literal_simple() {
        let program = parse_from_str(
            r#"
start, 'a', "foo", char_word
    "#,
        )
        .unwrap();

        assert_eq!(
            program,
            Program {
                // definitions: vec![],
                expressions: vec![
                    Expression::Literal(Literal::Symbol("start".to_owned())),
                    Expression::Literal(Literal::Char('a')),
                    Expression::Literal(Literal::String("foo".to_owned())),
                    Expression::Literal(Literal::PresetCharSet("char_word".to_owned())),
                ]
            }
        );

        assert_eq!(program.to_string(), r#"start, 'a', "foo", char_word"#);
    }

    #[test]
    fn test_parse_literal_charset() {
        let program = parse_from_str(
            r#"
['a', '0'..'9', char_word, end]
    "#,
        )
        .unwrap();

        assert_eq!(
            program,
            Program {
                // definitions: vec![],
                expressions: vec![Expression::Literal(Literal::CharSet(CharSet {
                    negative: false,
                    elements: vec![
                        CharSetElement::Char('a'),
                        CharSetElement::CharRange(CharRange {
                            start: '0',
                            end_included: '9'
                        }),
                        CharSetElement::PresetCharSet("char_word".to_owned()),
                        CharSetElement::Symbol("end".to_owned())
                    ]
                })),]
            }
        );

        assert_eq!(program.to_string(), r#"['a', '0'..'9', char_word, end]"#);

        // negative
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

        // multiline
        assert_eq!(
            parse_from_str(
                r#"
[
    'a'
    '0'
    ..
    '9'
    char_word
    end
]"#,
            )
            .unwrap()
            .to_string(),
            r#"['a', '0'..'9', char_word, end]"#
        );

        // multiline with comma
        assert_eq!(
            parse_from_str(
                r#"
[
    'a',
    '0'
    ..
    '9',
    char_word,
    end,
]"#,
            )
            .unwrap()
            .to_string(),
            r#"['a', '0'..'9', char_word, end]"#
        );
    }

    #[test]
    fn test_parse_expression_function_call() {
        assert_eq!(
            parse_from_str(
                r#"
optional('a')
one_or_more('b')
zero_or_more_lazy('c')
name("xyz", prefix)
    "#,
            )
            .unwrap()
            .to_string(),
            r#"optional('a')
one_or_more('b')
zero_or_more_lazy('c')
name("xyz", prefix)"#
        );

        assert_eq!(
            parse_from_str(
                r#"
repeat(
    'a'
    3
)
repeat_range(
    'b'
    5
    7
)
at_least('c'
    11)
    "#,
            )
            .unwrap()
            .to_string(),
            r#"repeat('a', 3)
repeat_range('b', 5, 7)
at_least('c', 11)"#
        );
    }

    #[test]
    fn test_parse_expression_function_call_rear() {
        assert_eq!(
            parse_from_str(
                r#"
'a'.optional()
'b'.one_or_more()
'c'.zero_or_more_lazy()
"xyz".name(prefix)
    "#,
            )
            .unwrap()
            .to_string(),
            r#"optional('a')
one_or_more('b')
zero_or_more_lazy('c')
name("xyz", prefix)"#
        );

        assert_eq!(
            parse_from_str(
                r#"
'a'.repeat(3)
'b'.repeat_range(
    5
    7
)
'c'.at_least(11
)
    "#,
            )
            .unwrap()
            .to_string(),
            r#"repeat('a', 3)
repeat_range('b', 5, 7)
at_least('c', 11)"#
        );
    }

    #[test]
    fn test_parse_expression_notations() {
        assert_eq!(
            parse_from_str(
                r#"
'a'?
'b'+
'c'*
'x'??
'y'+?
'z'*?
    "#,
            )
            .unwrap()
            .to_string(),
            r#"optional('a')
one_or_more('b')
zero_or_more('c')
optional_lazy('x')
one_or_more_lazy('y')
zero_or_more_lazy('z')"#
        );

        assert_eq!(
            parse_from_str(
                r#"
'a'{3}
'b'{5,7}
'c'{11,}
'x'{3}?
'y'{5,7}?
'z'{11,}?
    "#,
            )
            .unwrap()
            .to_string(),
            r#"repeat('a', 3)
repeat_range('b', 5, 7)
at_least('c', 11)
repeat_lazy('x', 3)
repeat_range_lazy('y', 5, 7)
at_least_lazy('z', 11)"#
        );
    }

    #[test]
    fn test_parse_expression_alternation() {
        let program = parse_from_str(
            r#"
'a' || 'b' || 'c'
"#,
        )
        .unwrap();

        assert_eq!(
            program,
            Program {
                // definitions: vec![],
                expressions: vec![Expression::Alternation(
                    Box::new(Expression::Alternation(
                        Box::new(Expression::Literal(Literal::Char('a'))),
                        Box::new(Expression::Literal(Literal::Char('b'))),
                    )),
                    Box::new(Expression::Literal(Literal::Char('c')))
                )]
            }
        );

        assert_eq!(program.to_string(), r#"'a' || 'b' || 'c'"#);

        assert_eq!(
            parse_from_str(
                r#"
char_digit.one_or_more() || [char_word, '-']+
"#,
            )
            .unwrap()
            .to_string(),
            r#"one_or_more(char_digit) || one_or_more([char_word, '-'])"#
        );
    }

    #[test]
    fn test_parse_expression_group() {
        assert_eq!(
            parse_from_str(
                r#"
('a', "foo", char_digit)
('b', ("bar", char_digit), end)
"#,
            )
            .unwrap()
            .to_string(),
            r#"('a', "foo", char_digit), ('b', ("bar", char_digit), end)"#
        );

        assert_eq!(
            parse_from_str(
                r#"
repeat(('a', "foo", char_digit), 3)
('b', repeat("bar", 5), end)
"#,
            )
            .unwrap()
            .to_string(),
            r#"repeat(('a', "foo", char_digit), 3)
('b', repeat("bar", 5), end)"#
        );

        assert_eq!(
            parse_from_str(
                r#"
'a' || ('b' || 'c')
"#,
            )
            .unwrap()
            .to_string(),
            r#"'a' || ('b' || 'c')"#
        );
    }

    #[test]
    fn test_parse_macro() {
        assert_eq!(
            parse_from_str(
                r#"
define(a, "abc")
start, a, end
"#,
            )
            .unwrap()
            .to_string(),
            r#"start, "abc", end"#
        );

        assert_eq!(
            parse_from_str(
                r#"
define(a, 'a')
define(b, (a, 'b'))
define(c, ([a, 'c'], optional(b), b.one_or_more()))
define(d, (a || b || 'd'))
start, a, b, c, d, end
"#,
            )
            .unwrap()
            .to_string(),
            r#"start, 'a', ('a', 'b'), (['a', 'c'], optional(('a', 'b')), one_or_more(('a', 'b'))), ('a' || ('a', 'b') || 'd'), end"#
        );
    }

    #[test]
    fn test_parse_examples() {
        assert_eq!(
            parse_from_str(
                r#"
/**
 * Decimal Numbers Regular Expression
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
 */

// The prefix "0x"
"0x"

// The hex digits
['0'..'9', 'a'..'f'].one_or_more()
"#,
            )
            .unwrap()
            .to_string(),
            "\"0x\"
one_or_more(['0'..'9', 'a'..'f'])"
        );

        assert_eq!(
            parse_from_str(
                r#"
/**
 * Email Address Validated Regular Expression
 *
 * Ref:
 * https://en.wikipedia.org/wiki/Email_address
 */

start

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
['a'..'z'].at_least(2)

end
"#,
            )
            .unwrap()
            .to_string(),
            "start
one_or_more([char_word, '.', '-'])
optional(('+', one_or_more([char_word, '-'])))
'@'
one_or_more((one_or_more(['a'..'z', 'A'..'Z', '0'..'9', '-']), '.'))
at_least(['a'..'z'], 2)
end"
        );

        assert_eq!(
            parse_from_str(
                r#"
/**
 * IPv4 Address Validated Regular Expression
 */
define(num_25x, ("25", ['0'..'5']))
define(num_2xx, ('2', ['0'..'4'], char_digit))
define(num_1xx, ('1', char_digit, char_digit))
define(num_xx, (['1'..'9'], char_digit))
define(num_x, char_digit)
define(ip_num, (num_25x || num_2xx || num_1xx || num_xx || num_x))

start, (ip_num, '.').repeat(3), ip_num, end
"#,
            )
            .unwrap()
            .to_string(),
            "start
repeat((((\"25\", ['0'..'5']) || ('2', ['0'..'4'], char_digit) || ('1', char_digit, char_digit) || (['1'..'9'], char_digit) || char_digit), '.'), 3)
((\"25\", ['0'..'5']) || ('2', ['0'..'4'], char_digit) || ('1', char_digit, char_digit) || (['1'..'9'], char_digit) || char_digit), end"
        );

        assert_eq!(
            parse_from_str(
                r#"
/**
 * Simple HTML tag Regular Expression
 */
'<'                                                     // opening tag
name(char_word+, tag_name)                              // tag name
(char_space, char_word+, '=', '"', char_word+, '"')*    // attributes
'>'
char_any+?                                              // text content
'<', '/', tag_name, '>'                                 // closing tag
"#,
            )
            .unwrap()
            .to_string(),
            "'<'
name(one_or_more(char_word), tag_name)
zero_or_more((char_space, one_or_more(char_word), '=', '\"', one_or_more(char_word), '\"'))
'>'
one_or_more_lazy(char_any)
'<', '/', tag_name, '>'"
        );


    }
}
