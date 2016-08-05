use super::pratt_parser::*;
use super::tokenizer::*;
use std::iter::Peekable;
use std::str::FromStr;
use std::ascii::AsciiExt;

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
					},
					LiteralToken::LiteralBool(value) => {
						tokens.next();
						Some(Box::new(SQLAST::SQLLiteral(LiteralExpr::LiteralBool(bool::from_str(&value).unwrap()))))
					},
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
				Token::Keyword(t) => match &t as &str {
					"UNION" => Some(self.parse_union(left, stream)),
					_ => {
						println!("Returning no infix for keyword {:?}", t);
						None
					}
				},
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
					"=" => 11,
					"AND" => 9,
					"OR" => 7,

					_ => panic!("Unsupported operator {}", t)
				},
				Token::Keyword(t) => match &t as &str {
					"UNION" => 60,
					_ => 0
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
					Some(self.parse_identifier(tokens)) // TODO real parse_relation
				},
				_ => None
			},
			_ => panic!("unexpected token {:?}", tokens.peek())
		};

		let whr = match tokens.peek().cloned() {
			Some(Token::Keyword(t)) => match &t as &str {
				"WHERE" => {
					tokens.next();
					Some(self.parse_expr(tokens, 0))
				},
				_ => None
			},
			_ => None
		};

		let ob: Option<ASTNode> = {
			if self.consume_keyword(&"ORDER", tokens) {
				if self.consume_keyword(&"BY", tokens) {
					Some(self.parse_order_by_list(tokens))
				} else {
					panic!("Expected ORDER BY, found ORDER {:?}", tokens.peek());
				}
			} else {
				None
			}
		};

		Box::new(SQLAST::SQLSelect{expr_list: proj, relation: from, selection: whr, order: ob})
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

	fn parse_order_by_list(&self, tokens: &mut Peekable<Tokens>) -> ASTNode {
		println!("parse_order_by_list()");
		let first = self.parse_order_by_expr(tokens);
		let mut v: Vec<ASTNode> = Vec::new();
		v.push(first);
		while let Some(Token::Punctuator(p)) = tokens.peek().cloned() {
			if p == "," {
				tokens.next();
				v.push(self.parse_order_by_expr(tokens));
			} else {
				break;
			}
		}
		Box::new(SQLAST::SQLExprList(v))
	}

	fn parse_order_by_expr(&self, tokens: &mut Peekable<Tokens>) -> ASTNode {
		let e = self.parse_expr(tokens, 0_u32);
		Box::new(SQLAST::SQLOrderBy {expr: e, is_asc: self.is_asc(tokens)})
	}

	fn is_asc(&self, tokens: &mut Peekable<Tokens>) -> bool {
		if self.consume_keyword(&"DESC", tokens) {
			false
		} else {
			self.consume_keyword(&"ASC", tokens);
			true
		}
	}

	fn parse_expr(&self, tokens: &mut Peekable<Tokens>, precedence: u32) -> ASTNode {
		PrattParser::parse(self, tokens, precedence)
	}

	fn parse_binary(&self, left: ASTNode, tokens: &mut Peekable<Tokens>) -> ASTNode {
		println!("parse_binary()");
		let precedence = self.get_precedence(tokens);
		// determine operator
		let operator = match tokens.next().unwrap() {
			Token::Operator(t) => match &t as &str {
				"+" => SQLOperator::ADD,
				"-" => SQLOperator::SUB,
				"*" => SQLOperator::MULT,
				"/" => SQLOperator::DIV,
				"%" => SQLOperator::MOD,
				">" => SQLOperator::GT,
				"<" => SQLOperator::LT,
				"=" => SQLOperator::EQ,
				"AND" => SQLOperator::AND,
				"OR" => SQLOperator::OR,
				_ => panic!("Unsupported operator {}", t)
			},
			_ => panic!("Expected operator, received something else")
		};

		Box::new(SQLAST::SQLBinary {left: left, op: operator, right: self.parse_expr(tokens, precedence)})
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
		match tokens.peek().cloned() {
			Some(Token::Punctuator(v)) => match &v as &str {
				")" => {tokens.next();},
				_ => panic!("Expected , punctuator, received {}", v)
			},
			_ => panic!("Illegal state, expected , received {:?}", tokens.peek())
		}
		//tokens.next(); // TODO not really correct, wish there was a consume expected

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

	fn parse_union(&self, left: ASTNode, tokens: &mut Peekable<Tokens>) -> ASTNode {
		// consume the UNION
		tokens.next();

		let union_type = match tokens.peek().cloned() {
			Some(Token::Keyword(t)) => match &t as &str {
				"ALL" => SQLUnionType::ALL,
				"DISTINCT" => SQLUnionType::DISTINCT,
				_ => SQLUnionType::UNION
			},
			_ => SQLUnionType::UNION
		};

		let right = self.parse_expr(tokens, 0);

		Box::new(SQLAST::SQLUnion{left: left, union_type: union_type, right: right})

	}

	fn consume_keyword(&self, text: &str, tokens: &mut Peekable<Tokens>) -> bool {
		match tokens.peek().cloned() {
			Some(Token::Keyword(v)) => {
				if text.eq_ignore_ascii_case(&v) {
					tokens.next();
					true
				} else {
					false
				}
			},
			_ => false
		}
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
	SQLOrderBy{expr: ASTNode, is_asc: bool},
	SQLSelect{
		expr_list: ASTNode,
		relation: Option<ASTNode>,
		selection: Option<ASTNode>,
		order: Option<ASTNode>
	},
	SQLUnion{left: ASTNode, union_type: SQLUnionType, right: ASTNode}

}
impl Node for SQLAST {}


#[derive(Debug)]
enum LiteralExpr {
	LiteralLong(u64),
	LiteralBool(bool)
}
impl Node for LiteralExpr {}

#[derive(Debug)]
enum SQLOperator {
	ADD,
	SUB,
	MULT,
	DIV,
	MOD,
	GT,
	LT,
	GTEQ,
	LTEQ,
	EQ,
	NEQ,
	OR,
	AND
}

#[derive(Debug)]
enum SQLUnionType {
	UNION,
	ALL,
	DISTINCT
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
		println!("{:?}", parser.parse("SELECT 1 + 1 + 1, a AS alias, (3 * (1 + 2)), -1  FROM tOne WHERE a > 10 AND b = true ORDER BY a DESC, (a + b) ASC, c"));
	}

	#[test]
	fn nasty() {
		let parser = AnsiSQLProvider {};
		println!("{:?}", parser.parse("((((SELECT a, b, c FROM tOne UNION (SELECT a, b, c FROM tTwo))))) UNION (((SELECT a, b, c FROM tThree) UNION ((SELECT a, b, c FROM tFour))))"))
	}
}
