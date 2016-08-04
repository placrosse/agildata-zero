use super::tokenizer::Token;

type ASTNode = Box<Node>;
trait Node {}

pub trait ParserProvider {
	fn parse(&self, sql: str) -> ASTNode;
	fn parse_prefix(&self, tokens: &mut Vec<Token>) -> Option<ASTNode>;
	fn parse_infix(&self, left: &ASTNode, stream: &mut Vec<Token>, precedence: u32) -> Option<ASTNode>;
	fn get_precedence(&self, stream: &mut Vec<Token>) -> u32;
}

struct PrattParser {}

impl PrattParser {
	fn parse(provider: &ParserProvider, mut stream: Vec<Token>, precedence: u32) -> ASTNode {
		match provider.parse_prefix(&mut stream) {
			Some(node) => {
				let mut ret: ASTNode = node;
				while precedence < provider.get_precedence(&mut stream) {
					let p = provider.get_precedence(&mut stream);
					match provider.parse_infix(&ret, &mut stream, p) {
						Some(n) => ret = n,
						None => break
					}
				}
				return ret
			}
			None => panic!("TBD")
		}
		panic!("Not implemented")
	}
}
