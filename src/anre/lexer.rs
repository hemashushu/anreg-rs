// Copyright (c) 2026 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

use crate::{
    char_with_position::{CharWithPosition, CharsWithPositionIter},
    error::AnreError,
    peekable_iter::PeekableIter,
    position::Position,
    range::Range,
};

use super::token::{Token, TokenWithRange};

pub fn lex_from_str(s: &str) -> Result<Vec<TokenWithRange>, AnreError> {
    let mut chars = s.chars();
    let mut char_position_iter = CharsWithPositionIter::new(&mut chars);
    let mut peekable_char_position_iter = PeekableIter::new(&mut char_position_iter);
    let mut lexer = Lexer::new(&mut peekable_char_position_iter);
    lexer.lex()
}

struct Lexer<'a> {
    upstream: &'a mut PeekableIter<'a, CharWithPosition>,

    // Position of the most recently consumed character.
    last_position: Position,

    // Temporary start positions used while building token ranges.
    position_stack: Vec<Position>,
}

impl<'a> Lexer<'a> {
    fn new(upstream: &'a mut PeekableIter<'a, CharWithPosition>) -> Self {
        Self {
            upstream,
            last_position: Position::default(),
            position_stack: vec![],
        }
    }

    fn next_char(&mut self) -> Option<char> {
        match self.upstream.next() {
            Some(CharWithPosition {
                character,
                position,
            }) => {
                self.last_position = position;
                Some(character)
            }
            None => None,
        }
    }

    fn peek_char(&self, offset: usize) -> Option<&char> {
        match self.upstream.peek(offset) {
            Some(CharWithPosition { character, .. }) => Some(character),
            None => None,
        }
    }

    fn peek_position(&self, offset: usize) -> Option<&Position> {
        match self.upstream.peek(offset) {
            Some(CharWithPosition { position, .. }) => Some(position),
            None => None,
        }
    }

    /// Returns `true` when the character at `offset` matches `expected_char`.
    fn peek_char_and_equals(&self, offset: usize, expected_char: char) -> bool {
        matches!(
            self.upstream.peek(offset),
            Some(CharWithPosition { character, .. }) if character == &expected_char)
    }

    /// Pushes the position of the character most recently consumed by `next_char()`.
    fn push_last_position_into_stack(&mut self) {
        self.position_stack.push(self.last_position);
    }

    /// Pushes the current input position without consuming a character.
    fn push_peek_position_into_stack(&mut self) {
        let position = *self.peek_position(0).unwrap();
        self.position_stack.push(position);
    }

    /// Pops a previously saved token start position.
    fn pop_position_from_stack(&mut self) -> Position {
        self.position_stack.pop().unwrap()
    }

    fn consume_char_and_assert(
        &mut self,
        expected_char: char,
        char_description: &str,
    ) -> Result<(), AnreError> {
        match self.next_char() {
            Some(ch) => {
                if ch == expected_char {
                    Ok(())
                } else {
                    Err(AnreError::MessageWithPosition(
                        format!("Expected character {}.", char_description),
                        self.last_position,
                    ))
                }
            }
            None => Err(AnreError::UnexpectedEndOfDocument(format!(
                "Expected character {}.",
                char_description
            ))),
        }
    }
}

