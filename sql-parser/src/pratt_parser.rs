use super::tokenizer::Token;

trait PrefixParser {
	fn parse(stream: Vec<Token>) -> ASTNode;
}

trait InfixParser {
	fn parse(left: ASTNode, stream: Vec<Token>);
	fn get_precedence();
}

trait ASTNode {}

trait ParserProvider {
	fn parse(sql: str) -> Box<ASTNode>;
	fn parse_prefix(tokens: Vec<Token>) -> Box<PrefixParser>;
	fn parse_infix(left: ASTNode, stream: Vec<Token>, precedence: u32) -> Box<InfixParser>;
}

struct PrattParser {}

impl PrattParser {
	fn parse(provider: ParserProvider, stream: Vec<Token>, precedence: u32) -> ASTNode {
		match provider.parse_prefix(stream) {
			_ => panic!("Not implemented")
		}
	}
}
