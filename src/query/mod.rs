use std::iter::Peekable;
use std::str::Chars;
use std::marker::PhantomData;

pub mod dialects;

#[cfg(test)]
mod tests;

// Dialect api
pub trait Dialect<T: IToken, A: IAST, P: IRel> {
	fn get_token(&self, chars: &mut Peekable<Chars>) -> Result<Option<Token<T>>, String>;

	//fn parse_prefix<It: Iterator<Item=Token<T>>>(&self, parser: &PrattParser<T, A, R>, tokens: It) -> Result<Option<ASTNode<A>>, String>;
	// fn get_token_precedence();
	//
	// fn parse();
	//
	// fn plan();
}

// Tokenizer apis
pub trait Tokenizer<D: Dialect<T, A, R>, T: IToken, A: IAST, R: IRel> {
	fn tokenize<'a>(&self, dialects: &'a Vec<D>) -> Result<Tokens<'a, D, T, A, R>, String>;
}

#[derive(Debug,PartialEq,Clone)]
pub struct Tokens<'a, D: 'a + Dialect<T, A, R>, T: IToken, A: IAST, R: IRel> {
	pub dialects: &'a Vec<D>,
	pub tokens: Vec<Token<T>>,
	_p1: PhantomData<A>,
	_p2: PhantomData<R>
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

		Ok(Tokens{
			dialects: dialects,
			tokens: stream,
			_p1: PhantomData,
			_p2: PhantomData
		})
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
	fn parse(&self, dialects: Vec<D>) -> Result<Option<ASTNode<A>>, String>;
}

impl<D: Dialect<T, A, R>, T: IToken, A: IAST, R: IRel> Parser<D, T, A, R> for Vec<Token<T>> {
	fn parse(&self, dialects: Vec<D>) -> Result<Option<ASTNode<A>>, String> {
		self.iter().peekable().parse(dialects)
	}
}

// sql.tokenize(&dialects).iter().peekable().parse()

impl<'a, D: Dialect<T, A, R>, T: 'a + IToken, A: IAST, R: IRel, It: Iterator<Item=&'a Token<T>>> Parser<D, T, A, R> for Peekable<It> {
	fn parse(&self, dialects: Vec<D>) -> Result<Option<ASTNode<A>>, String> {
		panic!("HERE")
	}
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
