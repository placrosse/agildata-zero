use super::pratt_parser::*;
use super::tokenizer::*;

struct AnsiSQLProvider {}

impl ParserProvider for AnsiSQLProvider {

	fn parse(&self, sql: &str) -> ASTNode {
		panic!("Not implemented")
	}

	fn parse_prefix(&self, tokens: &mut Vec<Token>) -> Option<ASTNode>{
		panic!("Not implemented")
	}

	fn parse_infix(&self, left: &ASTNode, stream: &mut Vec<Token>, precedence: u32) -> Option<ASTNode>{
		panic!("Not implemented")
	}
	
	fn get_precedence(&self, stream: &mut Vec<Token>) -> u32{
		panic!("Not implemented")
	}

}