impl Lexer<'_> {
    fn lex(&mut self) -> Result<Vec<TokenWithRange>, AnreError> {
        let mut token_with_ranges = vec![];

        while let Some(current_char) = self.peek_char(0) {
            match current_char {
                ' ' | '\t' => {
                    // Spaces and tabs are separators, not tokens.
                    self.next_char();
                }
                '\r' if self.peek_char_and_equals(1, '\n') => {
                    // Skip a Windows newline.
                    self.next_char(); // consume '\r'
                    self.next_char(); // consume '\n'
                }
                '\n' => {
                    // Skip a Unix newline.
                    self.next_char(); // consume '\n'
                }
                ',' => {
                    // Commas are treated like whitespace in ANRE source.
                    self.next_char();
                }
                '|' if self.peek_char_and_equals(1, '|') => {
                    self.push_peek_position_into_stack();

                    self.next_char(); // consume '|'
                    self.next_char(); // consume '|'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::LogicOr,
                        Range::new(&self.pop_position_from_stack(), &self.last_position),
                    ));
                }
                '!' => {
                    self.next_char(); // consume '!'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::Not,
                        Range::from_single_position(&self.last_position),
                    ));
                }
                '.' if self.peek_char_and_equals(1, '.') => {
                    self.push_peek_position_into_stack();

                    self.next_char(); // consume '.'
                    self.next_char(); // consume '.'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::Range,
                        Range::new(&self.pop_position_from_stack(), &self.last_position),
                    ));
                }
                '.' => {
                    self.next_char(); // consume '.'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::Dot,
                        Range::from_single_position(&self.last_position),
                    ));
                }
                '#' => {
                    self.next_char(); // consume '#'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::Hash,
                        Range::from_single_position(&self.last_position),
                    ));
                }
                '^' => {
                    self.next_char(); // consume '^'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::Caret,
                        Range::from_single_position(&self.last_position),
                    ));
                }
                '[' => {
                    self.next_char(); // consume '['

                    token_with_ranges.push(TokenWithRange::new(
                        Token::BracketOpen,
                        Range::from_single_position(&self.last_position),
                    ));
                }
                ']' => {
                    self.next_char(); // consume ']'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::BracketClose,
                        Range::from_single_position(&self.last_position),
                    ));
                }
                '(' => {
                    self.next_char(); // consume '('

                    token_with_ranges.push(TokenWithRange::new(
                        Token::ParenthesisOpen,
                        Range::from_single_position(&self.last_position),
                    ));
                }
                ')' => {
                    self.next_char(); // consume ')'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::ParenthesisClose,
                        Range::from_single_position(&self.last_position),
                    ));
                }
                '?' if self.peek_char_and_equals(1, '?') => {
                    self.push_peek_position_into_stack();

                    self.next_char(); // consume '?'
                    self.next_char(); // consume '?'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::LazyOptional,
                        Range::new(&self.pop_position_from_stack(), &self.last_position),
                    ));
                }
                '?' => {
                    self.next_char(); // consume '?'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::Optional,
                        Range::from_single_position(&self.last_position),
                    ));
                }
                '+' if self.peek_char_and_equals(1, '?') => {
                    self.push_peek_position_into_stack();

                    self.next_char(); // consume '+'
                    self.next_char(); // consume '?'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::LazyOneOrMore,
                        Range::new(&self.pop_position_from_stack(), &self.last_position),
                    ));
                }
                '+' => {
                    self.next_char(); // consume '+'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::OneOrMore,
                        Range::from_single_position(&self.last_position),
                    ));
                }
                '*' if self.peek_char_and_equals(1, '?') => {
                    self.push_peek_position_into_stack();

                    self.next_char(); // consume '*'
                    self.next_char(); // consume '?'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::LazyZeroOrMore,
                        Range::new(&self.pop_position_from_stack(), &self.last_position),
                    ));
                }
                '*' => {
                    self.next_char(); // consume '*'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::ZeroOrMore,
                        Range::from_single_position(&self.last_position),
                    ));
                }
                '{' => {
                    self.next_char(); // consume '{'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::BraceOpen,
                        Range::from_single_position(&self.last_position),
                    ));
                }
                '}' => {
                    self.next_char(); // consume '}'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::BraceClose,
                        Range::from_single_position(&self.last_position),
                    ));
                }
                '0'..='9' => {
                    // number
                    token_with_ranges.push(self.lex_number_decimal()?);
                }
                '"' => {
                    // string
                    token_with_ranges.push(self.lex_string()?);
                }
                '\'' => {
                    // char
                    token_with_ranges.push(self.lex_char()?);
                }
                '/' if self.peek_char_and_equals(1, '/') => {
                    // line comment
                    self.lex_line_comment();
                }
                '/' if self.peek_char_and_equals(1, '*') => {
                    // block comment
                    self.lex_block_comment()?;
                }
                'a'..='z' | 'A'..='Z' | '_' | '\u{a0}'..='\u{d7ff}' | '\u{e000}'..='\u{10ffff}' => {
                    // identifier and keyword
                    token_with_ranges.push(self.lex_identifier()?);
                }
                current_char => {
                    return Err(AnreError::MessageWithPosition(
                        format!("Unexpected character '{}'.", current_char),
                        *self.peek_position(0).unwrap(),
                    ));
                }
            }
        }

        Ok(token_with_ranges)
    }

    fn lex_identifier(&mut self) -> Result<TokenWithRange, AnreError> {
        // ```diagram
        // key_nameT  //
        // ^       ^__// to here
        // |__________// current char, validated
        //
        // T = terminator chars || EOF
        // ```

        let mut identifier_buffer = String::new();

        self.push_peek_position_into_stack();

        while let Some(current_char) = self.peek_char(0) {
            match current_char {
                '0'..='9' | 'a'..='z' | 'A'..='Z' | '_' => {
                    identifier_buffer.push(*current_char);
                    self.next_char(); // consume char
                }
                '\u{a0}'..='\u{d7ff}' | '\u{e000}'..='\u{10ffff}' => {
                    // A char is a ‘Unicode scalar value’, which is any ‘Unicode code point’ other than a surrogate code point.
                    // This has a fixed numerical definition: code points are in the range 0 to 0x10FFFF,
                    // inclusive. Surrogate code points, used by UTF-16, are in the range 0xD800 to 0xDFFF.
                    //
                    // check out:
                    // https://doc.rust-lang.org/std/primitive.char.html
                    //
                    // CJK chars: '\u{4e00}'..='\u{9fff}'
                    // for complete CJK chars, check out Unicode standard
                    // Ch. 18.1 Han CJK Unified Ideographs
                    //
                    // | Block                                   | LocRange    | Comment |
                    // |-----------------------------------------|-------------|---------|
                    // | CJK Unified Ideographs                  | 4E00–9FFF   | Common                         |
                    // | CJK Unified Ideographs Extension A      | 3400–4DBF   | Rare                           |
                    // | CJK Unified Ideographs Extension B      | 20000–2A6DF | Rare, historic                 |
                    // | CJK Unified Ideographs Extension C      | 2A700–2B73F | Rare, historic                 |
                    // | CJK Unified Ideographs Extension D      | 2B740–2B81F | Uncommon, some in current use  |
                    // | CJK Unified Ideographs Extension E      | 2B820–2CEAF | Rare, historic                 |
                    // | CJK Unified Ideographs Extension F      | 2CEB0–2EBEF | Rare, historic                 |
                    // | CJK Unified Ideographs Extension G      | 30000–3134F | Rare, historic                 |
                    // | CJK Unified Ideographs Extension H      | 31350–323AF | Rare, historic                 |
                    // | CJK Compatibility Ideographs            | F900–FAFF   | Duplicates, unifiable variants, corporate characters |
                    // | CJK Compatibility Ideographs Supplement | 2F800–2FA1F | Unifiable variants             |
                    //
                    // see also:
                    //
                    // - https://www.unicode.org/versions/Unicode15.0.0/ch18.pdf
                    // - https://en.wikipedia.org/wiki/CJK_Unified_Ideographs
                    // - https://www.unicode.org/versions/Unicode15.0.0/
                    // - https://www.unicode.org/reports/tr31/tr31-37.html

                    identifier_buffer.push(*current_char);
                    self.next_char(); // consume char
                }
                ' ' | '\t' | '\r' | '\n' | ',' | '|' | '!' | '[' | ']' | '(' | ')' | '/' | '\''
                | '"' | '.' | '#' | '^' | '?' | '+' | '*' | '{' | '}' => {
                    // terminator chars
                    break;
                }
                _ => {
                    return Err(AnreError::MessageWithPosition(
                        format!("Invalid character '{}' in identifier.", current_char),
                        *self.peek_position(0).unwrap(),
                    ));
                }
            }
        }

        let identifier_range = Range::new(&self.pop_position_from_stack(), &self.last_position);

        let token = match identifier_buffer.as_str() {
            "define" | "as" => Token::Keyword(identifier_buffer),
            _ => Token::Identifier(identifier_buffer),
        };

        Ok(TokenWithRange::new(token, identifier_range))
    }

    fn lex_number_decimal(&mut self) -> Result<TokenWithRange, AnreError> {
        // ```diagram
        // 123456T  //
        // ^     ^__// to here
        // |________// current char, validated
        //
        // T = terminator chars || EOF
        // ```

        let mut num_buffer = String::new();

        self.push_peek_position_into_stack();

        while let Some(current_char) = self.peek_char(0) {
            match current_char {
                '0'..='9' => {
                    // valid digits for decimal number
                    num_buffer.push(*current_char);
                    self.next_char(); // consume digit
                }
                '_' => {
                    self.next_char(); // consume '_'
                }
                ' ' | '\t' | '\r' | '\n' | ',' | '|' | '!' | '[' | ']' | '(' | ')' | '/' | '\''
                | '"' | '.' | '#' | '^' | '?' | '+' | '*' | '{' | '}' => {
                    // terminator chars
                    break;
                }
                _ => {
                    return Err(AnreError::MessageWithPosition(
                        format!("Invalid digit '{}' for decimal number.", current_char),
                        *self.peek_position(0).unwrap(),
                    ));
                }
            }
        }

        let number_range = Range::new(&self.pop_position_from_stack(), &self.last_position);

        let v = num_buffer.parse::<usize>().map_err(|_| {
            AnreError::MessageWithRange(
                format!("Cannot convert \"{}\" to an integer.", num_buffer),
                number_range,
            )
        })?;

        let number_token = Token::Number(v);

        Ok(TokenWithRange::new(number_token, number_range))
    }

    fn lex_char(&mut self) -> Result<TokenWithRange, AnreError> {
        // ```diagram
        // 'a'?  //
        // ^  ^__// to here
        // |_____// current char, validated
        // ```

        self.push_peek_position_into_stack();

        self.next_char(); // consume `'`

        let character = match self.next_char() {
            Some(current_char) => {
                match current_char {
                    '\\' => {
                        // Parse a backslash escape inside the char literal.
                        match self.next_char() {
                            Some(escape_type) => {
                                match escape_type {
                                    '\\' => '\\',
                                    '\'' => '\'',
                                    '"' => {
                                        // `"` is accepted for consistency with string literals.
                                        '"'
                                    }
                                    't' => {
                                        // horizontal tabulation
                                        '\t'
                                    }
                                    'r' => {
                                        // carriage return (CR, ascii 13)
                                        '\r'
                                    }
                                    'n' => {
                                        // new line character (line feed, LF, ascii 10)
                                        '\n'
                                    }
                                    '0' => {
                                        // null char
                                        '\0'
                                    }
                                    'u' => {
                                        if self.peek_char_and_equals(0, '{') {
                                            // Unicode code point, for example '\u{2d}', '\u{6587}'
                                            self.unescape_unicode_code_point()?
                                        } else {
                                            return Err(AnreError::MessageWithPosition(
                                                "Missing opening brace for Unicode escape sequence."
                                                    .to_owned(),
                                                self.last_position,
                                            ));
                                        }
                                    }
                                    _ => {
                                        return Err(AnreError::MessageWithRange(
                                            format!("Unsupported escape char '{}'.", escape_type),
                                            Range::new(
                                                &self.pop_position_from_stack(),
                                                &self.last_position,
                                            ),
                                        ));
                                    }
                                }
                            }
                            None => {
                                // `\` + EOF
                                return Err(AnreError::UnexpectedEndOfDocument(
                                    "Incomplete escape sequence.".to_string(),
                                ));
                            }
                        }
                    }
                    '\'' => {
                        // `''`
                        return Err(AnreError::MessageWithRange(
                            "Empty character literal.".to_string(),
                            Range::new(&self.pop_position_from_stack(), &self.last_position),
                        ));
                    }
                    _ => {
                        // ordinary char
                        current_char
                    }
                }
            }
            None => {
                // `'EOF`
                return Err(AnreError::UnexpectedEndOfDocument(
                    "Incomplete character literal.".to_string(),
                ));
            }
        };

        // The literal must end with a closing single quote.
        match self.next_char() {
            Some('\'') => {
                // Ok
            }
            Some(_) => {
                // `'a?`
                return Err(AnreError::MessageWithPosition(
                    "Expected a closing quote for a character literal.".to_string(),
                    self.last_position,
                ));
            }
            None => {
                // `'aEOF`
                return Err(AnreError::UnexpectedEndOfDocument(
                    "Incomplete character literal.".to_string(),
                ));
            }
        }

        let character_range = Range::new(&self.pop_position_from_stack(), &self.last_position);
        Ok(TokenWithRange::new(Token::Char(character), character_range))
    }

    fn unescape_unicode_code_point(&mut self) -> Result<char, AnreError> {
        // ```diagram
        // \u{6587}?  //
        //   ^     ^__// to here
        //   |________// current char, validated
        // ```

        self.push_peek_position_into_stack();

        self.next_char(); // consume '{'

        let mut codepoint_buffer = String::new();

        loop {
            match self.peek_char(0) {
                Some(current_char) => match current_char {
                    '}' => break,
                    '0'..='9' | 'a'..='f' | 'A'..='F' => {
                        codepoint_buffer.push(*current_char);
                        self.next_char(); // consume char
                    }
                    _ => {
                        return Err(AnreError::MessageWithPosition(
                            format!(
                                "Invalid character '{}' in Unicode escape sequence.",
                                current_char
                            ),
                            *self.peek_position(0).unwrap(),
                        ));
                    }
                },
                None => {
                    // EOF
                    return Err(AnreError::UnexpectedEndOfDocument(
                        "Incomplete unicode escape sequence.".to_string(),
                    ));
                }
            }

            if codepoint_buffer.len() > 6 {
                break;
            }
        }

        if codepoint_buffer.len() > 6 {
            return Err(AnreError::MessageWithRange(
                "Unicode code point exceeds six digits.".to_string(),
                Range::new(&self.position_stack.pop().unwrap(), &self.last_position),
            ));
        }

        self.consume_char_and_assert('}', "closing brace for unicode escape sequence")?;

        let codepoint_range = Range::new(&self.pop_position_from_stack(), &self.last_position);

        if codepoint_buffer.is_empty() {
            return Err(AnreError::MessageWithRange(
                "Unicode escape sequence has an empty code point.".to_string(),
                codepoint_range,
            ));
        }

        let codepoint = u32::from_str_radix(&codepoint_buffer, 16).unwrap();

        if let Some(c) = char::from_u32(codepoint) {
            // valid code point:
            // 0 to 0x10FFFF, inclusive
            //
            // ref:
            // https://doc.rust-lang.org/std/primitive.char.html
            Ok(c)
        } else {
            Err(AnreError::MessageWithRange(
                "Invalid Unicode code point.".to_string(),
                codepoint_range,
            ))
        }
    }

    fn lex_string(&mut self) -> Result<TokenWithRange, AnreError> {
        // ```diagram
        // "abc"?  //
        // ^    ^__// to here
        // |_______// current char, validated
        // ```

        // Save the position of the opening quote so the final range covers the
        // whole literal.
        self.push_peek_position_into_stack();

        self.next_char(); // consume '"'

        let mut string_buffer = String::new();

        loop {
            match self.next_char() {
                Some(current_char) => {
                    match current_char {
                        '\\' => {
                            // Save the escape start so `unescape_unicode_code_point`
                            // can reuse the same position stack.
                            self.push_last_position_into_stack();

                            // escape chars
                            match self.next_char() {
                                Some(escape_type) => {
                                    match escape_type {
                                        '\\' => {
                                            string_buffer.push('\\');
                                        }
                                        '\'' => {
                                            // `\'` is accepted for consistency with char literals.
                                            string_buffer.push('\'');
                                        }
                                        '"' => {
                                            string_buffer.push('"');
                                        }
                                        't' => {
                                            // horizontal tabulation
                                            string_buffer.push('\t');
                                        }
                                        'r' => {
                                            // carriage return (CR, ascii 13)
                                            string_buffer.push('\r');
                                        }
                                        'n' => {
                                            // new line character (line feed, LF, ascii 10)
                                            string_buffer.push('\n');
                                        }
                                        '0' => {
                                            // null char
                                            string_buffer.push('\0');
                                        }
                                        'u' => {
                                            if self.peek_char_and_equals(0, '{') {
                                                // Unicode code point, for example '\u{2d}', '\u{6587}'
                                                let ch = self.unescape_unicode_code_point()?;
                                                string_buffer.push(ch);
                                            } else {
                                                return Err(AnreError::MessageWithPosition(
                                                    "Missing opening brace for Unicode escape sequence.".to_string(),
                                                    self.last_position
                                                ));
                                            }
                                        }
                                        _ => {
                                            return Err(AnreError::MessageWithPosition(
                                                format!(
                                                    "Unsupported escape char '{}'.",
                                                    escape_type
                                                ),
                                                self.last_position,
                                            ));
                                        }
                                    }
                                }
                                None => {
                                    // `\` + EOF
                                    return Err(AnreError::UnexpectedEndOfDocument(
                                        "Incomplete escape sequence.".to_string(),
                                    ));
                                }
                            }

                            // Discard the temporary escape start position.
                            self.pop_position_from_stack();
                        }
                        '"' => {
                            // encounter the closing double quote, which
                            // means the end of the string literal.
                            break;
                        }
                        _ => {
                            // ordinary char
                            string_buffer.push(current_char);
                        }
                    }
                }
                None => {
                    // Incomplete string literal (`"...EOF`).
                    return Err(AnreError::UnexpectedEndOfDocument(
                        "Incomplete string literal.".to_string(),
                    ));
                }
            }
        }

        let string_range = Range::new(&self.pop_position_from_stack(), &self.last_position);

        Ok(TokenWithRange::new(
            Token::String(string_buffer),
            string_range,
        ))
    }

    fn lex_line_comment(&mut self) {
        // ```diagram
        // //.....?[\r]\n
        // ^^     ^__// to here ('?' = any char or EOF)
        // ||________// validated
        // |_________// current char, validated
        // ```

        // note that the trailing '\n' or '\r\n' does not belong to line comment

        self.next_char(); // consume the 1st '/'
        self.next_char(); // consume the 2nd '/'

        while let Some(current_char) = self.peek_char(0) {
            // Leave the newline for the outer loop so line accounting stays in one place.
            match current_char {
                '\n' => {
                    break;
                }
                '\r' if self.peek_char_and_equals(0, '\n') => {
                    break;
                }
                _ => {
                    self.next_char(); // consume char
                }
            }
        }
    }

    fn lex_block_comment(&mut self) -> Result<(), AnreError> {
        // ```diagram
        // /*...*/?  //
        // ^^     ^__// to here
        // ||________// validated
        // |_________// current char, validated
        // ```

        self.next_char(); // consume '/'
        self.next_char(); // consume '*'

        let mut block_comment_depth = 1; // nested depth

        loop {
            match self.next_char() {
                Some(current_char) => {
                    match current_char {
                        '/' if self.peek_char_and_equals(0, '*') => {
                            // nested block comment
                            self.next_char(); // consume '*'

                            // increase depth
                            block_comment_depth += 1;
                        }
                        '*' if self.peek_char_and_equals(0, '/') => {
                            self.next_char(); // consume '/'

                            // decrease depth
                            block_comment_depth -= 1;

                            // check pairs
                            if block_comment_depth == 0 {
                                break;
                            }
                        }
                        _ => {
                            // Everything else stays inside the current block comment.
                        }
                    }
                }
                None => {
                    let msg = if block_comment_depth > 1 {
                        "Incomplete nested block comment.".to_string()
                    } else {
                        "Incomplete block comment.".to_string()
                    };

                    return Err(AnreError::UnexpectedEndOfDocument(msg));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        anre::token::{Token, TokenWithRange},
        error::AnreError,
        position::Position,
        range::Range,
    };

    use super::lex_from_str;

    impl Token {
        pub fn new_identifier(s: &str) -> Self {
            Token::Identifier(s.to_owned())
        }

        pub fn new_keyword(s: &str) -> Self {
            Token::Keyword(s.to_owned())
        }

        pub fn new_string(s: &str) -> Self {
            Token::String(s.to_owned())
        }
    }

    fn lex_from_str_without_location(s: &str) -> Result<Vec<Token>, AnreError> {
        let tokens = lex_from_str(s)?
            .into_iter()
            .map(|e| e.token)
            .collect::<Vec<Token>>();
        Ok(tokens)
    }

    #[test]
    fn test_lex_whitespaces() {
        assert_eq!(lex_from_str_without_location("  ").unwrap(), vec![]);

        assert_eq!(
            lex_from_str_without_location("()").unwrap(),
            vec![Token::ParenthesisOpen, Token::ParenthesisClose]
        );

        assert_eq!(
            lex_from_str_without_location("(  )").unwrap(),
            vec![Token::ParenthesisOpen, Token::ParenthesisClose]
        );

        assert_eq!(
            lex_from_str_without_location("( , )").unwrap(),
            vec![Token::ParenthesisOpen, Token::ParenthesisClose]
        );

        assert_eq!(
            lex_from_str_without_location("( , , ,, )").unwrap(),
            vec![Token::ParenthesisOpen, Token::ParenthesisClose]
        );

        assert_eq!(
            lex_from_str_without_location("(\t\r\n\n\n)").unwrap(),
            vec![Token::ParenthesisOpen, Token::ParenthesisClose,]
        );

        assert_eq!(
            lex_from_str_without_location("(\n,\n)").unwrap(),
            vec![Token::ParenthesisOpen, Token::ParenthesisClose]
        );

        // Test ranges.

        assert_eq!(lex_from_str("  ").unwrap(), vec![]);

        assert_eq!(
            lex_from_str("()").unwrap(),
            vec![
                TokenWithRange::new(Token::ParenthesisOpen, Range::from_detail(0, 0, 0, 1)),
                TokenWithRange::new(Token::ParenthesisClose, Range::from_detail(1, 0, 1, 1)),
            ]
        );

        assert_eq!(
            lex_from_str("(  )").unwrap(),
            vec![
                TokenWithRange::new(Token::ParenthesisOpen, Range::from_detail(0, 0, 0, 1)),
                TokenWithRange::new(Token::ParenthesisClose, Range::from_detail(3, 0, 3, 1)),
            ]
        );

        // "(\t\r\n\n\n)"
        //  _--____--__-
        //  0  2   4 5 6    // index
        //  0  0   1 2 3    // line
        //  0  2   0 0 1    // column
        //  1  2   1 1 1    // length

        assert_eq!(
            lex_from_str("(\t\r\n\n\n)").unwrap(),
            vec![
                TokenWithRange::new(Token::ParenthesisOpen, Range::from_detail(0, 0, 0, 1)),
                TokenWithRange::new(Token::ParenthesisClose, Range::from_detail(6, 3, 0, 1)),
            ]
        );
    }

    #[test]
    fn test_lex_punctuations() {
        assert_eq!(
            lex_from_str_without_location("!...#^||[]()???++?**?{}").unwrap(),
            vec![
                Token::Not,
                Token::Range,
                Token::Dot,
                Token::Hash,
                Token::Caret,
                Token::LogicOr,
                Token::BracketOpen,
                Token::BracketClose,
                Token::ParenthesisOpen,
                Token::ParenthesisClose,
                Token::LazyOptional,
                Token::Optional,
                Token::OneOrMore,
                Token::LazyOneOrMore,
                Token::ZeroOrMore,
                Token::LazyZeroOrMore,
                Token::BraceOpen,
                Token::BraceClose
            ]
        );
    }

    #[test]
    fn test_lex_identifier() {
        assert_eq!(
            lex_from_str_without_location("name").unwrap(),
            vec![Token::new_identifier("name")]
        );

        assert_eq!(
            lex_from_str_without_location("(name)").unwrap(),
            vec![
                Token::ParenthesisOpen,
                Token::new_identifier("name"),
                Token::ParenthesisClose,
            ]
        );

        assert_eq!(
            lex_from_str_without_location("( a )").unwrap(),
            vec![
                Token::ParenthesisOpen,
                Token::new_identifier("a"),
                Token::ParenthesisClose,
            ]
        );

        assert_eq!(
            lex_from_str_without_location("a__b__c").unwrap(),
            vec![Token::new_identifier("a__b__c")]
        );

        assert_eq!(
            lex_from_str_without_location("foo bar").unwrap(),
            vec![Token::new_identifier("foo"), Token::new_identifier("bar")]
        );

        assert_eq!(
            lex_from_str_without_location("αβγ 文字 🍞🥛").unwrap(),
            vec![
                Token::new_identifier("αβγ"),
                Token::new_identifier("文字"),
                Token::new_identifier("🍞🥛"),
            ]
        );

        // Test ranges.

        assert_eq!(
            lex_from_str("hello ASON").unwrap(),
            vec![
                TokenWithRange::new(
                    Token::new_identifier("hello"),
                    Range::from_position_and_length(&Position::new(0, 0, 0), 5)
                ),
                TokenWithRange::new(
                    Token::new_identifier("ASON"),
                    Range::from_position_and_length(&Position::new(6, 0, 6), 4)
                )
            ]
        );

        // Error: invalid char
        assert!(matches!(
            lex_from_str("abc&xyz"),
            Err(AnreError::MessageWithPosition(
                _,
                Position {
                    index: 3,
                    line: 0,
                    column: 3
                }
            ))
        ));
    }

    #[test]
    fn test_lex_keyword() {
        assert_eq!(
            lex_from_str_without_location("define as").unwrap(),
            vec![Token::new_keyword("define"), Token::new_keyword("as")]
        );

        // Test ranges.

        // "[\n    define\n    as\n]"
        //  01 23456789012 3456789 0   // index
        //  00 11111111111 2222222 3   // line
        //  01 01234567890 0123456 0   // column
        //  1      6           2   1   // length

        assert_eq!(
            lex_from_str("[\n    define\n    as\n]").unwrap(),
            vec![
                TokenWithRange::new(
                    Token::BracketOpen,
                    Range::from_position_and_length(&Position::new(0, 0, 0), 1)
                ),
                TokenWithRange::new(
                    Token::new_keyword("define"),
                    Range::from_position_and_length(&Position::new(6, 1, 4), 6)
                ),
                TokenWithRange::new(
                    Token::new_keyword("as"),
                    Range::from_position_and_length(&Position::new(17, 2, 4), 2)
                ),
                TokenWithRange::new(
                    Token::BracketClose,
                    Range::from_position_and_length(&Position::new(20, 3, 0), 1)
                ),
            ]
        );
    }

    #[test]
    fn test_lex_number() {
        assert_eq!(
            lex_from_str_without_location("223").unwrap(),
            vec![Token::Number(223),]
        );

        assert_eq!(
            lex_from_str_without_location("211").unwrap(),
            vec![Token::Number(211)]
        );

        assert_eq!(
            lex_from_str_without_location("223_211").unwrap(),
            vec![Token::Number(223_211)]
        );

        assert_eq!(
            lex_from_str_without_location("223 211").unwrap(),
            vec![Token::Number(223), Token::Number(211),]
        );

        // location

        assert_eq!(
            lex_from_str("223 211").unwrap(),
            vec![
                TokenWithRange::new(
                    Token::Number(223),
                    Range::from_position_and_length(&Position::new(0, 0, 0,), 3)
                ),
                TokenWithRange::new(
                    Token::Number(211),
                    Range::from_position_and_length(&Position::new(4, 0, 4,), 3)
                ),
            ]
        );

        // Error: invalid char for decimal number
        assert!(matches!(
            lex_from_str_without_location("12x34"),
            Err(AnreError::MessageWithPosition(
                _,
                Position {
                    index: 2,
                    line: 0,
                    column: 2
                }
            ))
        ));

        // Error: unsupported hexadecimal number (starting with "0x")
        assert!(matches!(
            lex_from_str_without_location("0x1234"),
            Err(AnreError::MessageWithPosition(
                _,
                Position {
                    index: 1,
                    line: 0,
                    column: 1
                }
            ))
        ));
    }

    #[test]
    fn test_lex_char() {
        assert_eq!(
            lex_from_str_without_location("'a'").unwrap(),
            vec![Token::Char('a')]
        );

        assert_eq!(
            lex_from_str_without_location("('a')").unwrap(),
            vec![
                Token::ParenthesisOpen,
                Token::Char('a'),
                Token::ParenthesisClose
            ]
        );

        assert_eq!(
            lex_from_str_without_location("'a' 'z'").unwrap(),
            vec![Token::Char('a'), Token::Char('z')]
        );

        // CJK
        assert_eq!(
            lex_from_str_without_location("'文'").unwrap(),
            vec![Token::Char('文')]
        );

        // emoji
        assert_eq!(
            lex_from_str_without_location("'😊'").unwrap(),
            vec![Token::Char('😊')]
        );

        // escape char `\\`
        assert_eq!(
            lex_from_str_without_location("'\\\\'").unwrap(),
            vec![Token::Char('\\')]
        );

        // escape char `\'`
        assert_eq!(
            lex_from_str_without_location("'\\\''").unwrap(),
            vec![Token::Char('\'')]
        );

        // escape char `"`
        assert_eq!(
            lex_from_str_without_location("'\\\"'").unwrap(),
            vec![Token::Char('"')]
        );

        // escape char `\t`
        assert_eq!(
            lex_from_str_without_location("'\\t'").unwrap(),
            vec![Token::Char('\t')]
        );

        // escape char `\r`
        assert_eq!(
            lex_from_str_without_location("'\\r'").unwrap(),
            vec![Token::Char('\r')]
        );

        // escape char `\n`
        assert_eq!(
            lex_from_str_without_location("'\\n'").unwrap(),
            vec![Token::Char('\n')]
        );

        // escape char `\0`
        assert_eq!(
            lex_from_str_without_location("'\\0'").unwrap(),
            vec![Token::Char('\0')]
        );

    // Unicode escape sequence.
        assert_eq!(
            lex_from_str_without_location("'\\u{2d}'").unwrap(),
            vec![Token::Char('-')]
        );

    // Unicode escape sequence.
        assert_eq!(
            lex_from_str_without_location("'\\u{6587}'").unwrap(),
            vec![Token::Char('文')]
        );

        // Test ranges.

        assert_eq!(
            lex_from_str("'a' '文' '\\t' '\\u{6587}'").unwrap(),
            vec![
                TokenWithRange::new(
                    Token::Char('a'),
                    Range::from_position_and_length(&Position::new(0, 0, 0), 3)
                ),
                TokenWithRange::new(
                    Token::Char('文'),
                    Range::from_position_and_length(&Position::new(4, 0, 4), 3)
                ),
                TokenWithRange::new(
                    Token::Char('\t'),
                    Range::from_position_and_length(&Position::new(8, 0, 8), 4)
                ),
                TokenWithRange::new(
                    Token::Char('文'),
                    Range::from_position_and_length(&Position::new(13, 0, 13), 10)
                ),
            ]
        );

        // Error: empty char
        assert!(matches!(
            lex_from_str("''"),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 0,
                        line: 0,
                        column: 0
                    },
                    end_inclusive: Position {
                        index: 1,
                        line: 0,
                        column: 1
                    }
                }
            ))
        ));

        // Error: empty char, missing the char
        assert!(matches!(
            lex_from_str("'"),
            Err(AnreError::UnexpectedEndOfDocument(_))
        ));

        // Error: incomplete char, missing the right quote, encountered EOF
        assert!(matches!(
            lex_from_str("'a"),
            Err(AnreError::UnexpectedEndOfDocument(_))
        ));

        // Error: invalid char, expect the right quote, encounter another char
        assert!(matches!(
            lex_from_str("'ab"),
            Err(AnreError::MessageWithPosition(
                _,
                Position {
                    index: 2,
                    line: 0,
                    column: 2,
                }
            ))
        ));

        // Error: invalid char, expect the right quote, encounter another char
        assert!(matches!(
            lex_from_str("'ab'"),
            Err(AnreError::MessageWithPosition(
                _,
                Position {
                    index: 2,
                    line: 0,
                    column: 2,
                }
            ))
        ));

        // Error: unsupported escape char \v
        assert!(matches!(
            lex_from_str(r#"'\v'"#),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 0,
                        line: 0,
                        column: 0
                    },
                    end_inclusive: Position {
                        index: 2,
                        line: 0,
                        column: 2
                    }
                }
            ))
        ));

        // Error: unsupported hex escape "\x.."
        assert!(matches!(
            lex_from_str(r#"'\x33'"#),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 0,
                        line: 0,
                        column: 0
                    },
                    end_inclusive: Position {
                        index: 2,
                        line: 0,
                        column: 2
                    }
                }
            ))
        ));

        // Error: empty unicode escape string
        // "'\\u{}'"
        //  01 2345     // index
        assert!(matches!(
            lex_from_str("'\\u{}'"),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 3,
                        line: 0,
                        column: 3
                    },
                    end_inclusive: Position {
                        index: 4,
                        line: 0,
                        column: 4
                    }
                }
            ))
        ));

        // Error: invalid unicode code point, digits too much
        // "'\\u{10001111}'"
        //  01 234567890123     // index
        assert!(matches!(
            lex_from_str("'\\u{10001111}'"),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 3,
                        line: 0,
                        column: 3
                    },
                    end_inclusive: Position {
                        index: 10,
                        line: 0,
                        column: 10
                    }
                }
            ))
        ));

        // Error: invalid unicode code point, code point out of range
        // "'\\u{123456}'"
        //  01 2345678901   // index
        assert!(matches!(
            lex_from_str("'\\u{123456}'"),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 3,
                        line: 0,
                        column: 3
                    },
                    end_inclusive: Position {
                        index: 10,
                        line: 0,
                        column: 10
                    }
                }
            ))
        ));

        // Error: invalid char in the unicode escape sequence
        assert!(matches!(
            lex_from_str("'\\u{12mn}''"),
            Err(AnreError::MessageWithPosition(
                _,
                Position {
                    index: 6,
                    line: 0,
                    column: 6,
                }
            ))
        ));

        // Error: missing the closing brace for unicode escape sequence
        assert!(matches!(
            lex_from_str("'\\u{1234'"),
            Err(AnreError::MessageWithPosition(
                _,
                Position {
                    index: 8,
                    line: 0,
                    column: 8,
                }
            ))
        ));

        // Error: incomplete unicode escape sequence, encountered EOF
        assert!(matches!(
            lex_from_str("'\\u{1234"),
            Err(AnreError::UnexpectedEndOfDocument(_))
        ));

        // Error: missing left brace for unicode escape sequence
        assert!(matches!(
            lex_from_str("'\\u1234}'"),
            Err(AnreError::MessageWithPosition(
                _,
                Position {
                    index: 2,
                    line: 0,
                    column: 2,
                }
            ))
        ));
    }

    #[test]
    fn test_lex_string() {
        assert_eq!(
            lex_from_str_without_location(r#""abc""#).unwrap(),
            vec![Token::new_string("abc")]
        );

        assert_eq!(
            lex_from_str_without_location(r#"("abc")"#).unwrap(),
            vec![
                Token::ParenthesisOpen,
                Token::new_string("abc"),
                Token::ParenthesisClose,
            ]
        );

        assert_eq!(
            lex_from_str_without_location(r#""abc" "xyz""#).unwrap(),
            vec![Token::new_string("abc"), Token::new_string("xyz")]
        );

        assert_eq!(
            lex_from_str_without_location("\"abc\"\n\n\"xyz\"").unwrap(),
            vec![Token::new_string("abc"), Token::new_string("xyz"),]
        );

        // unicode
        assert_eq!(
            lex_from_str_without_location(
                r#"
                "abc文字😊"
                "#
            )
            .unwrap(),
            vec![Token::new_string("abc文字😊"),]
        );

        // empty string
        assert_eq!(
            lex_from_str_without_location("\"\"").unwrap(),
            vec![Token::new_string("")]
        );

        // escape chars
        assert_eq!(
            lex_from_str_without_location(
                r#"
                "\\\'\"\t\r\n\0\u{2d}\u{6587}"
                "#
            )
            .unwrap(),
            vec![Token::new_string("\\\'\"\t\r\n\0-文"),]
        );

        // Test ranges.
        // "abc" "文字😊"
        // 01234567 8 9 0

        assert_eq!(
            lex_from_str(r#""abc" "文字😊""#).unwrap(),
            vec![
                TokenWithRange::new(
                    Token::new_string("abc"),
                    Range::from_position_and_length(&Position::new(0, 0, 0), 5)
                ),
                TokenWithRange::new(
                    Token::new_string("文字😊"),
                    Range::from_position_and_length(&Position::new(6, 0, 6), 5)
                ),
            ]
        );

        // Error: incomplete string, missing the closed quote
        assert!(matches!(
            lex_from_str("\"abc"),
            Err(AnreError::UnexpectedEndOfDocument(_))
        ));

        // Error: incomplete string, missing the closed quote, ends with \n
        assert!(matches!(
            lex_from_str("\"abc\n"),
            Err(AnreError::UnexpectedEndOfDocument(_))
        ));

        // Error: incomplete string, missing the closed quote, ends with whitespaces/other chars
        assert!(matches!(
            lex_from_str("\"abc\n   "),
            Err(AnreError::UnexpectedEndOfDocument(_))
        ));

        // Error: unsupported escape char \v
        assert!(matches!(
            lex_from_str(r#""abc\vxyz""#),
            Err(AnreError::MessageWithPosition(
                _,
                Position {
                    index: 5,
                    line: 0,
                    column: 5,
                }
            ))
        ));

        // Error: unsupported hex escape "\x.."
        assert!(matches!(
            lex_from_str(r#""abc\x33xyz""#),
            Err(AnreError::MessageWithPosition(
                _,
                Position {
                    index: 5,
                    line: 0,
                    column: 5,
                }
            ))
        ));

        // Error: empty unicode escape string
        // "abc\u{}"
        // 012345678    // index
        assert!(matches!(
            lex_from_str(r#""abc\u{}xyz""#),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 6,
                        line: 0,
                        column: 6
                    },
                    end_inclusive: Position {
                        index: 7,
                        line: 0,
                        column: 7
                    }
                }
            ))
        ));

        // Error: invalid unicode code point, too many digits
        // "abc\u{1000111}xyz"
        // 0123456789023456789  // index
        assert!(matches!(
            lex_from_str(r#""abc\u{1000111}xyz""#),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 6,
                        line: 0,
                        column: 6
                    },
                    end_inclusive: Position {
                        index: 13,
                        line: 0,
                        column: 13
                    }
                }
            ))
        ));

        // Error: invalid unicode code point, code point out of range
        // "abc\u{123456}xyz"
        // 012345678901234567   // index
        assert!(matches!(
            lex_from_str(r#""abc\u{123456}xyz""#),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 6,
                        line: 0,
                        column: 6
                    },
                    end_inclusive: Position {
                        index: 13,
                        line: 0,
                        column: 13
                    }
                }
            ))
        ));

        // Error: invalid char in the unicode escape sequence
        assert!(matches!(
            lex_from_str(r#""abc\u{12mn}xyz""#),
            Err(AnreError::MessageWithPosition(
                _,
                Position {
                    index: 9,
                    line: 0,
                    column: 9,
                }
            ))
        ));

        // Error: missing the closing brace for unicode escape sequence
        assert!(matches!(
            lex_from_str(r#""abc\u{1234""#),
            Err(AnreError::MessageWithPosition(
                _,
                Position {
                    index: 11,
                    line: 0,
                    column: 11,
                }
            ))
        ));

        // Error: incomplete unicode escape sequence, encountered EOF
        assert!(matches!(
            lex_from_str(r#""abc\u{1234"#),
            Err(AnreError::UnexpectedEndOfDocument(_))
        ));

        // Error: missing left brace for unicode escape sequence
        assert!(matches!(
            lex_from_str(r#""abc\u1234}xyz""#),
            Err(AnreError::MessageWithPosition(
                _,
                Position {
                    index: 5,
                    line: 0,
                    column: 5,
                }
            ))
        ));
    }

    #[test]
    fn test_lex_line_comment() {
        assert_eq!(
            lex_from_str_without_location(
                r#"
                7 //11
                13 17// 19 23
                //  29
                31//    37
                "#
            )
            .unwrap(),
            vec![
                Token::Number(7),
                Token::Number(13),
                Token::Number(17),
                Token::Number(31),
            ]
        );

        // Test ranges.

        assert_eq!(
            lex_from_str("foo // bar").unwrap(),
            vec![TokenWithRange::new(
                Token::Identifier("foo".to_string()),
                Range::from_position_and_length(&Position::new(0, 0, 0), 3)
            ),]
        );

        assert_eq!(
            lex_from_str("abc // def\n// xyz\n").unwrap(),
            vec![TokenWithRange::new(
                Token::Identifier("abc".to_string()),
                Range::from_position_and_length(&Position::new(0, 0, 0), 3)
            ),]
        );
    }

    #[test]
    fn test_lex_block_comment() {
        assert_eq!(
            lex_from_str_without_location(
                r#"
                7 /* 11 13 */ 17
                "#
            )
            .unwrap(),
            vec![Token::Number(7), Token::Number(17),]
        );

        // nested block comment
        assert_eq!(
            lex_from_str_without_location(
                r#"
                7 /* 11 /* 13 */ 17 */ 19
                "#
            )
            .unwrap(),
            vec![Token::Number(7), Token::Number(19),]
        );

        // line comment chars "//" within the block comment
        assert_eq!(
            lex_from_str_without_location(
                r#"
                7 /* 11 // 13 17 */ 19
                "#
            )
            .unwrap(),
            vec![Token::Number(7), Token::Number(19),]
        );

        // Test ranges.

        assert_eq!(
            lex_from_str("foo /* hello */ bar").unwrap(),
            vec![
                TokenWithRange::new(
                    Token::Identifier("foo".to_string()),
                    Range::from_position_and_length(&Position::new(0, 0, 0), 3)
                ),
                TokenWithRange::new(
                    Token::Identifier("bar".to_string()),
                    Range::from_position_and_length(&Position::new(16, 0, 16), 3)
                ),
            ]
        );

        assert_eq!(lex_from_str("/* abc\nxyz */ /* hello */").unwrap(), vec![]);

        // Error: incomplete, missing "*/"
        assert!(matches!(
            lex_from_str("7 /* 11"),
            Err(AnreError::UnexpectedEndOfDocument(_))
        ));

        // Error: incomplete, missing "*/", ends with \n
        assert!(matches!(
            lex_from_str("7 /* 11\n"),
            Err(AnreError::UnexpectedEndOfDocument(_))
        ));

        // Error: incomplete, unpaired, missing "*/"
        assert!(matches!(
            lex_from_str("a /* b /* c */"),
            Err(AnreError::UnexpectedEndOfDocument(_))
        ));

        // Error: incomplete, unpaired, missing "*/", ends with \n
        assert!(matches!(
            lex_from_str("a /* b /* c */\n"),
            Err(AnreError::UnexpectedEndOfDocument(_))
        ));
    }

    #[test]
    fn test_lex_combined_line_comments_and_block_comments() {
        assert_eq!(
            lex_from_str_without_location(
                r#"11 // line comment 1
                // line comment 2
                13 /* block comment 1 */
                /*
                block comment 2
                */
                17
                "#
            )
            .unwrap(),
            vec![Token::Number(11), Token::Number(13), Token::Number(17),]
        );

        assert_eq!(
            lex_from_str(r#"11 /* foo */ 13"#).unwrap(),
            vec![
                TokenWithRange::new(
                    Token::Number(11),
                    Range::from_position_and_length(&Position::new(0, 0, 0), 2)
                ),
                TokenWithRange::new(
                    Token::Number(13),
                    Range::from_position_and_length(&Position::new(13, 0, 13), 2)
                ),
            ]
        );
    }

    #[test]
    fn test_lex_combined_comments_and_whitespaces() {
        assert_eq!(
            lex_from_str_without_location(
                r#"
                    [1,2,

                    3

                    ,4

                    ,

                    5
                    ,
                    // comment between commas
                    ,
                    6

                    // comment between blank lines

                    7
                    8
                    ]

                    "#
            )
            .unwrap(),
            vec![
                Token::BracketOpen,
                Token::Number(1),
                Token::Number(2),
                Token::Number(3),
                Token::Number(4),
                Token::Number(5),
                Token::Number(6),
                Token::Number(7),
                Token::Number(8),
                Token::BracketClose,
            ]
        );

        // range

        // blanks -> blank
        assert_eq!(
            lex_from_str("11\n \n  \n13").unwrap(),
            vec![
                TokenWithRange::new(
                    Token::Number(11),
                    Range::from_position_and_length(&Position::new(0, 0, 0), 2)
                ),
                TokenWithRange::new(
                    Token::Number(13),
                    Range::from_position_and_length(&Position::new(8, 3, 0), 2)
                ),
            ]
        );

        // comma + blanks -> comma
        assert_eq!(
            lex_from_str(",\n\n\n11").unwrap(),
            vec![TokenWithRange::new(
                Token::Number(11),
                Range::from_position_and_length(&Position::new(4, 3, 0), 2)
            ),]
        );

        // blanks + comma -> comma
        assert_eq!(
            lex_from_str("11\n\n\n,").unwrap(),
            vec![TokenWithRange::new(
                Token::Number(11),
                Range::from_position_and_length(&Position::new(0, 0, 0), 2)
            ),]
        );

        // blanks + comma + blanks -> comma
        assert_eq!(
            lex_from_str("11\n\n,\n\n13").unwrap(),
            vec![
                TokenWithRange::new(
                    Token::Number(11),
                    Range::from_position_and_length(&Position::new(0, 0, 0), 2)
                ),
                TokenWithRange::new(
                    Token::Number(13),
                    Range::from_position_and_length(&Position::new(7, 4, 0), 2)
                ),
            ]
        );

        // comma + comment + comma -> comma + comma
        assert_eq!(lex_from_str(",//abc\n,").unwrap(), vec![]);

        // blanks + comment + blanks -> blank
        assert_eq!(
            lex_from_str("11\n\n//abc\n\n13").unwrap(),
            vec![
                TokenWithRange::new(
                    Token::Number(11),
                    Range::from_position_and_length(&Position::new(0, 0, 0), 2)
                ),
                TokenWithRange::new(
                    Token::Number(13),
                    Range::from_position_and_length(&Position::new(11, 4, 0), 2)
                ),
            ]
        );
    }

    #[test]
    fn test_lex_multiple_tokens() {
        assert_eq!(
            lex_from_str_without_location(
                r#"
                one_or_more([
                    'a'..'f'    // comment 1
                    '0'..'9'    // comment 2
                    '_'
                ])
                "#
            )
            .unwrap(),
            vec![
                Token::new_identifier("one_or_more"),
                Token::ParenthesisOpen,
                Token::BracketOpen,
                Token::Char('a'),
                Token::Range,
                Token::Char('f'),
                Token::Char('0'),
                Token::Range,
                Token::Char('9'),
                Token::Char('_'),
                Token::BracketClose,
                Token::ParenthesisClose,
            ]
        );

        assert_eq!(
            lex_from_str_without_location(
                r#"
                ('a', "def", /* block comment */ "xyz".repeat(3)).one_or_more()
                "#
            )
            .unwrap(),
            vec![
                Token::ParenthesisOpen,
                Token::Char('a'),
                Token::new_string("def"),
                Token::new_string("xyz"),
                Token::Dot,
                Token::new_identifier("repeat"),
                Token::ParenthesisOpen,
                Token::Number(3),
                Token::ParenthesisClose,
                Token::ParenthesisClose,
                Token::Dot,
                Token::new_identifier("one_or_more"),
                Token::ParenthesisOpen,
                Token::ParenthesisClose,
            ]
        );

        assert_eq!(
            lex_from_str_without_location(
                r#"
                'a'?
                'b'+
                'c'*
                'd'{1..2}
                "#
            )
            .unwrap(),
            vec![
                Token::Char('a'),
                Token::Optional,
                Token::Char('b'),
                Token::OneOrMore,
                Token::Char('c'),
                Token::ZeroOrMore,
                Token::Char('d'),
                Token::BraceOpen,
                Token::Number(1),
                Token::Range,
                Token::Number(2),
                Token::BraceClose
            ]
        );
    }
}
