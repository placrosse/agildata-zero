use std::ops::BitAnd;

use std::vec;
use std::iter::Peekable;
use std::str::Chars;

use std::convert::AsRef;

#[derive(Debug,PartialEq)]
pub enum Token {
    Whitespace,
    Keyword(String),
    Identifier(String),
    LiteralString(String),
    LiteralInt(i32),
    Operator(String),
    Comma
}

#[derive(Debug)]
pub enum TokenizerError {
    InvalidToken(String),
    NoMoreTokens
}

fn parse_token(it: &mut Peekable<Chars>) -> Result<Option<Token>, TokenizerError> {
    println!("parse_token()");

    match it.peek().cloned() {
        Some(ch) => match ch {
            ' ' | '\t' | '\n' => {
                it.next();
                Ok(Some(Token::Whitespace))
            },
            '=' | '+' | '-' | '*' | '/' | '%' => Ok(Some(Token::Operator(it.next().unwrap().to_string()))),
            '0'...'9' => {
                let text: String = it
                    .take_while(|ch| ch.is_numeric())
                    .map(|ch| ch.to_string())
                    .collect();
                    Ok(Some(Token::LiteralInt(text.parse::<i32>().unwrap())))
            },
            'a'...'z' | 'A'...'Z' => {
                let text: String = it
                    .take_while(|ch| ch.is_alphabetic())
                    .map(|ch| ch.to_string())
                    .collect();
                match text.as_ref() {
                    "SELECT" | "INSERT" => Ok(Some(Token::Keyword(text))),
                    _ => Ok(Some(Token::LiteralString(text)))
                }
            },
            _ => Err(TokenizerError::InvalidToken(ch.to_string()))
        },
        None => Ok(None)
    }
}

pub trait Tokenizer {
    fn tokenize(&self) -> Result<Vec<Token>, TokenizerError>;
}

impl Tokenizer for String {

    fn tokenize(&self) -> Result<Vec<Token>, TokenizerError> {

        let mut tokens: Vec<Token> = Vec::new();
        let mut it = self.chars().peekable();
        let mut error: Option<TokenizerError> = None;

        // iterate as long as there are tokens left
        loop {
            match parse_token(&mut it) {
                Ok(Some(token)) => tokens.push(token),
                Ok(None) => return Ok(tokens
                    .into_iter()
                    .filter(|t| match t { &Token::Whitespace => false, _ => true })
                    .collect::<Vec<_>>()
                ),
                Err(e) => return Err(e)
            }
        }

    }
}

#[derive(Debug)]
pub enum SQLExpr {
    Keyword(String),
    Identifier(String),
    LiteralInt(i32),
    Operator(String),
    SQLSelect { item: Vec<SQLExpr> },
    SQLInsert { colName: Vec<String>, colValue: Vec<SQLExpr> },
    SQLBinaryExpr { left: Box<SQLExpr>, op: String, right: Box<SQLExpr> }
}

fn parseExpressionList(tokens: &Vec<Token>, offset: usize) -> (usize, Vec<SQLExpr>) {
    println!("parseExpressionList() BEGIN token={:?}", tokens[offset]);
    let mut o: usize = offset;
    let mut ret: Vec<SQLExpr> = Vec::new();

    let mut foo = true;
    while o < tokens.len() && foo {
        println!("parseExpressionList() TOP_OF_LOOP token={:?}", tokens[o]);

        let (index, expr) = parse(tokens, o).unwrap();
        ret.push(expr);

        o = index;

        if o < tokens.len() {
            foo = match tokens[o] {
                Token::Comma => {
                    o = o + 1;
                    true
                },
                _ => false
            }
        } else {
            foo = false;
        }
    }
    println!("parseExpressionList() END returning {:?}", ret);
    (o, ret)
}

fn parseSelect(tokens: &Vec<Token>, offset: usize) -> (usize, SQLExpr) {
    println!("parseSelect()");
    let (a, b) = parseExpressionList(tokens, offset);
    (a, SQLExpr::SQLSelect { item: b })
}

fn parsePrefix(tokens: &Vec<Token>, offset: usize) -> Result<(usize, SQLExpr), String> {
    println!("parsePrefix() token={:?}", tokens[offset]);
    let token: &Token = &tokens[offset];
    match token {
        &Token::Keyword(ref s) => {
	    	match s.as_ref() {
	            "SELECT" => Ok(parseSelect(tokens, offset+1)),
	            _ => Err(String::from("TBD"))
	        }
        }
        &Token::LiteralInt(i) => Ok((offset+1, SQLExpr::LiteralInt(i))),
        _ => Err(String::from("TBD"))
    }
}

fn getPrecedence(tokens: &Vec<Token>, offset: usize) -> u32 {
    println!("getPrecedence() token={:?}", tokens[offset]);
    match &tokens[offset] {
        &Token::Operator(ref op) => match op.as_ref() {
            "=" => 5,
            "OR" => 7,
            "AND" => 9,
            "NOT" => 10,
            "<" | "<=" | ">" | ">=" | "<>" | "!=" => 20,
            "-" | "+" => 33,
            "*" | "/" => 40,
            _ => 0
        },
        _ => 0
    }
}

fn parseInfix(_left: SQLExpr, tokens: &Vec<Token>, offset: usize) -> Result<(usize, SQLExpr), String> {
    println!("parseInfix() token={:?}", tokens[offset]);
    match &tokens[offset] {
        &Token::Operator(ref _op) => {
            let (index, _right) = parse(tokens, offset + 1).unwrap();
            Ok((index, SQLExpr::SQLBinaryExpr {
                left: Box::new(_left),
                op: _op.clone(),
                right: Box::new(_right)
            }))
        },
        _ => Err(String::from("No infix parser found for token"))
    }
}

/** This is the main pratt parser logic */
pub fn parse(tokens: &Vec<Token>, offset: usize) -> Result<(usize, SQLExpr), String> {
    println!("parse() token={:?}", tokens[offset]);

    let (index, expr) = parsePrefix(tokens, offset).unwrap();

    let mut o = index;
    let mut e = expr;

    //TODO: not complete
    let precedence = 0;
    while o < tokens.len() && precedence < getPrecedence(tokens, index) {
        println!("Before parseInfix and expr = {:?}", e);
        let (index, foo) = parseInfix(e, tokens, index).unwrap();
        println!("After parseInfix and expr = {:?}", foo);

        o = index;
        e = foo;
        //e = Box::new(&expr);
    }

    println!("parse returning {:?}", e);
    Ok((o, e))
}


#[cfg(test)]
mod tests {
    use super::{Token, Tokenizer, parse};
    use Token::*;

    #[test]
    fn simple_tokenize() {
        assert_eq!(
            vec![Keyword(String::from("SELECT")), LiteralInt(1), Operator(String::from("+")), LiteralInt(1)],
            String::from("SELECT 1 + 1").tokenize().unwrap()
        );
    }

    #[test]
    fn simple_parse() {
        let t = vec!(
            Keyword(String::from("SELECT")),
            LiteralInt(1),
            Operator(String::from("+")),
            LiteralInt(2),
            Operator(String::from("+")),
            LiteralInt(3)
        );

        let (foo, ast) = parse(&t, 0).unwrap();
        // what is foo?
        // put assert here to compare ast with expected
    }

}
