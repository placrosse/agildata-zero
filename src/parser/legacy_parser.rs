// Remove these when we are done
#[allow(dead_code)]      // silence unused code in this file
#[allow(non_snake_case)] // silence naming convention warnings
// End remove these

use std::iter::Peekable;
use std::str::Chars;

use std::convert::AsRef;

#[derive(Debug,PartialEq)]
pub enum Token {
    Whitespace,
    Keyword(String),
    // Identifier(String),
    LiteralString(String),
    LiteralInt(i32),
    Operator(String),
    Comma,
}

#[derive(Debug)]
pub enum TokenizerError {
    InvalidToken(String),
    // NoMoreTokens
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
        // let mut error: Option<TokenizerError> = None;

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
    // Keyword(String),
    // Identifier(String),
    LiteralInt(i32),
    // Operator(String),
    SQLSelect { item: Vec<SQLExpr> },
    // SQLInsert { colName: Vec<String>, colValue: Vec<SQLExpr> },
    SQLBinaryExpr { left: Box<SQLExpr>, op: String, right: Box<SQLExpr> }
}

fn parse_expression_list(tokens: &Vec<Token>, offset: usize) -> (usize, Vec<SQLExpr>) {
    println!("parse_expression_list() BEGIN token={:?}", tokens[offset]);
    let mut o: usize = offset;
    let mut ret: Vec<SQLExpr> = Vec::new();

    let mut foo = true;
    while o < tokens.len() && foo {
        println!("parse_expression_list() TOP_OF_LOOP token={:?}", tokens[o]);

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
    println!("parse_expression_list() END returning {:?}", ret);
    (o, ret)
}

fn parse_select(tokens: &Vec<Token>, offset: usize) -> (usize, SQLExpr) {
    println!("parse_select()");
    let (a, b) = parse_expression_list(tokens, offset);
    (a, SQLExpr::SQLSelect { item: b })
}

fn parse_prefix(tokens: &Vec<Token>, offset: usize) -> Result<(usize, SQLExpr), String> {
    println!("parse_prefix() token={:?}", tokens[offset]);
    let token: &Token = &tokens[offset];
    match token {
        &Token::Keyword(ref s) => {
	    	match s.as_ref() {
	            "SELECT" => Ok(parse_select(tokens, offset+1)),
	            _ => Err(String::from("TBD"))
	        }
        }
        &Token::LiteralInt(i) => Ok((offset+1, SQLExpr::LiteralInt(i))),
        _ => Err(String::from("TBD"))
    }
}

fn get_precedence(tokens: &Vec<Token>, offset: usize) -> u32 {
    println!("get_precedence() token={:?}", tokens[offset]);
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

fn parse_infix(_left: SQLExpr, tokens: &Vec<Token>, offset: usize) -> Result<(usize, SQLExpr), String> {
    println!("parse_infix() token={:?}", tokens[offset]);
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

    let (index, expr) = parse_prefix(tokens, offset).unwrap();

    let mut o = index;
    let mut e = expr;

    //TODO: not complete
    let precedence = 0;
    while o < tokens.len() && precedence < get_precedence(tokens, index) {
        println!("Before parse_infix and expr = {:?}", e);
        let (index, foo) = parse_infix(e, tokens, index).unwrap();
        println!("After parse_infix and expr = {:?}", foo);

        o = index;
        e = foo;
        //e = Box::new(&expr);
    }

    println!("parse returning {:?}", e);
    Ok((o, e))
}


#[cfg(test)]
mod tests {
    use super::{Tokenizer, parse};
    use super::Token::*;

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

        let (_foo, _ast) = parse(&t, 0).unwrap();
        // what is foo?
        // it's the predecessor to bar
        // put assert here to compare ast with expected
    }

}
