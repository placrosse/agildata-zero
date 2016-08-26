use std::iter::Peekable;
use std::str::Chars;
use super::tokenizer::{IToken, Token};

pub trait Dialect<T> where T: IToken {
	fn get_token(&self, chars: &mut Peekable<Chars>) -> Result<Option<Token<T>>, String>;
}
