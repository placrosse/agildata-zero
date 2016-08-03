use std::iter::Peekable;
use std::str::Chars;
use std::fmt::Write;

use std::ascii::AsciiExt;

use helper::DoesContain;

#[derive(Debug,PartialEq)]
pub enum Token {
    Whitespace,
    Keyword(String),
    Identifier(String),
    LiteralString(String),
    LiteralLong(String),
    LiteralDouble(String),
    LiteralBool(String),
    Operator(String),
    Punctuator(String)
}
static KEYWORDS: &'static [&'static str] = &["SELECT", "FROM", "WHERE", "AND", "OR"];

fn next_token(it: &mut Peekable<Chars>) -> Result<Option<Token>, &'static str> {

    match it.peek() {
        Some(&ch) => match ch {
            ' ' | '\t' | '\n' => {
                it.next(); // consumer the char
                Ok(Some(Token::Whitespace))
            },
            '+' | '-' | '/' | '*' | '%' => {
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
                            //let tail = it.next().unwrap().to_string();
                            //op.push_str(&tail);
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

                // Old loop
                // loop {
                //     //write!(&mut text, "{}", it.next().unwrap().to_string()).unwrap();
                //     match it.peek() {
                //         Some(&c) => {
                //             if c.is_numeric() || '.'.eq(&c) {
                //                 write!(&mut text, "{}", it.next().unwrap().to_string()).unwrap();
                //             } else {
                //                 break;
                //             }
                //         }
                //         None => break
                //     }
                // }

                // New loop:
                while let Some(&c) = it.peek() { // will break when it.peek() => None

                    if c.is_numeric() || '.' == c  {
                        text.push(c);
                    } else {
                        break; // leave the loop early
                    }

                    it.next(); // consume one
                }

                // let text: String = it
                //     .take_while(|ch| ch.is_numeric() || '.'.eq(ch))
                //     .map(|ch| ch.to_string())
                //     .collect();

                if text.as_str().contains('.') {
                    Ok(Some(Token::LiteralDouble(text)))
                } else {
                    Ok(Some(Token::LiteralLong(text)))
                }
            },
            'a'...'z' | 'A'...'Z' => {
                let mut text = String::new();
                // Old loop:
                // loop {
                //     //write!(&mut text, "{}", it.next().unwrap().to_string()).unwrap();
                //     match it.peek() {
                //         Some(&c) => {
                //             if c.is_alphabetic() {
                //                 write!(&mut text, "{}", it.next().unwrap().to_string()).unwrap();
                //             } else {
                //                 break;
                //             }
                //         }
                //         None => break
                //     }
                // }

                // New loop:
                while let Some(&c) = it.peek() { // will break when it.peek() => None

                    if c.is_alphabetic() {
                        text.push(c);
                    } else {
                        break; // leave the loop early
                    }

                    it.next(); // consume one
                }

                // let text: String = it
                //     .take_while(|ch| {
                //         println!("Taking {:?}", ch);
                //         println!("is alphabetic? {:?}", ch.is_alphabetic());
                //         ch.is_alphabetic()
                //     })
                //     .map(|ch| ch.to_string())
                //     .collect();

                if "true".eq_ignore_ascii_case(&text) || "false".eq_ignore_ascii_case(&text) {
                    Ok(Some(Token::LiteralBool(text)))
                } else if KEYWORDS.iter().position(|&r| r.eq_ignore_ascii_case(&text)).is_none() {
                    Ok(Some(Token::Identifier(text)))
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
                                //write!(&mut s, "{}", it.next().unwrap()).unwrap();
                                s.push(c); // No need to unwrap again! value already unwrapped
                                it.next();
                                match it.peek() {
                                    Some(&n) => match n {
                                        '\'' => {
                                            //write!(&mut s, "{}", it.next().unwrap()).unwrap();
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
                                //write!(&mut s, "{}", it.next().unwrap()).unwrap();
                                s.push(c);
                                it.next();
                            }
                        },
                        None => panic!("Unexpected end of string")
                    }
                }
                Ok(Some(Token::LiteralString(s)))
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

        loop {
            match next_token(&mut it) {
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

#[cfg(test)]
mod tests {
    use super::{Token, Tokenizer};
    use super::Token::*;

    #[test]
    fn simple_tokenize() {
        assert_eq!(
            vec![Keyword("SELECT".to_string()),
                LiteralLong("1".to_string()),
                Operator("+".to_string()),
                LiteralLong("1".to_string())],
            String::from("SELECT 1 + 1").tokenize().unwrap()
        );
    }

    #[test]
    fn complex_tokenize() {
        assert_eq!(
            vec![Keyword("SELECT".to_string()),
                Identifier("a".to_string()),
                Punctuator(",".to_string()),
                LiteralString("hello".to_string()),
                Keyword("FROM".to_string()),
                Identifier("tOne".to_string()),
                Keyword("WHERE".to_string()),
                Identifier("b".to_string()),
                Operator(">".to_string()),
                LiteralDouble("2.22".to_string()),
                Keyword("AND".to_string()),
                Identifier("c".to_string()),
                Operator("!=".to_string()),
                LiteralBool("true".to_string())],
            String::from("SELECT a, 'hello' FROM tOne WHERE b > 2.22 AND c != true").tokenize().unwrap()
        );
    }

}
