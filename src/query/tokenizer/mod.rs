use std::iter::Peekable;
use std::str::Chars;
use super::dialects::Dialect;

pub trait Tokenizer<D: Dialect<T>, T: IToken> {
	fn tokenize(&self, dialects: &Vec<D>) -> Result<Vec<Token<T>>, String>;
}

impl<D: Dialect<T>, T: IToken> Tokenizer<D, T> for String {
	fn tokenize(&self, dialects: &Vec<D>) -> Result<Vec<Token<T>>, String> {
		let mut chars = self.chars().peekable();
		let mut tokens: Vec<Token<T>> = Vec::new();
		while let Some(&ch) = chars.peek() {
			match get_dialect_token(&dialects, &mut chars)? {
				None => return Err(String::from(format!("No token dialect support for character {:?}", ch))),
				Some(token) => tokens.push(token)
			}
		}

		return Ok(tokens
			.into_iter()
			.filter(|t| match t { &Token::Whitespace => false, _ => true })
			.collect::<Vec<_>>()
		)
	}
}

fn get_dialect_token<D: Dialect<T>, T: IToken> (dialects: &Vec<D>, chars: &mut Peekable<Chars>) -> Result<Option<Token<T>>, String> {
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
