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
				Token::Identifier(v) => Some(self.parse_identifier(tokens)),
				Token::Punctuator(v) => match &v as &str {
					"(" => {
						Some(self.parse_nested(tokens))
					},
					_ => panic!("Unsupported prefix for punctuator {:?}", v)
				},
				Token::Operator(v) => match &v as &str {
					"+" | "-" => Some(self.parse_unary(tokens)),
					_ => panic!("Unsupported operator as prefix {:?}", v)
				},
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

		let from = match tokens.peek().cloned() {
			Some(Token::Keyword(t)) => match &t as &str {
				"FROM" => {
					tokens.next();
					Some(self.parse_expr(tokens, 0))
				},
				_ => None
			},
			_ => panic!("unexpected token {:?}", tokens.peek())
		};

		// let select = Box::new(SQLAST::SQLSelect{expr_list: proj});
		// match tokens.peek().cloned() {
		// 	Some(Token::Keyword(t)) => match &t as &str {
		// 		"UNION" => return Box::new(self.parse_infix(select, tokens, 99).unwrap()),
		// 		_ => {}
		// 	}
		// }
		Box::new(SQLAST::SQLSelect{expr_list: proj, relation: from})
	}

	fn parse_expr_list(&self, tokens: &mut Peekable<Tokens>) -> ASTNode {
		println!("parse_expr_list()");
		let first = self.parse_expr(tokens, 0_u32);
		let mut v: Vec<ASTNode> = Vec::new();
		v.push(first);
		while let Some(Token::Punctuator(p)) = tokens.peek().cloned() {
			if p == "," {
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
				"-" => SQLOperator::SUB,
				"*" => SQLOperator::MULT,
				"/" => SQLOperator::DIV,
				"%" => SQLOperator::MOD,
				_ => panic!("Unsupported operator {}", t)
			},
			_ => panic!("Expected operator, received something else")
		};

		// TODO real precedence
		Box::new(SQLAST::SQLBinary {left: left, op: operator, right: self.parse_expr(tokens, 0)})
	}

	fn parse_identifier(&self, tokens: &mut Peekable<Tokens>) -> ASTNode {
		println!("parse_identifier()");
		let ident = match tokens.next().unwrap() {
			Token::Identifier(v) => Box::new(SQLAST::SQLIdentifier(v)),
			_ => panic!("Illegal state")
		};

		match tokens.peek().cloned() {
			Some(Token::Keyword(k)) => match &k as &str {
				"AS" => {
					tokens.next();
					return Box::new(SQLAST::SQLAlias{expr: ident, alias: self.parse_identifier(tokens)})
				},
				_ => {}
			},
			_ => {}
		}
		ident
	}

	fn parse_nested(&self, tokens: &mut Peekable<Tokens>) -> ASTNode {
		//consume (
		tokens.next();
		let nested = self.parse_expr(tokens, 0);
		// consume )
		tokens.next(); // TODO not really correct, wish there was a consume expected

		Box::new(SQLAST::SQLNested(nested))
	}

	fn parse_unary(&self, tokens: & mut Peekable<Tokens>) -> ASTNode {
		let op = match tokens.next() {
			Some(Token::Operator(o)) => match &o as &str {
				"+" => SQLOperator::ADD,
				"-" => SQLOperator::SUB,
				_ => panic!("Illegal operator for unary {}", o)
			},
			_ => panic!("Illegal state")
		};
		Box::new(SQLAST::SQLUnary{operator: op, expr: self.parse_expr(tokens, 0)})

	}
}


#[derive(Debug)]
enum SQLAST {
	SQLExprList(Vec<ASTNode>),
	SQLLiteralLing(u64),
	SQLBinary{left: ASTNode, op: SQLOperator, right: ASTNode},
	SQLLiteral(LiteralExpr),
	SQLIdentifier(String),
	SQLAlias{expr: ASTNode, alias: ASTNode},
	SQLNested(ASTNode),
	SQLUnary{operator: SQLOperator, expr: ASTNode},
	SQLSelect{expr_list: ASTNode, relation: Option<ASTNode>},

}
impl Node for SQLAST {}


#[derive(Debug)]
enum LiteralExpr {
	LiteralLong(u64)
}
impl Node for LiteralExpr {}

#[derive(Debug)]
enum SQLOperator {
	ADD,
	SUB,
	MULT,
	DIV,
	MOD
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
		println!("{:?}", parser.parse("SELECT 1 + 1, a AS alias, (3 * (1 + 2)), -1  FROM t1"));
	}
}
