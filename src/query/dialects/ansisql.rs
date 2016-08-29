use super::super::*;


use std::iter::Peekable;
use std::str::Chars;
use std::sync::atomic::{AtomicU32, Ordering};
use std::ascii::AsciiExt;

static KEYWORDS: &'static [&'static str] = &["SELECT", "FROM", "WHERE", "AND", "OR", "UNION", "FROM", "AS",
    "WHERE", "ORDER", "BY", "HAVING", "GROUP", "ASC", "DESC", "JOIN", "INNER", "LEFT", "RIGHT", "CROSS",
    "FULL", "ON", "INSERT", "UPDATE", "SET", "VALUES", "INTO", "SHOW", "CREATE", "TABLE", "PRECISION",
    "PRIMARY", "KEY", "UNIQUE", "FULLTEXT", "FOREIGN", "REFERENCES", "CONSTRAINT"];

pub struct AnsiSQLDialect {
	lit_index: AtomicU32
}

#[derive(Debug,PartialEq,Clone)]
pub enum SQLToken {
	Literal(LiteralToken)
}

#[derive(Debug,PartialEq,Clone)]
pub enum LiteralToken {
    LiteralString(u32, String),
    LiteralLong(u32, String),
    LiteralDouble(u32, String),
    LiteralBool(u32, String),
}

impl IToken for SQLToken {}

impl AnsiSQLDialect {
	pub fn new() -> Self {AnsiSQLDialect{lit_index: AtomicU32::new(0)}}
}

pub enum SQLAST {}
impl IAST for SQLAST{}

pub enum SQLRel {}
impl IRel for SQLRel {}

impl Dialect<SQLToken, SQLAST, SQLRel> for AnsiSQLDialect {

	fn get_token(&self, chars: &mut Peekable<Chars>) -> Result<Option<Token<SQLToken>>, String> {
		match chars.peek() {
	        Some(&ch) => match ch {
	            ' ' | '\t' | '\n' => {
	                chars.next(); // consumer the char
	                Ok(Some(Token::Whitespace))
	            },
	            '+' | '-' | '/' | '*' | '%' | '=' => {
	                chars.next(); // consume one
	                Ok(Some(Token::Operator(ch.to_string()))) // after consume because return val
	            },
	            '>' | '<' | '!' => {

	                let mut op = chars.next().unwrap().to_string();

	                match chars.peek() {
	                    Some(&c) => match c {
	                        '=' => {
	                            op.push(c);
	                            chars.next(); // consume one
	                        }
	                        _ => {}
	                    },
	                    None => return Err(String::from("Expected token received None"))
	                }
	                Ok(Some(Token::Operator(op)))
	            },
	            '0'...'9' | '.' => {
	                let mut text = String::new();
	                while let Some(&c) = chars.peek() { // will break when it.peek() => None

	                    if c.is_numeric() || '.' == c  {
	                        text.push(c);
	                    } else {
	                        break; // leave the loop early
	                    }

	                    chars.next(); // consume one
	                }

	                if text.as_str().contains('.') {
						let token = SQLToken::Literal(LiteralToken::LiteralDouble(self.lit_index.fetch_add(1, Ordering::SeqCst), text));
	                    Ok(Some(
							Token::TokenExtension(token)
						))
	                } else {
						let token = SQLToken::Literal(LiteralToken::LiteralLong(self.lit_index.fetch_add(1, Ordering::SeqCst), text));
	                    Ok(Some(Token::TokenExtension(token)))
	                }
	            },
	            'a'...'z' | 'A'...'Z' => { // TODO this should really be any valid char for an identifier..
	                let mut text = String::new();
	                while let Some(&c) = chars.peek() { // will break when it.peek() => None

	                    if c.is_alphabetic() || c.is_numeric() || c == '.' || c == '_' {
	                        text.push(c);
	                    } else {
	                        break; // leave the loop early
	                    }

	                    chars.next(); // consume one
	                }

	                if "true".eq_ignore_ascii_case(&text) || "false".eq_ignore_ascii_case(&text) {
	                    let token = SQLToken::Literal(LiteralToken::LiteralBool(self.lit_index.fetch_add(1, Ordering::SeqCst), text));
						Ok(Some(Token::TokenExtension(token)))
	                } else if KEYWORDS.iter().position(|&r| r.eq_ignore_ascii_case(&text)).is_none() {
	                    Ok(Some(Token::Identifier(text)))
	                } else if "AND".eq_ignore_ascii_case(&text) || "OR".eq_ignore_ascii_case(&text) {
	                    Ok(Some(Token::Operator(text)))
	                } else {
	                    Ok(Some(Token::Keyword(text.to_uppercase())))
	                }
	            },
	            '\'' => {
	                chars.next();
	                let mut s = String::new();
	                loop {
	                    match chars.peek() {
	                        Some(&c) => match c {
	                            '\\' => {
	                                s.push(c);
	                                chars.next();
	                                match chars.peek() {
	                                    Some(&n) => match n {
	                                        '\'' => {
	                                            s.push(n);
	                                            chars.next();
	                                        },
	                                        _ => continue,
	                                    },
	                                    None => return Err(String::from("Unexpected end of string"))
	                                }
	                            },
	                            '\'' => {
	                                chars.next();
	                                break;
	                            },
	                            _ => {
	                                s.push(c);
	                                chars.next();
	                            }
	                        },
	                        None => return Err(String::from("Unexpected end of string"))
	                    }
	                }

					let token = SQLToken::Literal(LiteralToken::LiteralString(self.lit_index.fetch_add(1, Ordering::SeqCst), s));
					Ok(Some(Token::TokenExtension(token)))
	            },
	            ',' | '(' | ')' => {
	                chars.next();
	                Ok(Some(Token::Punctuator(ch.to_string())))
	            },
	            _ => {
	                Err(format!("Unsupported char {:?}", ch))
	            }
	        },
	        None => Ok(None),
	    }
	}

    // fn parse_prefix<It: Iterator<Item=Token<SQLToken>>>(&self, parser: &PrattParser<SQLToken, SQLAST, SQLRel>, tokens: It) -> Result<Option<ASTNode<SQLAST>>, String> {
    //     Err(String::from("parse_prefix() Not implemented!"))
    // }

}
