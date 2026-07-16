// Copyright (c) 2026 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

// Regular Expression syntax summary:
//
// Meta characters:
//
// | Meta      | Description                                  |
// |-----------|----------------------------------------------|
// | `[...]`   | Character set                                |
// | `[^...]`  | Negative character set                       |
// | `{`m}     | Exact repetition (m times)                   |
// | `{`m,n}   | Repetition range (from m to n)               |
// | `{`m,}    | Repetition from m to infinity                |
// | `(...)`   | Grouping with indexed capturing                  |
// | `*`       | Zero or more repetitions                     |
// | `+`       | One or more repetitions                      |
// | `?`       | Optional or lazy repetition                  |
// | `|`       | Logical OR                                   |
// | `^`       | Start-of-line assertion                      |
// | `$`       | End-of-line assertion                        |
// | `.`       | Any character except newline (`\r` and `\n`) |
// | `\...`    | Escape character for special symbols         |
//
// About the meta characters escaping:
//
// Meta characters `( ) { } [ ] + * ? . | ^ $ \` must be escaped when
// they are used as literal,
//
// For example:
//
// - `\(`
// - `\*`
// - `\\`
//
// In character sets, only `]` and `\` need escaping, and
// the hyphen `-` must be escaped unless it is the first or the
// last character in the set,
//
// For example:
//
// - `[a\-b]`
// - `[ab-]`
// - `[-ab]`
//
// Escaped characters:
//
// | Escaped char | Description                                 |
// |--------------|---------------------------------------------|
// | `\t`         | Horizontal tab                              |
// | `\n`         | Newline                                     |
// | `\r`         | Carriage return                             |
// | `\u{hhhh}`   | Unicode character (hexadecimal code point)  |
//
// Unsupported escape sequences:
//
// | Escaped char | Description                                 |
// |--------------|---------------------------------------------|
// | `\f`         | Form feed                                   |
// | `\v`         | Vertical tab                                |
// | `\0`         | Null character                              |
//
// Preset character sets:
//
// | Preset char | Description                             |
// |-------------|-----------------------------------------|
// | `\w`        | Alphanumeric characters: `[a-zA-Z0-9_]` |
// | `\W`        | Negated \w: `[^\w]`                     |
// | `\d`        | Digits: `[0-9]`                         |
// | `\D`        | Negated \d: `[^\d]`                     |
// | `\s`        | Whitespace characters: `[ \t\r\n\v\f]`  |
// | `\S`        | Negated \s: `[^\s]`                     |
//
// Word boundary assertions:
//
// | Assertion | Description         |
// |-----------|---------------------|
// | `\b`      | Word boundary       |
// | `\B`      | Not a word boundary |
//
// Non-capturing groups:
//
// `(?:...)`
//
// Named capture groups:
//
// `(?<name>...)`
//
// Backreferences:
//
// | Type       |  Description                                 |
// |------------|----------------------------------------------|
// | `\number`  | Backreference by group number, for example, `\1`    |
// | `\k<name>` | Backreference by group name, for example, `\k<foo>` |
//
// Lookaround assertions:
//
// | Type       | Description         |
// |------------|---------------------|
// | `(?=...)`  | Positive lookahead  |
// | `(?!...)`  | Negative lookahead  |
// | `(?<=...)` | Positive lookbehind |
// | `(?<!...)` | Negative lookbehind |

use crate::{
    char_with_position::{CharWithPosition, CharsWithPositionIter},
    error::AnreError,
    peekable_iter::PeekableIter,
    position::Position,
    range::Range,
};

