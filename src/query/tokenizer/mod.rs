use std::iter::Peekable;
use std::str::Chars;
use super::dialects::Dialect;

pub trait Tokenizer<D, T> where D: Dialect<T>, T: IToken {
	fn tokenize(&self, dialects: &Vec<D>) -> Result<Vec<Token<T>>, String>;
}

impl<D, T> Tokenizer<D, T> for String where D: Dialect<T>, T: IToken {
	fn tokenize(&self, dialects: &Vec<D>) -> Result<Vec<Token<T>>, String> {
		let mut chars = self.chars().peekable();
		let mut tokens: Vec<Token<T>> = Vec::new();
		while let Some(&ch) = chars.peek() {
			match get_dialect_token(&dialects, &mut chars)? {
				None => return Err(String::from(format!("No token dialect support for character {:?}", ch))),
				Some(token) => tokens.push(token)
			}
		}
		Ok(tokens)
	}
}

fn get_dialect_token<D, T>(dialects: &Vec<D>, chars: &mut Peekable<Chars>) -> Result<Option<Token<T>>, String> where D: Dialect<T>, T: IToken {
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

pub enum Token<T> where T: IToken {
	Whitespace,
	Keyword(String),
	Identifier(String),
	//Literal(LiteralToken),
	Operator(String),
	Punctuator(String),
	TokenExtension(T)
}

pub trait IToken{}
