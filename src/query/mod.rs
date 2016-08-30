use std::iter::Peekable;
use std::str::Chars;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU32, Ordering};

pub mod dialects;

#[cfg(test)]
mod tests;

// Dialect api
pub trait Dialect {
	fn get_token(&self, chars: &mut Peekable<Chars>) -> Result<Option<Token>, String>;

	fn parse_prefix<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<Option<ASTNode>, String>;

	fn get_precedence<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<u8, String>;

	fn parse_infix<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>, left: ASTNode, precedence: u8) -> Result<Option<ASTNode>, String>;

	// fn plan();
}

// Tokenizer apis
pub trait Tokenizer<D: Dialect> {
	fn tokenize<'a>(&self, dialects: &'a Vec<D>) -> Result<Tokens<'a, D>, String>;
}

impl<D: Dialect> Tokenizer<D> for String {
	fn tokenize<'a>(&self, dialects: &'a Vec<D>) -> Result<Tokens<'a, D>, String> {
		let mut chars = self.chars().peekable();
		let mut tokens: Vec<Token> = Vec::new();
		while let Some(&ch) = chars.peek() {
			match get_dialect_token(&dialects, &mut chars)? {
				None => return Err(String::from(format!("No token dialect support for character {:?}", ch))),
				Some(token) => tokens.push(token)
			}
		}

		let stream = tokens
			.into_iter()
			.filter(|t| match t { &Token::Whitespace => false, _ => true })
			.collect::<Vec<_>>();

		Ok(Tokens::new(dialects, stream))
	}
}

fn get_dialect_token<D: Dialect> (dialects: &Vec<D>, chars: &mut Peekable<Chars>) -> Result<Option<Token>, String> {
	for d in dialects.iter() {
		let token = d.get_token(chars)?;
		match token {
			Some(t) => {
				return Ok(Some(t));
			},
			None => {}
		}
	}
	Ok(None)
}

#[derive(Debug)]
pub struct Tokens<'a, D: 'a + Dialect> {
	pub dialects: &'a Vec<D>,
	pub tokens: Vec<Token>,
	pub index: AtomicU32
}

impl<'a, D: 'a + Dialect> Tokens<'a, D> {
	pub fn new(dialects: &'a Vec<D>, tokens: Vec<Token>) -> Self {
		Tokens {
			dialects: dialects,
			tokens: tokens,
			index: AtomicU32::new(0)
		}
	}

	pub fn peek(&'a self) -> Option<&'a Token> {
		let i = self.index.load(Ordering::SeqCst) as usize;
		if (i < (self.tokens.len() - 1)) {
			Some(&self.tokens[i as usize])
		} else {
			None
		}
	}

	pub fn next(&'a self) -> Option<&'a Token> {
		let i = self.index.load(Ordering::SeqCst) as usize;
		if (i < (self.tokens.len() - 1)) {
			self.index.fetch_add(1, Ordering::SeqCst);
			Some(&self.tokens[i as usize])
		} else {
			panic!("Index out of bounds")
		}
	}
}

#[derive(Debug,PartialEq,Clone)]
pub enum Token  {
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
// Parser APIs
pub trait Parser<D: Dialect> {
	fn parse(&self) -> Result<Option<ASTNode>, String>;
	fn parse_expr(&self, precedence: u8) -> Result<Option<ASTNode>, String>;
}


impl<'a, D: Dialect> Parser<D> for Tokens<'a, D> {
	fn parse(&self) -> Result<Option<ASTNode>, String> { self.parse_expr(0) }

	fn parse_expr(&self, precedence: u8) -> Result<Option<ASTNode>, String> {
		match get_dialect_ast(&self.dialects, self, precedence)? {
			Some(node) => Ok(Some(node)),
			None => Err(String::from("No dialect support for token prefix TBD")) // TODO
		}
	}

}

fn get_dialect_ast<'a, D: Dialect>(dialects: &Vec<D>, tokens: &Tokens<'a, D>, precedence: u8) ->
	Result<Option<ASTNode>, String> {

	let mut expr = get_dialect_prefix(dialects, tokens)?;
	if expr.is_some() {
		return get_dialect_infix(dialects, tokens, expr.unwrap(), precedence)
	} else {
		Ok(expr)
	}
}

fn get_dialect_prefix<'a, D: Dialect>
	(dialects: &Vec<D>, tokens: &Tokens<'a, D>) ->
	Result<Option<ASTNode>, String> {

	for d in dialects.iter() {
		let expr = d.parse_prefix(tokens)?;
		if expr.is_some() {
			return Ok(expr)
		}
	}

	Ok(None)
}

fn get_dialect_infix<'a, D: Dialect>(dialects: &Vec<D>, tokens: &Tokens<'a, D>, left: ASTNode, precedence: u8) ->
	Result<Option<ASTNode>, String> {

	for d in dialects.iter() {
		let next_precedence = d.get_precedence(tokens)?;

		if precedence >= next_precedence {
			continue;
		}
		match d.parse_infix(tokens, left, next_precedence)? {
			Some(e) => return Ok(Some(e)),
			None => return Err(String::from("Illegal state!"))
		}
	}

	Ok(Some(left))
}


pub enum ASTNode {
	AST
}


// Planner APIs
pub trait Planner<D: Dialect> {
	fn plan(&self, dialects: D, ast: ASTNode) -> Result<Option<RelNode>, String>;
}

pub enum RelNode {
	Rel
}
