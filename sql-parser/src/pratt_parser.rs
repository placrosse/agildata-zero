use super::tokenizer::{Token, Tokens};
use std::iter::Peekable;

pub type ASTNode = Box<Node>;

pub trait Node {}

pub trait ParserProvider {
	fn parse(&self, sql: &str) -> ASTNode;
	fn parse_prefix(&self, tokens: &mut Peekable<Tokens>) -> Option<ASTNode>;
	fn parse_infix(&self, left: ASTNode, stream: &mut Peekable<Tokens>, precedence: u32) -> Option<ASTNode>;
	fn get_precedence(&self, stream: &mut Peekable<Tokens>) -> u32;
}

pub struct PrattParser {}

impl PrattParser {
	pub fn parse(provider: &ParserProvider, stream: &mut Peekable<Tokens>, precedence: u32) -> ASTNode {
		match provider.parse_prefix(stream) {
			Some(node) => {
				let mut ret: ASTNode = node;
				while precedence < provider.get_precedence(stream) {
					let p = provider.get_precedence(stream);
					match provider.parse_infix(ret,stream, p) {
						Some(n) => ret = n,
						None => break
					}
				}
				return ret
			}
			None => panic!("TBD")
		}
	}
}
