use std::iter::Peekable;
use std::str::Chars;
use std::fmt::Write;
use std::sync::atomic::{AtomicU32, Ordering};

use std::ascii::AsciiExt;

use helper::DoesContain;

#[derive(Debug,PartialEq,Clone)]
pub enum Token {
    Whitespace,
    Keyword(String),
    Identifier(String),
    Literal(LiteralToken),
    Operator(String),
    Punctuator(String)
}
#[derive(Debug,PartialEq,Clone)]
pub enum LiteralToken {
    LiteralString(u32, String),
    LiteralLong(u32, String),
    LiteralDouble(u32, String),
    LiteralBool(u32, String),
}

static KEYWORDS: &'static [&'static str] = &["SELECT", "FROM", "WHERE", "AND", "OR", "UNION", "FROM", "AS",
    "WHERE", "ORDER", "BY", "HAVING", "GROUP", "ASC", "DESC", "JOIN", "INNER", "LEFT", "RIGHT", "CROSS",
    "FULL", "ON"];

fn next_token(it: &mut Peekable<Chars>, lit_index: &AtomicU32) -> Result<Option<Token>, &'static str> {

    match it.peek() {
        Some(&ch) => match ch {
            ' ' | '\t' | '\n' => {
                it.next(); // consumer the char
                Ok(Some(Token::Whitespace))
            },
            '+' | '-' | '/' | '*' | '%' | '=' => {
                it.next(); // consume one
                Ok(Some(Token::Operator(ch.to_string()))) // after consume because return val
            },
            '>' | '<' | '!' => {

            // !: possibly unsafe. We are mutating a string may not be copied from the Iterator
            // Better to do something else. Possibly let op = String::new().push(c);

                let mut op = it.next().unwrap().to_string();

                match it.peek() {
                    Some(&c) => match c {
                        '=' => {
                            op.push(c);
                            it.next(); // consume one
                        }
                        _ => {}
                    },
                    None => panic!("Expected token received None")
                }
                Ok(Some(Token::Operator(op)))
            },
            '0'...'9' | '.' => {
                let mut text = String::new();
                while let Some(&c) = it.peek() { // will break when it.peek() => None

                    if c.is_numeric() || '.' == c  {
                        text.push(c);
                    } else {
                        break; // leave the loop early
                    }

                    it.next(); // consume one
                }

                if text.as_str().contains('.') {
                    Ok(Some(Token::Literal(LiteralToken::LiteralDouble(lit_index.fetch_add(1, Ordering::SeqCst), text))))
                } else {
                    Ok(Some(Token::Literal(LiteralToken::LiteralLong(lit_index.fetch_add(1, Ordering::SeqCst), text))))
                }
            },
            'a'...'z' | 'A'...'Z' => {
                let mut text = String::new();
                while let Some(&c) = it.peek() { // will break when it.peek() => None

                    if c.is_alphabetic() || c == '.' {
                        text.push(c);
                    } else {
                        break; // leave the loop early
                    }

                    it.next(); // consume one
                }

                if "true".eq_ignore_ascii_case(&text) || "false".eq_ignore_ascii_case(&text) {
                    Ok(Some(Token::Literal(LiteralToken::LiteralBool(lit_index.fetch_add(1, Ordering::SeqCst), text))))
                } else if KEYWORDS.iter().position(|&r| r.eq_ignore_ascii_case(&text)).is_none() {
                    Ok(Some(Token::Identifier(text)))
                } else if "AND".eq_ignore_ascii_case(&text) || "OR".eq_ignore_ascii_case(&text) {
                    Ok(Some(Token::Operator(text)))
                } else {
                    Ok(Some(Token::Keyword(text)))
                }
            }
            '\'' => {
                it.next();
                let mut s = String::new();
                loop {
                    match it.peek() {
                        Some(&c) => match c {
                            '\\' => {
                                s.push(c);
                                it.next();
                                match it.peek() {
                                    Some(&n) => match n {
                                        '\'' => {
                                            s.push(n);
                                            it.next();
                                        },
                                        _ => continue,
                                    },
                                    None => panic!("Unexpected end of string")
                                }
                            },
                            '\'' => {
                                it.next();
                                break;
                            },
                            _ => {
                                s.push(c);
                                it.next();
                            }
                        },
                        None => panic!("Unexpected end of string")
                    }
                }

                Ok(Some(Token::Literal(LiteralToken::LiteralString(lit_index.fetch_add(1, Ordering::SeqCst), s))))
            },
            ',' | '(' | ')' => {
                it.next();
                Ok(Some(Token::Punctuator(ch.to_string())))
            },
            // just playing around ...
            _ => {
                panic!("Unsupported char {:?}", ch)
            }
        },
        None => Ok(None),
    }
}

pub trait Tokenizer {
    fn tokenize(&self) -> Result<Vec<Token>, &'static str>;
}

impl Tokenizer for String {

    fn tokenize(&self) -> Result<Vec<Token>, &'static str> {

        let mut it = self.chars().peekable();
        let mut stream: Vec<Token> = Vec::new();
        let mut literal_index = AtomicU32::new(0);
        loop {
            match next_token(&mut it, &literal_index) {
                Ok(Some(token)) => stream.push(token),
                Ok(None) =>
                    return Ok(stream
                    .into_iter()
                    .filter(|t| match t { &Token::Whitespace => false, _ => true })
                    .collect::<Vec<_>>()
                ),
                Err(e) => return Err(e),
            }
        }
    }

}

pub struct Tokens {
    pub tokens: Vec<Token>,
    pub index: usize,
}

impl Iterator for Tokens {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        // TODO clone?
        if self.tokens.len() > self.index {
            let result = self.tokens[self.index].clone();
            self.index += 1;
            Some(result)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Token, Tokenizer};
    use super::Token::*;
    use super::LiteralToken;

    #[test]
    fn simple_tokenize() {
        assert_eq!(
            vec![Keyword("SELECT".to_string()),
                Literal(LiteralToken::LiteralLong(0, "1".to_string())),
                Operator("+".to_string()),
                Literal(LiteralToken::LiteralLong(1, "1".to_string()))],
            String::from("SELECT 1 + 1").tokenize().unwrap()
        );
    }

    #[test]
    fn complex_tokenize() {
        assert_eq!(
            vec![Keyword("SELECT".to_string()),
                Identifier("a".to_string()),
                Punctuator(",".to_string()),
                Literal(LiteralToken::LiteralString(0, "hello".to_string())),
                Keyword("FROM".to_string()),
                Identifier("tOne".to_string()),
                Keyword("WHERE".to_string()),
                Identifier("b".to_string()),
                Operator(">".to_string()),
                Literal(LiteralToken::LiteralDouble(1, "2.22".to_string())),
                Operator("AND".to_string()),
                Identifier("c".to_string()),
                Operator("!=".to_string()),
                Literal(LiteralToken::LiteralBool(2, "true".to_string()))],
            String::from("SELECT a, 'hello' FROM tOne WHERE b > 2.22 AND c != true").tokenize().unwrap()
        );
    }

}
