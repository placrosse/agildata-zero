use std::iter::Peekable;
use std::str::Chars;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU32, Ordering};

pub mod dialects;

#[cfg(test)]
mod tests;


// sql.tokenize(&dialects).parse().plan()
// Dialect api
pub trait Dialect<T: IToken, A: IAST, R: IRel> {
	fn get_token(&self, chars: &mut Peekable<Chars>) -> Result<Option<Token<T>>, String>;

	fn parse_prefix<'a, D: Dialect<T, A, R>>(&self, tokens: &Tokens<'a, D, T, A, R>) -> Result<Option<ASTNode<A>>, String>;

	fn get_precedence<'a, D: Dialect<T, A, R>>(&self, tokens: &Tokens<'a, D, T, A, R>) -> Result<u8, String>;

	fn parse_infix<'a, D: Dialect<T, A, R>>(&self, tokens: &Tokens<'a, D, T, A, R>, left: ASTNode<A>, precedence: u8) -> Result<Option<ASTNode<A>>, String>;

	// fn plan();
}

// Tokenizer apis
pub trait Tokenizer<D: Dialect<T, A, R>, T: IToken, A: IAST, R: IRel> {
	fn tokenize<'a>(&self, dialects: &'a Vec<D>) -> Result<Tokens<'a, D, T, A, R>, String>;
}

impl<D: Dialect<T, A, R>, T: IToken, A: IAST, R: IRel> Tokenizer<D, T, A, R> for String {
	fn tokenize<'a>(&self, dialects: &'a Vec<D>) -> Result<Tokens<'a, D, T, A, R>, String> {
		let mut chars = self.chars().peekable();
		let mut tokens: Vec<Token<T>> = Vec::new();
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

fn get_dialect_token<D: Dialect<T, A, R>, T: IToken, A: IAST, R: IRel> (dialects: &Vec<D>, chars: &mut Peekable<Chars>) -> Result<Option<Token<T>>, String> {
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
pub struct Tokens<'a, D: 'a + Dialect<T, A, R>, T: IToken, A: IAST, R: IRel> {
	pub dialects: &'a Vec<D>,
	pub tokens: Vec<Token<T>>,
	pub index: AtomicU32,
	_p1: PhantomData<A>,
	_p2: PhantomData<R>
}

impl<'a, D: 'a + Dialect<T, A, R>, T: IToken, A: IAST, R: IRel> Tokens<'a, D, T, A, R> {
	pub fn new(dialects: &'a Vec<D>, tokens: Vec<Token<T>>) -> Self {
		Tokens {
			dialects: dialects,
			tokens: tokens,
			index: AtomicU32::new(0),
			_p1: PhantomData,
			_p2: PhantomData
		}
	}

	pub fn peek(&'a self) -> Option<&'a Token<T>> {
		let i = self.index.load(Ordering::SeqCst) as usize;
		if (i < (self.tokens.len() - 1)) {
			Some(&self.tokens[i as usize])
		} else {
			None
		}
	}

	pub fn next(&'a self) -> Option<&'a Token<T>> {
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
pub enum Token<T: IToken>  {
	Whitespace,
	Keyword(String),
	Identifier(String),
	//Literal(LiteralToken),
	Operator(String),
	Punctuator(String),
	TokenExtension(T)
}

pub trait IToken{}

impl<T> IToken for Token<T> where T: IToken {}

// Parser APIs
pub trait Parser<D: Dialect<T, A, R>, T: IToken, A: IAST, R: IRel> {
	fn parse(&self) -> Result<Option<ASTNode<A>>, String>;
	fn parse_expr(&self, precedence: u8) -> Result<Option<ASTNode<A>>, String>;
}


impl<'a, D: Dialect<T, A, R>, T: IToken, A: IAST, R: IRel> Parser<D, T, A, R> for Tokens<'a, D, T, A, R> {
	fn parse(&self) -> Result<Option<ASTNode<A>>, String> { self.parse_expr(0) }

	fn parse_expr(&self, precedence: u8) -> Result<Option<ASTNode<A>>, String> {
		match get_dialect_ast(&self.dialects, self, precedence)? {
			Some(node) => Ok(Some(node)),
			None => Err(String::from("No dialect support for token prefix TBD")) // TODO
		}
	}

}

fn get_dialect_ast<'a, D: Dialect<T, A, R>, T: IToken, A: IAST, R: IRel>
	(dialects: &Vec<D>, tokens: &Tokens<'a, D, T, A, R>, precedence: u8) ->
	Result<Option<ASTNode<A>>, String> {

	let mut expr = get_dialect_prefix(dialects, tokens)?;
	if expr.is_some() {
		return get_dialect_infix(dialects, tokens, expr.unwrap(), precedence)
	} else {
		Ok(expr)
	}
}

fn get_dialect_prefix<'a, D: Dialect<T, A, R>, T: IToken, A: IAST, R: IRel>
	(dialects: &Vec<D>, tokens: &Tokens<'a, D, T, A, R>) ->
	Result<Option<ASTNode<A>>, String> {

	for d in dialects.iter() {
		let expr = d.parse_prefix(tokens)?;
		if expr.is_some() {
			return Ok(expr)
		}
	}

	Ok(None)
}

fn get_dialect_infix<'a, D: Dialect<T, A, R>, T: IToken, A: IAST, R: IRel>
	(dialects: &Vec<D>, tokens: &Tokens<'a, D, T, A, R>, left: ASTNode<A>, precedence: u8) ->
	Result<Option<ASTNode<A>>, String> {

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


pub enum ASTNode<A: IAST> {
	AST(A)
}
pub trait IAST{}


// Planner APIs
pub trait Planner<D: Dialect<T, A, R>, T: IToken, A: IAST, R: IRel> {
	fn plan(&self, dialects: D, ast: A) -> Result<Option<RelNode<R>>, String>;
}

pub trait IRel {}
pub enum RelNode<R: IRel> {
	Rel(R)
}
