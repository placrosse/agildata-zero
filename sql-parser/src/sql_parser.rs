use super::pratt_parser::*;
use super::tokenizer::*;
use std::iter::Peekable;
use std::str::FromStr;

struct AnsiSQLProvider {}

impl ParserProvider for AnsiSQLProvider {

	fn parse(&self, sql: &str) -> ASTNode {
		let tvec = String::from(sql).tokenize().unwrap();
		let mut stream = (Tokens {tokens: tvec, index: 0}).peekable();
		PrattParser::parse(self, &mut stream, 0u32)
	}

	fn parse_prefix(&self, tokens: &mut Peekable<Tokens>) -> Option<ASTNode>{
		println!("parse_prefix()");
		// TODO need a better solution than cloned()
		match tokens.peek().cloned() {
			Some(t) => match t {
				Token::Keyword(ref v) => match &v as &str {
					"SELECT" => Some(self.parse_select(tokens)),
					_ => panic!("Unsupported prefix {}", v)
				},
				Token::Literal(v) => match v {
					LiteralToken::LiteralLong(value) => {
						tokens.next();
						Some(Box::new(SQLAST::SQLLiteral(LiteralExpr::LiteralLong(u64::from_str(&value).unwrap()))))
					}
					_ => panic!("Literals")
				},
				Token::Identifier(v) => Some(Box::new(SQLAST::SQLIdentifier(v))),
				_ => panic!("parse_prefix() {:?}", t)
			},
			None => None
		}
	}

	fn parse_infix(&self, left: ASTNode, stream: &mut Peekable<Tokens>, precedence: u32) -> Option<ASTNode>{
		println!("parse_infix()");
		match stream.peek().cloned() {
			Some(token) => match token {
				Token::Operator(t) => Some(self.parse_binary(left, stream)),
				_ => {
					println!("Returning no infix for token {:?}", token);
					None
				}

			},
			None => None
		}
		//panic!("parse_infix() Not implemented")
	}

	fn get_precedence(&self, stream: &mut Peekable<Tokens>) -> u32{
		println!("get_precedence() token={:?}", stream.peek());
	    // match &tokens[offset] {
	    //     &Token::Operator(ref op) => match op.as_ref() {
	    //         "=" => 5,
	    //         "OR" => 7,
	    //         "AND" => 9,
	    //         "NOT" => 10,
	    //         "<" | "<=" | ">" | ">=" | "<>" | "!=" => 20,
	    //         "-" | "+" => 33,
	    //         "*" | "/" => 40,
	    //         _ => 0
	    //     },
	    //     _ => 0
	    // }
		match stream.peek().cloned() {
			Some(token) => match token {
				Token::Operator(t) => match &t as &str {
					"<" | "<=" | ">" | ">=" | "<>" | "!=" => 20,
					"-" | "+" => 33,
					"*" | "/" => 40,
					"=" => 5,
					_ => panic!("Unsupported operator {}", t)
				},
				_ => 0
			},
			None => 0
		}
	}

}

impl AnsiSQLProvider {
	fn parse_select(&self, tokens: &mut Peekable<Tokens>) -> ASTNode {
		println!("parse_select()");
		// consume the SELECT
		tokens.next();
		let proj = self.parse_expr_list(tokens);

		Box::new(SQLAST::SQLSelect{expr_list: proj})
	}

	fn parse_expr_list(&self, tokens: &mut Peekable<Tokens>) -> ASTNode {
		println!("parse_expr_list()");
		let first = self.parse_expr(tokens, 0_u32);
		let mut v: Vec<ASTNode> = Vec::new();
		v.push(first);
		while let Some(Token::Punctuator(p)) = tokens.peek().cloned() {
			if p == "," {
				println!("HERE");
				tokens.next();
				v.push(self.parse_expr(tokens, 0_u32));
			} else {
				break;
			}
		}
		Box::new(SQLAST::SQLExprList(v))
	}

	fn parse_expr(&self, tokens: &mut Peekable<Tokens>, precedence: u32) -> ASTNode {
		PrattParser::parse(self, tokens, precedence)
	}

	fn parse_binary(&self, left: ASTNode, tokens: &mut Peekable<Tokens>) -> ASTNode {
		println!("parse_binary()");
		// determine operator
		let operator = match tokens.next().unwrap() {
			Token::Operator(t) => match &t as &str {
				"+" => SQLOperator::ADD,
				_ => panic!("Unsupported operator {}", t)
			},
			_ => panic!("Expected operator, received something else")
		};

		// TODO real precedence
		Box::new(SQLAST::SQLBinary {left: left, op: operator, right: self.parse_expr(tokens, 0)})
	}
}


#[derive(Debug)]
enum SQLAST {
	SQLExprList(Vec<ASTNode>),
	SQLLiteralLing(u64),
	SQLBinary{left: ASTNode, op: SQLOperator, right: ASTNode},
	SQLLiteral(LiteralExpr),
	SQLIdentifier(String),

	SQLSelect{expr_list: ASTNode}

}
impl Node for SQLAST {}


#[derive(Debug)]
enum LiteralExpr {
	LiteralLong(u64)
}
impl Node for LiteralExpr {}

#[derive(Debug)]
enum SQLOperator {
	ADD
}

#[cfg(test)]
mod tests {
	use super::{AnsiSQLProvider, SQLAST, LiteralExpr};
	use pratt_parser::ParserProvider;

	#[test]
	fn sqlparser() {
		let parser = AnsiSQLProvider {};
		// assert_eq!(
		// 	SQLAST::SQLLiteral(LiteralExpr::LiteralLong(0_u64)),
		// 	parser.parse("SELECT 1 + 1, a")
		// );
		println!("{:?}", parser.parse("SELECT 1 + 1, a"));
	}
}
