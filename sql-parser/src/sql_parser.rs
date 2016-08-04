use super::pratt_parser::*;
use super::tokenizer::*;
use std::iter::Peekable;

struct AnsiSQLProvider {}

impl ParserProvider for AnsiSQLProvider {

	fn parse(&self, sql: &str) -> ASTNode {
		panic!("Not implemented")
	}

	fn parse_prefix(&self, tokens: &mut Peekable<Tokens>) -> Option<ASTNode>{
		panic!("Not implemented")
	}

	fn parse_infix(&self, left: &ASTNode, stream: &mut Peekable<Tokens>, precedence: u32) -> Option<ASTNode>{
		panic!("Not implemented")
	}

	fn get_precedence(&self, stream: &mut Peekable<Tokens>) -> u32{
		panic!("Not implemented")
	}

}
