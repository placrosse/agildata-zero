#[macro_use(lazy_static)]
use std::iter::Peekable;
use std::str::Chars;
use std::collections::HashSet;

#[derive(Debug,PartialEq)]
pub enum Token {
    Whitespace,
    Keyword(String),
    Identifier(String),
    LiteralString(String),
    LiteralLong(String),
    Operator(String),
    Comma
}
static KEYWORDS: &'static [&'static str] = &["SELECT", "FROM"];

// lazy_static! {
//     static ref KEYWORDS: HashSet<String> = vec!("SELECT", "FROM").into_iter().collect();
// }

fn next_token(it: &mut Peekable<Chars>) -> Result<Option<Token>, &'static str> {

    match it.peek() {
        Some(&ch) => match ch {
            ' ' | '\t' | '\n' => {
                it.next(); // consumer the char
                Ok(Some(Token::Whitespace))
            },
            '+' | '-' | '/' | '*' | '%' => {
                Ok(Some(Token::Operator(it.next().unwrap().to_string())))
            },
            '0'...'9' => {
                Ok(Some(Token::LiteralLong(it
                    .take_while(|ch| ch.is_numeric())
                    .map(|ch| ch.to_string())
                    .collect()
                )))
            },
            'a'...'z' | 'A'...'Z' => {
                let text = it
                    .take_while(|ch| ch.is_alphabetic())
                    .map(|ch| ch.to_string())
                    .collect::<String>()
                    .to_uppercase();
                if KEYWORDS.iter().position(|&r| r == text).is_none() {
                    Ok(Some(Token::Identifier(text)))
                } else {
                    Ok(Some(Token::Keyword(text)))
                }
            }
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

}
