use super::pratt_parser::*;
use super::tokenizer::*;
use std::iter::Peekable;

struct AnsiSQLProvider {}

impl ParserProvider for AnsiSQLProvider {

	fn parse(&self, sql: &str) -> ASTNode {
		let tvec = String::from(sql).tokenize().unwrap();
		let stream = (Tokens {tokens: tvec, index: 0}).peekable();
		PrattParser::parse(self, stream, 0u32)
	}

	fn parse_prefix(&self, tokens: &mut Peekable<Tokens>) -> Option<ASTNode>{
		// TODO need a better solution than cloned()
		match tokens.peek().cloned() {
			Some(t) => match t {
				Token::Keyword(ref v) => match &v as &str {
					"SELECT" => Some(self.parse_select(tokens)),
					_ => panic!("Unsupported prefix {}", v)
				},
				_ => panic!("Not implemented")
			},
			None => None
		}
	}

	fn parse_infix(&self, left: &ASTNode, stream: &mut Peekable<Tokens>, precedence: u32) -> Option<ASTNode>{
		panic!("Not implemented")
	}

	fn get_precedence(&self, stream: &mut Peekable<Tokens>) -> u32{
		panic!("Not implemented")
	}

}

impl AnsiSQLProvider {
	fn parse_select(&self, tokens: &mut Peekable<Tokens>) -> ASTNode {
		panic!("Not implemented")
	}
}



#[cfg(test)]
mod tests {
	use super::AnsiSQLProvider;
	use pratt_parser::ParserProvider;

	#[test]
	fn test() {
		let parser = AnsiSQLProvider {};
		parser.parse("SELECT 1 + 1");
	}
}
