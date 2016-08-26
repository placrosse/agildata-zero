use std::iter::Peekable;
use std::str::Chars;
use super::tokenizer::{IToken, Token};

pub mod ansisql;

pub trait Dialect<T: IToken> {
	fn get_token(&self, chars: &mut Peekable<Chars>) -> Result<Option<Token<T>>, String>;
}