use super::token::{Repetition, Token, TokenWithRange};

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
                '[' => {
                    // charset start
                    let mut twrs = self.lex_charset()?;
                    token_with_ranges.append(&mut twrs);
                }
                '{' => {
                    // repetition
                    let twr = self.lex_repetition()?;
                    token_with_ranges.push(twr);
                }
                '(' if self.peek_char_and_equals(1, '?') => {
                    // `(?...)` group, which can be non-capturing, lookahead, lookbehind, or named capture group
                    //
                    // - `(?:...)` Non-capturing group
                    // - `(?<name>...)` Named capture group
                    // - `(?<=...)` Positive lookbehind
                    // - `(?<!...)` Negative lookbehind
                    // - `(?=...)` Positive lookahead
                    // - `(?!...)` Negative lookahead

                    self.push_peek_position_into_stack();

                    match self.peek_char(2) {
                        Some(':' | '<' | '=' | '!') => {
                            self.next_char(); // consume '('
                            self.next_char(); // consume '?'

                            match self.peek_char(0).unwrap() {
                                ':' => {
                                    // non-capturing
                                    self.next_char(); // consule ':'
                                    token_with_ranges.push(TokenWithRange {
                                        token: Token::NonCaptureGroupStart,
                                        range: Range::new(
                                            &self.pop_position_from_stack(),
                                            &self.last_position,
                                        ),
                                    });
                                }
                                '<' => {
                                    match self.peek_char(1) {
                                        Some('=') => {
                                            // look-behind group
                                            self.next_char(); // consume '<'
                                            self.next_char(); // consume '='
                                            token_with_ranges.push(TokenWithRange {
                                                token: Token::LookBehindGroupStart,
                                                range: Range::new(
                                                    &self.pop_position_from_stack(),
                                                    &self.last_position,
                                                ),
                                            });
                                        }
                                        Some('!') => {
                                            // negative look-behind group
                                            self.next_char(); // consume '<'
                                            self.next_char(); // consume '='
                                            token_with_ranges.push(TokenWithRange {
                                                token: Token::LookBehindNegativeGroupStart,
                                                range: Range::new(
                                                    &self.pop_position_from_stack(),
                                                    &self.last_position,
                                                ),
                                            });
                                        }
                                        _ => {
                                            // named capture group or incomplete lookbehind assertion
                                            let name = self.lex_capture_group_name()?;
                                            token_with_ranges.push(TokenWithRange {
                                                token: Token::NamedCaptureGroupStart(name),
                                                range: Range::new(
                                                    &self.pop_position_from_stack(),
                                                    &self.last_position,
                                                ),
                                            });
                                        }
                                    }
                                }
                                '=' => {
                                    // look-ahead group
                                    self.next_char(); // consule '='
                                    token_with_ranges.push(TokenWithRange {
                                        token: Token::LookAheadGroupStart,
                                        range: Range::new(
                                            &self.pop_position_from_stack(),
                                            &self.last_position,
                                        ),
                                    });
                                }
                                '!' => {
                                    // negative look-ahead group
                                    self.next_char(); // consule '!'
                                    token_with_ranges.push(TokenWithRange {
                                        token: Token::LookAheadNegativeGroupStart,
                                        range: Range::new(
                                            &self.pop_position_from_stack(),
                                            &self.last_position,
                                        ),
                                    });
                                }

                                _ => unreachable!(),
                            }
                        }
                        Some(_) => {
                            return Err(AnreError::MessageWithRange(
                                "Invalid group syntax.".to_string(),
                                Range::new(
                                    &self.pop_position_from_stack(),
                                    self.peek_position(2).unwrap(),
                                ),
                            ));
                        }
                        None => {
                            return Err(AnreError::UnexpectedEndOfDocument(
                                "Incomplete group.".to_string(),
                            ));
                        }
                    }
                }
                '(' => {
                    // group start
                    self.next_char(); // consume '('
                    token_with_ranges.push(TokenWithRange::new(
                        Token::GroupStart,
                        Range::from_single_position(&self.last_position),
                    ));
                }
                ')' => {
                    // group end
                    self.next_char(); // consume ')'
                    token_with_ranges.push(TokenWithRange::new(
                        Token::GroupEnd,
                        Range::from_single_position(&self.last_position),
                    ));
                }
                '?' if self.peek_char_and_equals(1, '?') => {
                    // lazy optional `??`
                    self.push_peek_position_into_stack();

                    self.next_char(); // consume '?'
                    self.next_char(); // consume '?'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::LazyOptional,
                        Range::new(&self.pop_position_from_stack(), &self.last_position),
                    ));
                }
                '?' => {
                    // greedy optional '?'
                    self.next_char(); // consume '?'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::Optional,
                        Range::from_single_position(&self.last_position),
                    ));
                }
                '+' if self.peek_char_and_equals(1, '?') => {
                    // lazy one-or-more `+?`
                    self.push_peek_position_into_stack();

                    self.next_char(); // consume '+'
                    self.next_char(); // consume '?'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::LazyOneOrMore,
                        Range::new(&self.pop_position_from_stack(), &self.last_position),
                    ));
                }
                '+' => {
                    // greedy one-or-more '+'
                    self.next_char(); // consume '+'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::OneOrMore,
                        Range::from_single_position(&self.last_position),
                    ));
                }
                '*' if self.peek_char_and_equals(1, '?') => {
                    // lazy zero-or-more `*?`
                    self.push_peek_position_into_stack();

                    self.next_char(); // consume '*'
                    self.next_char(); // consume '?'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::LazyZeroOrMore,
                        Range::new(&self.pop_position_from_stack(), &self.last_position),
                    ));
                }
                '*' => {
                    // greedy zero-or-more '*'
                    self.next_char(); // consume '*'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::ZeroOrMore,
                        Range::from_single_position(&self.last_position),
                    ));
                }
                '^' => {
                    // line start boundary assertion
                    self.next_char(); // consume '^'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::LineBoundaryAssertionStart,
                        Range::from_single_position(&self.last_position),
                    ));
                }
                '$' => {
                    // line end boundary assertion
                    self.next_char(); // consume '$'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::LineBoundaryAssertionEnd,
                        Range::from_single_position(&self.last_position),
                    ));
                }
                '.' => {
                    // dot, matches any char except newline
                    self.next_char(); // consume '.'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::Dot,
                        Range::from_single_position(&self.last_position),
                    ));
                }
                '|' => {
                    // logical OR
                    self.next_char(); // consume '|'

                    token_with_ranges.push(TokenWithRange::new(
                        Token::LogicOr,
                        Range::from_single_position(&self.last_position),
                    ));
                }
                '\\' => {
                    // Escape sequence, for example `\t`, `\n`, `\r`, `\u{hhhh}`, `\w`, `\d`, `\s`, etc.
                    let twr = self.lex_escape_sequence()?;
                    token_with_ranges.push(twr);
                }
                _ => {
                    // ordinary char
                    let c = *current_char;
                    self.next_char(); // consume current char

                    token_with_ranges.push(TokenWithRange::new(
                        Token::Char(c),
                        Range::from_single_position(&self.last_position),
                    ));
                }
            }
        }

        Ok(token_with_ranges)
    }

    fn lex_charset(&mut self) -> Result<Vec<TokenWithRange>, AnreError> {
        // ```diagram
        // [^....]?  //
        // ^      ^__// to here
        // |_________// current char, validated
        // ```

        let mut token_with_ranges = vec![];

        self.push_peek_position_into_stack();

        self.next_char(); // consume '['

        let charset_start = if self.peek_char_and_equals(0, '^') {
            self.next_char(); // consume '^'
            TokenWithRange::new(
                Token::CharSetStartNegative,
                Range::new(&self.pop_position_from_stack(), &self.last_position),
            )
        } else {
            TokenWithRange::new(
                Token::CharSetStart,
                Range::new(&self.pop_position_from_stack(), &self.last_position),
            )
        };

        token_with_ranges.push(charset_start);

        loop {
            match self.peek_char(0) {
                Some(current_char) => {
                    match current_char {
                        '\\' => {
                            let twr = self.lex_escape_sequence_within_charset()?;
                            token_with_ranges.push(twr);
                        }
                        ']' => {
                            break;
                        }
                        _ => {
                            let c = *current_char;
                            self.next_char(); // consume current char

                            token_with_ranges.push(TokenWithRange::new(
                                Token::Char(c),
                                Range::from_single_position(&self.last_position),
                            ));
                        }
                    }
                }
                None => {
                    return Err(AnreError::UnexpectedEndOfDocument(
                        "Incomplete character set.".to_string(),
                    ));
                }
            }
        }

        self.next_char(); // consume ']'

        let charset_end = TokenWithRange::new(
            Token::CharSetEnd,
            Range::from_single_position(&self.last_position),
        );

        token_with_ranges.push(charset_end);

        // Scan and parse the char ranges in the charset, and merge them into `CharRange` tokens.
        //
        // For example:
        //
        // ```diagram
        // [a-z]
        //  ^ ^__ // to here
        //  |____ // merge from here
        // ```
        //
        // Note: the character range operator '-' must be escaped when it is not the first or last char
        // in the charset, so there won't be any ambiguity when scanning for char ranges.

        if token_with_ranges.len() > 4 {
            // reverse scanning
            let mut idx = token_with_ranges.len() - 3;
            while idx > 1 {
                if matches!(
                    token_with_ranges[idx],
                    TokenWithRange {
                        token: Token::Char('-'),
                        ..
                    }
                ) {
                    let range_start = &token_with_ranges[idx - 1].range;
                    let range_end = &token_with_ranges[idx + 1].range;

                    let char_start = if let Token::Char(c) = &token_with_ranges[idx - 1].token {
                        *c
                    } else {
                        return Err(AnreError::MessageWithRange(
                            "Expected a character for a range, for example \"A-Z\"."
                                .to_owned(),
                            *range_start,
                        ));
                    };

                    let char_end = if let Token::Char(c) = &token_with_ranges[idx + 1].token {
                        *c
                    } else {
                        return Err(AnreError::MessageWithRange(
                            "Expected a character for a range, for example \"a-z\"."
                                .to_owned(),
                            *range_end,
                        ));
                    };

                    let token = Token::CharRange(char_start, char_end);
                    let range = Range::merge(range_start, range_end);
                    let twr = TokenWithRange::new(token, range);

                    let pos = idx - 1;
                    token_with_ranges.drain(pos..(pos + 3));
                    token_with_ranges.insert(pos, twr);

                    idx -= 2;
                } else {
                    idx -= 1;
                }
            }
        }

        Ok(token_with_ranges)
    }

    fn lex_escape_sequence(&mut self) -> Result<TokenWithRange, AnreError> {
        // ```diagram
        // \xxxx?  //
        // ^    ^__// to here
        // |_______// current char, validated
        // ```
        self.push_peek_position_into_stack();

        self.next_char(); // consume '\'

        let token = match self.peek_char(0) {
            Some(current_char) => {
                match current_char {
                    // general escaped chars
                    't' => {
                        // horizontal tabulation
                        self.next_char();
                        Token::Char('\t')
                    }
                    'n' => {
                        // new line character (line feed, LF, ascii 10)
                        self.next_char();
                        Token::Char('\n')
                    }
                    'r' => {
                        // carriage return (CR, ascii 13)
                        self.next_char();
                        Token::Char('\r')
                    }
                    'u' => {
                        // Unicode code point, for example '\u{2d}', '\u{6587}'
                        self.next_char(); // consume 'u'

                        if self.peek_char_and_equals(0, '{') {
                            let c = self.unescape_unicode()?;
                            Token::Char(c)
                        } else {
                            return Err(AnreError::MessageWithPosition(
                                "Missing opening brace for Unicode escape sequence.".to_string(),
                                self.last_position,
                            ));
                        }
                    }
                    // meta chars
                    '(' | ')' | '{' | '}' | '[' | ']' | '+' | '*' | '?' | '.' | '|' | '^' | '$'
                    | '\\' => {
                        let c = *current_char;
                        self.next_char();
                        Token::Char(c)
                    }
                    // preset charsets
                    'w' | 'W' | 'd' | 'D' | 's' | 'S' => {
                        let c = *current_char;
                        self.next_char();
                        Token::PresetCharSet(c)
                    }
                    // word boundary assertions
                    'b' | 'B' => {
                        let c = *current_char;
                        self.next_char();
                        Token::WordBoundaryAssertion(c == 'B')
                    }
                    // back reference by index
                    '1'..='9' => {
                        let num = self.lex_number_decimal()?;
                        Token::BackReferenceNumber(num)
                    }
                    // invalid back reference group number 0
                    '0' => {
                        return Err(AnreError::MessageWithRange(
                            "Cannot back-reference group 0.".to_string(),
                            Range::new(
                                &self.pop_position_from_stack(),
                                self.peek_position(0).unwrap(),
                            ),
                        ));
                    }
                    // back reference by name
                    'k' => {
                        self.next_char(); // consume 'k'

                        if self.peek_char_and_equals(0, '<') {
                            let s = self.lex_capture_group_name()?;
                            Token::BackReferenceName(s)
                        } else {
                            return Err(AnreError::MessageWithRange(
                                "Missing opening angle bracket for group name.".to_string(),
                                Range::new(
                                    &self.pop_position_from_stack(),
                                    self.peek_position(0).unwrap(),
                                ),
                            ));
                        }
                    }
                    _ => {
                        return Err(AnreError::MessageWithRange(
                            format!("Unsupported escape char '{}'.", current_char),
                            Range::new(
                                &self.pop_position_from_stack(),
                                self.peek_position(0).unwrap(),
                            ),
                        ));
                    }
                }
            }
            None => {
                // `\` | EOF
                return Err(AnreError::UnexpectedEndOfDocument(
                    "Incomplete escape sequence.".to_string(),
                ));
            }
        };

        let token_range = Range::new(&self.pop_position_from_stack(), &self.last_position);

        Ok(TokenWithRange::new(token, token_range))
    }

    fn lex_escape_sequence_within_charset(&mut self) -> Result<TokenWithRange, AnreError> {
        // ```diagram
        // [\xxxx...]  //
        //  ^    ^_____// to here
        //  |__________// current char, validated
        // ```

        self.push_peek_position_into_stack();

        self.next_char(); // consume '\'

        let token = match self.peek_char(0) {
            Some(current_char) => {
                match current_char {
                    // general escaped chars
                    't' => {
                        // horizontal tabulation
                        self.next_char();
                        Token::Char('\t')
                    }
                    'n' => {
                        // new line character (line feed, LF, ascii 10)
                        self.next_char();
                        Token::Char('\n')
                    }
                    'r' => {
                        // carriage return (CR, ascii 13)
                        self.next_char();
                        Token::Char('\r')
                    }
                    'u' => {
                        // Unicode code point, for example '\u{2d}', '\u{6587}'
                        self.next_char(); // consume 'u'

                        if self.peek_char_and_equals(0, '{') {
                            let c = self.unescape_unicode()?;
                            Token::Char(c)
                        } else {
                            return Err(AnreError::MessageWithRange(
                                "Missing opening brace for Unicode escape sequence.".to_string(),
                                Range::new(&self.pop_position_from_stack(), &self.last_position),
                            ));
                        }
                    }

                    // meta chars
                    //
                    // in the charset, only the meta char ']', '\' and (non-first or non-last) '-'
                    // are required to be escaped,
                    // but the escapes of meta chars are also supported for consistency.
                    '(' | ')' | '{' | '}' | '[' | ']' | '+' | '*' | '?' | '.' | '|' | '^' | '$'
                    | '\\' => {
                        let c = *current_char;
                        self.next_char();
                        Token::Char(c)
                    }
                    // preset charsets
                    'w' | 'd' | 's' => {
                        let c = *current_char;
                        self.next_char();
                        Token::PresetCharSet(c)
                    }
                    // negative preset charsets, only positive preset charsets are supported
                    'W' | 'D' | 'S' => {
                        return Err(AnreError::MessageWithRange(
                            format!(
                                "Negative character class '{}' is not supported in a character set.",
                                current_char
                            ),
                            Range::new(
                                &self.pop_position_from_stack(),
                                self.peek_position(0).unwrap(),
                            ),
                        ));
                    }
                    'b' | 'B' => {
                        return Err(AnreError::MessageWithRange(
                            "Word boundary assertions are not supported in a character set.".to_string(),
                            Range::new(
                                &self.pop_position_from_stack(),
                                self.peek_position(0).unwrap(),
                            ),
                        ));
                    }
                    '0'..='9' | 'k' => {
                        return Err(AnreError::MessageWithRange(
                            "Backreferences are not supported in a character set.".to_string(),
                            Range::new(
                                &self.pop_position_from_stack(),
                                self.peek_position(0).unwrap(),
                            ),
                        ));
                    }
                    _ => {
                        return Err(AnreError::MessageWithRange(
                            format!("Unsupported escape char '{}' in charset.", current_char),
                            Range::new(
                                &self.pop_position_from_stack(),
                                self.peek_position(0).unwrap(),
                            ),
                        ));
                    }
                }
            }
            None => {
                // `\` | EOF
                return Err(AnreError::UnexpectedEndOfDocument(
                    "Incomplete escape sequence.".to_string(),
                ));
            }
        };

        let token_range = Range::new(&self.pop_position_from_stack(), &self.last_position);

        Ok(TokenWithRange::new(token, token_range))
    }

    fn unescape_unicode(&mut self) -> Result<char, AnreError> {
        // ```diagram
        // \u{6587}?  //
        //   ^     ^__// to here
        //   |________// current char, validated
        // ```

        self.push_peek_position_into_stack();

        self.next_char(); // comsume char '{'

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

    fn lex_capture_group_name(&mut self) -> Result<String, AnreError> {
        // ```diagram
        // <name>?  //
        // ^     ^__// to here
        // |________// current char, validated
        // ```

        self.push_peek_position_into_stack();

        self.next_char(); // consume '<'

        let mut name_buffer = String::new();

        loop {
            match self.peek_char(0) {
                Some(current_char) => match current_char {
                    '0'..='9' | 'a'..='z' | 'A'..='Z' | '_' => {
                        name_buffer.push(*current_char);
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
                        // summary:
                        // Block Location Comment
                        // CJK Unified Ideographs 4E00–9FFF Common
                        // CJK Unified Ideographs Extension A 3400–4DBF Rare
                        // CJK Unified Ideographs Extension B 20000–2A6DF Rare, historic
                        // CJK Unified Ideographs Extension C 2A700–2B73F Rare, historic
                        // CJK Unified Ideographs Extension D 2B740–2B81F Uncommon, some in current use
                        // CJK Unified Ideographs Extension E 2B820–2CEAF Rare, historic
                        // CJK Unified Ideographs Extension F 2CEB0–2EBEF Rare, historic
                        // CJK Unified Ideographs Extension G 30000–3134F Rare, historic
                        // CJK Unified Ideographs Extension H 31350–323AF Rare, historic
                        // CJK Compatibility Ideographs F900–FAFF Duplicates, unifiable variants, corporate characters
                        // CJK Compatibility Ideographs Supplement 2F800–2FA1F Unifiable variants
                        //
                        // https://www.unicode.org/versions/Unicode15.0.0/ch18.pdf
                        // https://en.wikipedia.org/wiki/CJK_Unified_Ideographs
                        // https://www.unicode.org/versions/Unicode15.0.0/
                        //
                        // see also
                        // https://www.unicode.org/reports/tr31/tr31-37.html

                        name_buffer.push(*current_char);
                        self.next_char(); // consume char
                    }
                    '>' => {
                        // terminator char
                        break;
                    }
                    _ => {
                        return Err(AnreError::MessageWithPosition(
                            format!("Invalid character '{}' in capture group name.", current_char),
                            *self.peek_position(0).unwrap(),
                        ));
                    }
                },
                None => {
                    return Err(AnreError::UnexpectedEndOfDocument(
                        "Incomplete capture group name.".to_string(),
                    ));
                }
            }
        }

        self.consume_char_and_assert('>', "closing angle bracket")?;

        if name_buffer.is_empty() {
            return Err(AnreError::MessageWithRange(
                "Expected a capture group name.".to_string(),
                Range::new(&self.pop_position_from_stack(), &self.last_position),
            ));
        }

        self.pop_position_from_stack(); // discard the group name start position

        Ok(name_buffer)
    }

    fn lex_number_decimal(&mut self) -> Result<usize, AnreError> {
        // ```diagram
        // 123456T  //
        // ^     ^__// to here
        // |________// current char, validated
        //
        // T = not a number || EOF
        // ```

        let mut num_buffer = String::new();

        self.push_peek_position_into_stack();

        while let Some(current_char) = self.peek_char(0) {
            match current_char {
                '0'..='9' => {
                    num_buffer.push(*current_char);
                    self.next_char(); // consume digit
                }
                _ => {
                    break;
                }
            }
        }

        if num_buffer.is_empty() {
            return Err(AnreError::MessageWithPosition(
                "Expected a number.".to_string(),
                self.pop_position_from_stack(),
            ));
        }

        let num_range = Range::new(&self.pop_position_from_stack(), &self.last_position);

        let num_token = num_buffer.parse::<usize>().map_err(|_| {
            AnreError::MessageWithRange(
                format!("Cannot convert \"{}\" to an integer.", num_buffer),
                num_range,
            )
        })?;

        Ok(num_token)
    }

    fn lex_repetition(&mut self) -> Result<TokenWithRange, AnreError> {
        // ```diagram
        // {m,n}?  //
        // ^    ^__// to here
        // |_______// from here, validated
        // ```

        self.push_peek_position_into_stack();

        self.next_char(); // consume '{'

        let from = self.lex_number_decimal()?;

        let repetition = if self.peek_char_and_equals(0, ',') {
            self.next_char(); // consume ','

            if self.peek_char_and_equals(0, '}') {
                self.next_char(); // consume '}'
                Repetition::RepeatFrom(from)
            } else {
                let to = self.lex_number_decimal()?;
                // consume '}'
                self.consume_char_and_assert('}', "closing brace")?;
                Repetition::RepeatRange(from, to)
            }
        } else {
            // consume '}'
            self.consume_char_and_assert('}', "closing brace")?;
            Repetition::Repeat(from)
        };

        let is_lazy = if self.peek_char_and_equals(0, '?') {
            self.next_char(); // consume '?'
            true
        } else {
            false
        };

        let token = Token::Repetition(repetition, is_lazy);
        let range = Range::new(&self.pop_position_from_stack(), &self.last_position);

        Ok(TokenWithRange { token, range })
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        error::AnreError,
        position::Position,
        range::Range,
        traditional::token::{Repetition, Token, TokenWithRange},
    };

    use super::lex_from_str;

    fn lex_from_str_without_location(s: &str) -> Result<Vec<Token>, AnreError> {
        let tokens = lex_from_str(s)?
            .into_iter()
            .map(|e| e.token)
            .collect::<Vec<Token>>();
        Ok(tokens)
    }

    #[test]
    fn test_lex_char() {
        assert_eq!(
            lex_from_str_without_location("a").unwrap(),
            vec![Token::Char('a')]
        );

        assert_eq!(
            lex_from_str_without_location("(a)").unwrap(),
            vec![Token::GroupStart, Token::Char('a'), Token::GroupEnd]
        );

        assert_eq!(
            lex_from_str_without_location("az").unwrap(),
            vec![Token::Char('a'), Token::Char('z')]
        );

        // CJK
        assert_eq!(
            lex_from_str_without_location("文").unwrap(),
            vec![Token::Char('文')]
        );

        // emoji
        assert_eq!(
            lex_from_str_without_location("😊").unwrap(),
            vec![Token::Char('😊')]
        );

        // escape char `\\`
        assert_eq!(
            lex_from_str_without_location("\\\\").unwrap(),
            vec![Token::Char('\\')]
        );

        // escape char `\t`
        assert_eq!(
            lex_from_str_without_location("\\t").unwrap(),
            vec![Token::Char('\t')]
        );

        // escape char `\r`
        assert_eq!(
            lex_from_str_without_location("\\r").unwrap(),
            vec![Token::Char('\r')]
        );

        // escape char `\n`
        assert_eq!(
            lex_from_str_without_location("\\n").unwrap(),
            vec![Token::Char('\n')]
        );

    // Unicode escape sequence.
        assert_eq!(
            lex_from_str_without_location("\\u{2d}").unwrap(),
            vec![Token::Char('-')]
        );

    // Unicode escape sequence.
        assert_eq!(
            lex_from_str_without_location("\\u{6587}").unwrap(),
            vec![Token::Char('文')]
        );

        // escaped meta chars
        assert_eq!(
            lex_from_str_without_location(r#"\(\)\{\}\[\]\+\*\?\.\|\^\$"#).unwrap(),
            vec![
                Token::Char('('),
                Token::Char(')'),
                Token::Char('{'),
                Token::Char('}'),
                Token::Char('['),
                Token::Char(']'),
                Token::Char('+'),
                Token::Char('*'),
                Token::Char('?'),
                Token::Char('.'),
                Token::Char('|'),
                Token::Char('^'),
                Token::Char('$'),
            ]
        );

        // Test ranges.

        assert_eq!(
            lex_from_str(r#"a文😊\t\u{6587}"#).unwrap(),
            vec![
                TokenWithRange::new(Token::Char('a'), Range::from_detail(0, 0, 0, 1)),
                TokenWithRange::new(Token::Char('文'), Range::from_detail(1, 0, 1, 1)),
                TokenWithRange::new(Token::Char('😊'), Range::from_detail(2, 0, 2, 1)),
                TokenWithRange::new(Token::Char('\t'), Range::from_detail(3, 0, 3, 2)),
                TokenWithRange::new(Token::Char('文'), Range::from_detail(5, 0, 5, 8)),
            ]
        );

        // Error: unsupported escape char \v
        assert!(matches!(
            lex_from_str_without_location(r#"\v"#),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 0,
                        line: 0,
                        column: 0,
                    },
                    end_inclusive: Position {
                        index: 1,
                        line: 0,
                        column: 1,
                    },
                }
            ))
        ));

        // Error: unsupported hex escape "\x.."
        assert!(matches!(
            lex_from_str_without_location(r#"\x33"#),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 0,
                        line: 0,
                        column: 0,
                    },
                    end_inclusive: Position {
                        index: 1,
                        line: 0,
                        column: 1,
                    },
                }
            ))
        ));

        // Error: empty unicode escape string
        // "'\\u{}'"
        //  01 2345     // index
        assert!(matches!(
            lex_from_str_without_location("'\\u{}'"),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 3,
                        line: 0,
                        column: 3,
                    },
                    end_inclusive: Position {
                        index: 4,
                        line: 0,
                        column: 4,
                    }
                }
            ))
        ));

        // Error: invalid unicode code point, digits too much
        // "'\\u{10001111}'"
        //  01 234567890123    // index
        assert!(matches!(
            lex_from_str_without_location("'\\u{10001111}'"),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 3,
                        line: 0,
                        column: 3,
                    },
                    end_inclusive: Position {
                        index: 10,
                        line: 0,
                        column: 10,
                    }
                }
            ))
        ));

        // Error: invalid unicode code point, code point out of range
        // "'\\u{123456}'"
        //  01 2345678901
        assert!(matches!(
            lex_from_str_without_location("'\\u{123456}'"),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 3,
                        line: 0,
                        column: 3,
                    },
                    end_inclusive: Position {
                        index: 10,
                        line: 0,
                        column: 10,
                    }
                }
            ))
        ));

        // Error: invalid char in the unicode escape sequence
        assert!(matches!(
            lex_from_str_without_location("'\\u{12mn}''"),
            Err(AnreError::MessageWithPosition(
                _,
                Position {
                    index: 6,
                    line: 0,
                    column: 6,
                }
            ))
        ));

        // Error: missing the closed brace for unicode escape sequence
        assert!(matches!(
            lex_from_str_without_location("'\\u{1234'"),
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
            lex_from_str_without_location("'\\u{1234"),
            Err(AnreError::UnexpectedEndOfDocument(_))
        ));

        // Error: missing left brace for unicode escape sequence
        assert!(matches!(
            lex_from_str_without_location("'\\u1234}'"),
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
    fn test_lex_preset_charset() {
        assert_eq!(
            lex_from_str_without_location(r#"\d\D\w\W\s\S"#).unwrap(),
            vec![
                Token::PresetCharSet('d'),
                Token::PresetCharSet('D'),
                Token::PresetCharSet('w'),
                Token::PresetCharSet('W'),
                Token::PresetCharSet('s'),
                Token::PresetCharSet('S'),
            ]
        );
    }

    #[test]
    fn test_lex_charset() {
        assert_eq!(
            lex_from_str_without_location(r#"[a文😊]"#).unwrap(),
            vec![
                Token::CharSetStart,
                Token::Char('a'),
                Token::Char('文'),
                Token::Char('😊'),
                Token::CharSetEnd
            ]
        );

        assert_eq!(
            lex_from_str_without_location(r#"[^a]"#).unwrap(),
            vec![
                Token::CharSetStartNegative,
                Token::Char('a'),
                Token::CharSetEnd
            ]
        );

        // general escaped char
        assert_eq!(
            lex_from_str_without_location(r#"[\t\r\n\]\u{6587}]"#).unwrap(),
            vec![
                Token::CharSetStart,
                Token::Char('\t'),
                Token::Char('\r'),
                Token::Char('\n'),
                Token::Char(']'),
                Token::Char('文'),
                Token::CharSetEnd
            ]
        );

        // escaped meta chars
        // note: only ']' is necessary.
        assert_eq!(
            lex_from_str_without_location(r#"[\(\)\{\}\[\]\+\*\?\.\|\^\$\\]"#).unwrap(),
            vec![
                Token::CharSetStart,
                Token::Char('('),
                Token::Char(')'),
                Token::Char('{'),
                Token::Char('}'),
                Token::Char('['),
                Token::Char(']'),
                Token::Char('+'),
                Token::Char('*'),
                Token::Char('?'),
                Token::Char('.'),
                Token::Char('|'),
                Token::Char('^'),
                Token::Char('$'),
                Token::Char('\\'),
                Token::CharSetEnd
            ]
        );

        // meta chars in charset
        // note: only ']' and '\' are escaped
        assert_eq!(
            lex_from_str_without_location(r#"[(){}[\]+*?.|^$\\]"#).unwrap(),
            vec![
                Token::CharSetStart,
                Token::Char('('),
                Token::Char(')'),
                Token::Char('{'),
                Token::Char('}'),
                Token::Char('['),
                Token::Char(']'),
                Token::Char('+'),
                Token::Char('*'),
                Token::Char('?'),
                Token::Char('.'),
                Token::Char('|'),
                Token::Char('^'),
                Token::Char('$'),
                Token::Char('\\'),
                Token::CharSetEnd
            ]
        );

        // range
        assert_eq!(
            lex_from_str_without_location(r#"[-a-zA-Z0-9_-]"#).unwrap(),
            vec![
                Token::CharSetStart,
                Token::Char('-'),
                Token::CharRange('a', 'z'),
                Token::CharRange('A', 'Z'),
                Token::CharRange('0', '9'),
                Token::Char('_'),
                Token::Char('-'),
                Token::CharSetEnd
            ]
        );

        // preset charset
        assert_eq!(
            lex_from_str_without_location(r#"[\w\d\s]"#).unwrap(),
            vec![
                Token::CharSetStart,
                Token::PresetCharSet('w'),
                Token::PresetCharSet('d'),
                Token::PresetCharSet('s'),
                Token::CharSetEnd
            ]
        );

        // testing the ranges

        assert_eq!(
            lex_from_str(r#"[a文😊\t\u{5b57}0-9-]"#).unwrap(),
            //              012 3 456789012345678
            vec![
                TokenWithRange::new(Token::CharSetStart, Range::from_detail(0, 0, 0, 1)),
                TokenWithRange::new(Token::Char('a'), Range::from_detail(1, 0, 1, 1)),
                TokenWithRange::new(Token::Char('文'), Range::from_detail(2, 0, 2, 1)),
                TokenWithRange::new(Token::Char('😊'), Range::from_detail(3, 0, 3, 1)),
                TokenWithRange::new(Token::Char('\t'), Range::from_detail(4, 0, 4, 2)),
                TokenWithRange::new(Token::Char('字'), Range::from_detail(6, 0, 6, 8)),
                TokenWithRange::new(Token::CharRange('0', '9'), Range::from_detail(14, 0, 14, 3)),
                TokenWithRange::new(Token::Char('-'), Range::from_detail(17, 0, 17, 1)),
                TokenWithRange::new(Token::CharSetEnd, Range::from_detail(18, 0, 18, 1)),
            ]
        );

        // Error: missing ']'
        assert!(matches!(
            lex_from_str_without_location(r#"[abc"#),
            Err(AnreError::UnexpectedEndOfDocument(_))
        ));

        // Error: negative preset charset
        assert!(matches!(
            lex_from_str_without_location(r#"[ab\Wcd]"#),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 3,
                        line: 0,
                        column: 3,
                    },
                    end_inclusive: Position {
                        index: 4,
                        line: 0,
                        column: 4,
                    },
                }
            ))
        ));

        // Error: does not support word boundary assertions within charset
        assert!(matches!(
            lex_from_str_without_location(r#"[\b]"#),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 1,
                        line: 0,
                        column: 1,
                    },
                    end_inclusive: Position {
                        index: 2,
                        line: 0,
                        column: 2,
                    }
                }
            ))
        ));

        // Error: unsupported escape char
        assert!(matches!(
            lex_from_str_without_location(r#"[\v]"#),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 1,
                        line: 0,
                        column: 1,
                    },
                    end_inclusive: Position {
                        index: 2,
                        line: 0,
                        column: 2,
                    }
                }
            ))
        ));

        // Error: unsupported back reference - number
        assert!(matches!(
            lex_from_str_without_location(r#"[\1]"#),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 1,
                        line: 0,
                        column: 1,
                    },
                    end_inclusive: Position {
                        index: 2,
                        line: 0,
                        column: 2,
                    }
                }
            ))
        ));

        // Error: unsupported back reference - name
        assert!(matches!(
            lex_from_str_without_location(r#"[\k<name>]"#),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 1,
                        line: 0,
                        column: 1,
                    },
                    end_inclusive: Position {
                        index: 2,
                        line: 0,
                        column: 2,
                    }
                }
            ))
        ));
    }

    #[test]
    fn test_lex_notations() {
        assert_eq!(
            lex_from_str_without_location(r#"a?b??c+d+?e*f*?"#).unwrap(),
            vec![
                Token::Char('a'),
                Token::Optional,
                Token::Char('b'),
                Token::LazyOptional,
                Token::Char('c'),
                Token::OneOrMore,
                Token::Char('d'),
                Token::LazyOneOrMore,
                Token::Char('e'),
                Token::ZeroOrMore,
                Token::Char('f'),
                Token::LazyZeroOrMore,
            ]
        );

        // location
        assert_eq!(
            lex_from_str(r#"a+b+?"#).unwrap(),
            vec![
                TokenWithRange::new(Token::Char('a'), Range::from_detail(0, 0, 0, 1)),
                TokenWithRange::new(Token::OneOrMore, Range::from_detail(1, 0, 1, 1)),
                TokenWithRange::new(Token::Char('b'), Range::from_detail(2, 0, 2, 1)),
                TokenWithRange::new(Token::LazyOneOrMore, Range::from_detail(3, 0, 3, 2)),
            ]
        );
    }

    #[test]
    fn test_lex_line_boundary_assertions() {
        assert_eq!(
            lex_from_str(r#"^a$"#).unwrap(),
            vec![
                TokenWithRange::new(Token::LineBoundaryAssertionStart, Range::from_detail(0, 0, 0, 1),),
                TokenWithRange::new(Token::Char('a'), Range::from_detail(1, 0, 1, 1),),
                TokenWithRange::new(Token::LineBoundaryAssertionEnd, Range::from_detail(2, 0, 2, 1),),
            ]
        );
    }

    #[test]
    fn test_lex_word_boundary_assertions() {
        assert_eq!(
            lex_from_str(r#"\ba\B"#).unwrap(),
            vec![
                TokenWithRange::new(
                    Token::WordBoundaryAssertion(false),
                    Range::from_detail(0, 0, 0, 2)
                ),
                TokenWithRange::new(Token::Char('a'), Range::from_detail(2, 0, 2, 1)),
                TokenWithRange::new(
                    Token::WordBoundaryAssertion(true),
                    Range::from_detail(3, 0, 3, 2)
                ),
            ]
        );
    }

    #[test]
    fn test_lex_repetition() {
        assert_eq!(
            lex_from_str(r#"{3}{5,}{7,13}"#).unwrap(),
            vec![
                TokenWithRange::new(
                    Token::Repetition(Repetition::Repeat(3), false),
                    Range::from_detail(0, 0, 0, 3)
                ),
                TokenWithRange::new(
                    Token::Repetition(Repetition::RepeatFrom(5), false),
                    Range::from_detail(3, 0, 3, 4)
                ),
                TokenWithRange::new(
                    Token::Repetition(Repetition::RepeatRange(7, 13), false),
                    Range::from_detail(7, 0, 7, 6)
                ),
            ]
        );

        assert_eq!(
            lex_from_str(r#"{3}?{5,}?{7,13}?"#).unwrap(),
            vec![
                TokenWithRange::new(
                    Token::Repetition(Repetition::Repeat(3), true),
                    Range::from_detail(0, 0, 0, 4)
                ),
                TokenWithRange::new(
                    Token::Repetition(Repetition::RepeatFrom(5), true),
                    Range::from_detail(4, 0, 4, 5)
                ),
                TokenWithRange::new(
                    Token::Repetition(Repetition::RepeatRange(7, 13), true),
                    Range::from_detail(9, 0, 9, 7)
                ),
            ]
        );

        // Error: missing number
        assert!(matches!(
            lex_from_str(r#"{}"#),
            Err(AnreError::MessageWithPosition(
                _,
                Position {
                    index: 1,
                    line: 0,
                    column: 1,
                }
            ))
        ));

        // Error: expect a number
        assert!(matches!(
            lex_from_str(r#"{a}"#),
            Err(AnreError::MessageWithPosition(
                _,
                Position {
                    index: 1,
                    line: 0,
                    column: 1,
                }
            ))
        ));

        // Error: incomplete
        assert!(matches!(
            lex_from_str(r#"{1"#),
            Err(AnreError::UnexpectedEndOfDocument(_,))
        ));

        // Error: expect a number
        assert!(matches!(
            lex_from_str(r#"{1,a}"#),
            Err(AnreError::MessageWithPosition(
                _,
                Position {
                    index: 3,
                    line: 0,
                    column: 3,
                }
            ))
        ));

        // Error: incomplete
        assert!(matches!(
            lex_from_str(r#"{1,3"#),
            Err(AnreError::UnexpectedEndOfDocument(_))
        ));
    }

    #[test]
    fn test_logic_or() {
        assert_eq!(
            lex_from_str(r#"a|b"#).unwrap(),
            vec![
                TokenWithRange::new(Token::Char('a'), Range::from_detail(0, 0, 0, 1)),
                TokenWithRange::new(Token::LogicOr, Range::from_detail(1, 0, 1, 1)),
                TokenWithRange::new(Token::Char('b'), Range::from_detail(2, 0, 2, 1)),
            ]
        );
    }

    #[test]
    fn test_group() {
        assert_eq!(
            lex_from_str(r#"(a)(?:b)(?<c>d)"#).unwrap(),
            vec![
                TokenWithRange::new(Token::GroupStart, Range::from_detail(0, 0, 0, 1)),
                TokenWithRange::new(Token::Char('a'), Range::from_detail(1, 0, 1, 1)),
                TokenWithRange::new(Token::GroupEnd, Range::from_detail(2, 0, 2, 1)),
                // non-capturing group
                TokenWithRange::new(Token::NonCaptureGroupStart, Range::from_detail(3, 0, 3, 3)),
                TokenWithRange::new(Token::Char('b'), Range::from_detail(6, 0, 6, 1)),
                TokenWithRange::new(Token::GroupEnd, Range::from_detail(7, 0, 7, 1)),
                // named group
                TokenWithRange::new(
                    Token::NamedCaptureGroupStart("c".to_string()),
                    Range::from_detail(8, 0, 8, 5)
                ),
                TokenWithRange::new(Token::Char('d'), Range::from_detail(13, 0, 13, 1)),
                TokenWithRange::new(Token::GroupEnd, Range::from_detail(14, 0, 14, 1)),
            ]
        );

        // Error: invalid group syntax
        assert!(matches!(
            lex_from_str(r#"(?abc)"#),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 0,
                        line: 0,
                        column: 0,
                    },
                    end_inclusive: Position {
                        index: 2,
                        line: 0,
                        column: 2,
                    }
                }
            ))
        ));

        // Error: missing identifier for named group
        assert!(matches!(
            lex_from_str(r#"(?<>abc)"#),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 2,
                        line: 0,
                        column: 2,
                    },
                    end_inclusive: Position {
                        index: 3,
                        line: 0,
                        column: 3,
                    }
                }
            ))
        ));
    }

    #[test]
    fn test_back_reference() {
        assert_eq!(
            lex_from_str(r#"\1\k<e>"#).unwrap(),
            vec![
                // back reference - number
                TokenWithRange::new(
                    Token::BackReferenceNumber(1),
                    Range::from_detail(0, 0, 0, 2)
                ),
                // back reference - name
                TokenWithRange::new(
                    Token::BackReferenceName("e".to_string()),
                    Range::from_detail(2, 0, 2, 5)
                ),
            ]
        );

        // Error: back reference to group 0
        assert!(matches!(
            lex_from_str(r#"(a)b\0"#),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 4,
                        line: 0,
                        column: 4,
                    },
                    end_inclusive: Position {
                        index: 5,
                        line: 0,
                        column: 5,
                    }
                }
            ))
        ));

        // Error: missing identifier for named back reference
        assert!(matches!(
            lex_from_str(r#"\k<>)"#),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 2,
                        line: 0,
                        column: 2,
                    },
                    end_inclusive: Position {
                        index: 3,
                        line: 0,
                        column: 3,
                    }
                }
            ))
        ));

        // Error: missing '>' for named back reference
        assert!(matches!(
            lex_from_str(r#"\k<abc"#),
            Err(AnreError::UnexpectedEndOfDocument(_))
        ));

        // Error: missing '<' for named back reference
        assert!(matches!(
            lex_from_str(r#"\kabc>"#),
            Err(AnreError::MessageWithRange(
                _,
                Range {
                    start: Position {
                        index: 0,
                        line: 0,
                        column: 0,
                    },
                    end_inclusive: Position {
                        index: 2,
                        line: 0,
                        column: 2,
                    }
                }
            ))
        ));
    }

    #[test]
    fn test_look_around_assertions() {
        assert_eq!(
            lex_from_str(r#"(?=a)(?!b)(?<=c)(?<!d)"#).unwrap(),
            vec![
                // look-ahead
                TokenWithRange::new(Token::LookAheadGroupStart, Range::from_detail(0, 0, 0, 3)),
                TokenWithRange::new(Token::Char('a'), Range::from_detail(3, 0, 3, 1)),
                TokenWithRange::new(Token::GroupEnd, Range::from_detail(4, 0, 4, 1)),
                // look-ahead - negative
                TokenWithRange::new(
                    Token::LookAheadNegativeGroupStart,
                    Range::from_detail(5, 0, 5, 3)
                ),
                TokenWithRange::new(Token::Char('b'), Range::from_detail(8, 0, 8, 1)),
                TokenWithRange::new(Token::GroupEnd, Range::from_detail(9, 0, 9, 1)),
                // look-behind
                TokenWithRange::new(
                    Token::LookBehindGroupStart,
                    Range::from_detail(10, 0, 10, 4)
                ),
                TokenWithRange::new(Token::Char('c'), Range::from_detail(14, 0, 14, 1)),
                TokenWithRange::new(Token::GroupEnd, Range::from_detail(15, 0, 15, 1)),
                // look-behind - negative
                TokenWithRange::new(
                    Token::LookBehindNegativeGroupStart,
                    Range::from_detail(16, 0, 16, 4)
                ),
                TokenWithRange::new(Token::Char('d'), Range::from_detail(20, 0, 20, 1)),
                TokenWithRange::new(Token::GroupEnd, Range::from_detail(21, 0, 21, 1)),
            ]
        );
    }
}
