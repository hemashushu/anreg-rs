// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::{
    location::Location,
    peekableiter::PeekableIter,
    token::{Token, TokenWithRange},
};

pub fn clean(tokens: Vec<TokenWithRange>) -> Vec<TokenWithRange> {
    // remove all comments.
    let mut token_iter = tokens.into_iter();
    let mut peekable_token_iter = PeekableIter::new(&mut token_iter, 1);
    let mut clean_tokens: Vec<TokenWithRange> = vec![];

    while let Some(tr) = peekable_token_iter.next() {
        match tr {
            TokenWithRange {
                token: Token::Comment(_),
                ..
            } => {
                // consume comments
            }
            _ => {
                clean_tokens.push(tr);
            }
        }
    }

    clean_tokens
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        charposition::CharsWithPositionIter,
        error::Error,
        lexer::Lexer,
        location::Location,
        peekableiter::PeekableIter,
        token::{Token, TokenWithRange},
    };

    use super::clean;

    fn lex_str_to_vec_with_range(s: &str) -> Result<Vec<TokenWithRange>, Error> {
        let mut chars = s.chars();
        let mut char_position_iter = CharsWithPositionIter::new(0, &mut chars);
        let mut peekable_char_position_iter = PeekableIter::new(&mut char_position_iter, 3);
        let mut lexer = Lexer::new(&mut peekable_char_position_iter);
        let tokens = lexer.lex()?;
        let clean_tokens = clean(tokens);
        Ok(clean_tokens)
    }

    fn lex_str_to_vec(s: &str) -> Result<Vec<Token>, Error> {
        let tokens = lex_str_to_vec_with_range(s)?
            .into_iter()
            .map(|e| e.token)
            .collect::<Vec<Token>>();
        Ok(tokens)
    }

    #[test]
    fn test_clear_comments() {
        assert_eq!(
            lex_str_to_vec(
                r#"'1' // line comment 1
                // line comment 2
                '2' /* block comment 1 */
                /*
                block comment 2
                */
                '3'
                "#
            )
            .unwrap(),
            vec![
                Token::Char('1'),
                Token::NewLine,
                Token::NewLine,
                Token::Char('2'),
                Token::NewLine,
                Token::NewLine,
                Token::Char('3'),
                Token::NewLine,
            ]
        );
    }
}
