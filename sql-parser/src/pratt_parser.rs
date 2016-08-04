use super::tokenizer::{Token, Tokens};
use std::iter::Peekable;
use std::fmt::Debug;

pub type ASTNode = Box<Node>;

pub trait Node: Debug {}

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
				let mut ret: ASTNode = get_infix(provider, stream, precedence, node);
				// while precedence < provider.get_precedence(stream) {
				// 	let p = provider.get_precedence(stream);
				// 	match provider.parse_infix(ret,stream, p) {
				// 		Some(n) => ret = n,
				// 		None => break
				// 	}
				// }
				return ret
			}
			None => panic!("TBD")
		}
	}
}

pub fn get_infix(provider: &ParserProvider, stream: &mut Peekable<Tokens>, precedence: u32, left: ASTNode) -> ASTNode {
	println!("get_infix()");
	if precedence >= provider.get_precedence(stream) {
		println!("return");
		left
	} else {
		println!("recurse");
		let p = provider.get_precedence(stream);
		let ret = {
			let r = provider.parse_infix(left, stream, p).unwrap();
			get_infix(provider, stream, precedence, r)
		};
		ret
	}
}
