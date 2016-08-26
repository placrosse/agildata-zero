use std::iter::Peekable;
use std::str::Chars;
use super::tokenizer::{IToken, Token};
use super::parser::IAST;
use super::planner::IRel;

pub mod ansisql;

pub trait Dialect<T: IToken, A: IAST, P: IRel> {
	fn get_token(&self, chars: &mut Peekable<Chars>) -> Result<Option<Token<T>>, String>;

	// fn get_token_precedence();
	//
	// fn parse();
	//
	// fn plan();
}
