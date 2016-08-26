use super::Dialect;
use super::super::tokenizer::*;
use std::iter::Peekable;
use std::str::Chars;

struct AnsiSQLParser {}

impl<T> Dialect<T> for AnsiSQLParser where T: IToken {
	fn get_token(&self, chars: &mut Peekable<Chars>) -> Result<Option<Token<T>>, String> {
		Err(String::from("Not implemented"))
	}
}
