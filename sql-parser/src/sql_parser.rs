use super::tokenizer::*;
use std::iter::Peekable;
use std::str::FromStr;
use std::ascii::AsciiExt;

#[derive(Debug)]
pub enum SQLExpr {
	SQLExprList(Vec<SQLExpr>),
	SQLBinary{left: Box<SQLExpr>, op: SQLOperator, right: Box<SQLExpr>},
	SQLLiteral(LiteralExpr),
	SQLIdentifier(String),
	SQLAlias{expr: Box<SQLExpr>, alias: Box<SQLExpr>},
	SQLNested(Box<SQLExpr>),
	SQLUnary{operator: SQLOperator, expr: Box<SQLExpr>},
	SQLOrderBy{expr: Box<SQLExpr>, is_asc: bool},
	SQLSelect{
		expr_list: Box<SQLExpr>,
		relation: Option<Box<SQLExpr>>,
		selection: Option<Box<SQLExpr>>,
		order: Option<Box<SQLExpr>>
	},
	SQLInsert {
		table: Box<SQLExpr>,
		column_list: Box<SQLExpr>,
		values_list: Box<SQLExpr>
	},
	SQLUnion{left: Box<SQLExpr>, union_type: SQLUnionType, right: Box<SQLExpr>},
	SQLJoin{left: Box<SQLExpr>, join_type: SQLJoinType, right: Box<SQLExpr>, on_expr: Option<Box<SQLExpr>>}
}


#[derive(Debug)]
pub enum LiteralExpr {
	LiteralLong(u32, u64),
	LiteralBool(u32, bool),
	LiteralDouble(u32, f64),
	LiteralString(u32, String)
}

#[derive(Debug)]
pub enum SQLOperator {
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
pub enum SQLUnionType {
	UNION,
	ALL,
	DISTINCT
}

#[derive(Debug, PartialEq)]
pub enum SQLJoinType {
	INNER,
	LEFT,
	RIGHT,
	FULL,
	CROSS
}

pub struct AnsiSQLParser{}

// TODO should switch to Result returns instead of use of panic! ?
impl AnsiSQLParser {

	pub fn parse(&self, sql: &str) -> SQLExpr {
		let tvec = String::from(sql).tokenize().unwrap();
		let mut stream = (Tokens {tokens: tvec, index: 0}).peekable();
		self.parse_expr(&mut stream, 0u32)
	}

	pub fn parse_expr(&self, stream: &mut Peekable<Tokens>, precedence: u32) -> SQLExpr {
		match self.parse_prefix(stream) {
			Some(node) => self.get_infix(stream, precedence, node),
			None => panic!("TBD")
		}
	}

	pub fn get_infix(&self, stream: &mut Peekable<Tokens>, precedence: u32, left: SQLExpr) -> SQLExpr {
		println!("get_infix()");
		if precedence >= self.get_precedence(stream) {
			println!("return");
			left
		} else {
			println!("recurse");
			let p = self.get_precedence(stream);
			let ret = {
				let r = self.parse_infix(left, stream, p).unwrap();
				self.get_infix(stream, precedence, r)
			};
			ret
		}
	}

	fn parse_prefix(&self, tokens: &mut Peekable<Tokens>) -> Option<SQLExpr>{
		println!("parse_prefix()");
		// TODO need a better solution than cloned()
		match tokens.peek().cloned() {
			Some(t) => match t {
				Token::Keyword(ref v) => match &v as &str {
					"SELECT" => Some(self.parse_select(tokens)),
					"INSERT" => Some(self.parse_insert(tokens)),
					_ => panic!("Unsupported prefix {}", v)
				},
				Token::Literal(v) => match v {
					LiteralToken::LiteralLong(i, value) => {
						tokens.next();
						Some(SQLExpr::SQLLiteral(LiteralExpr::LiteralLong(i, u64::from_str(&value).unwrap())))
					},
					LiteralToken::LiteralBool(i, value) => {
						tokens.next();
						Some(SQLExpr::SQLLiteral(LiteralExpr::LiteralBool(i, bool::from_str(&value).unwrap())))
					},
					LiteralToken::LiteralDouble(i, value) => {
						tokens.next();
						Some(SQLExpr::SQLLiteral(LiteralExpr::LiteralDouble(i, f64::from_str(&value).unwrap())))
					},
					LiteralToken::LiteralString(i, value) => {
						tokens.next();
						Some(SQLExpr::SQLLiteral(LiteralExpr::LiteralString(i, value.clone())))
					}
					//_ => panic!("Unsupported literal {:?}", v)
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

	fn parse_infix(&self, left: SQLExpr, stream: &mut Peekable<Tokens>, precedence: u32) -> Option<SQLExpr>{
		println!("parse_infix()");
		match stream.peek().cloned() {
			Some(token) => match token {
				Token::Operator(t) => Some(self.parse_binary(left, stream)),
				Token::Keyword(t) => match &t as &str {
					"UNION" => Some(self.parse_union(left, stream)),
					"JOIN" | "INNER" | "RIGHT" | "LEFT" | "CROSS" | "FULL" => Some(self.parse_join(left, stream)),
					"AS" => Some(self.parse_alias(left, stream)),
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
	}

	fn get_precedence(&self, stream: &mut Peekable<Tokens>) -> u32{
		println!("get_precedence() token={:?}", stream.peek());
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
					"JOIN" | "INNER" | "RIGHT" | "LEFT" | "CROSS" | "FULL" => 50,
					"AS" => 4,
					_ => 0
				},
				_ => 0
			},
			None => 0
		}
	}

	fn parse_insert(&self, tokens: &mut Peekable<Tokens>) -> SQLExpr {
		println!("parse_insert()");

		// TODO validation
		self.consume_keyword("INSERT", tokens);
		self.consume_keyword("INTO", tokens);

		let table = self.parse_identifier(tokens);

		let columns = if self.consume_punctuator("(", tokens) {
			let ret = self.parse_expr_list(tokens);
			self.consume_punctuator(")", tokens);
			ret
		} else {
			panic!("Expected column list paren, received {:?}", tokens.peek());
		};

		self.consume_keyword("VALUES", tokens);
		self.consume_punctuator("(", tokens);
		let values = self.parse_expr_list(tokens);
		self.consume_keyword(")", tokens);

		SQLExpr::SQLInsert {
			table: Box::new(table),
			column_list: Box::new(columns),
			values_list: Box::new(values)
		}

	}

	fn parse_select(&self, tokens: &mut Peekable<Tokens>) -> SQLExpr {
		println!("parse_select()");
		// consume the SELECT
		tokens.next();
		let proj = Box::new(self.parse_expr_list(tokens));

		let from = match tokens.peek().cloned() {
			Some(Token::Keyword(t)) => match &t as &str {
				"FROM" => {
					tokens.next();
					Some(Box::new(self.parse_relation(tokens)))
				},
				_ => None
			},
			_ => panic!("unexpected token {:?}", tokens.peek())
		};

		let whr = match tokens.peek().cloned() {
			Some(Token::Keyword(t)) => match &t as &str {
				"WHERE" => {
					tokens.next();
					Some(Box::new(self.parse_expr(tokens, 0)))
				},
				_ => None
			},
			_ => None
		};

		let ob = {
			if self.consume_keyword(&"ORDER", tokens) {
				if self.consume_keyword(&"BY", tokens) {
					Some(Box::new(self.parse_order_by_list(tokens)))
				} else {
					panic!("Expected ORDER BY, found ORDER {:?}", tokens.peek());
				}
			} else {
				None
			}
		};

		SQLExpr::SQLSelect{expr_list: proj, relation: from, selection: whr, order: ob}
	}

	// TODO real parse_relation
	fn parse_relation(&self, tokens: &mut Peekable<Tokens>) -> SQLExpr {
		self.parse_expr(tokens, 0)
		//self.parse_identifier(tokens)
	}

	fn parse_expr_list(&self, tokens: &mut Peekable<Tokens>) -> SQLExpr {
		println!("parse_expr_list()");
		let first = self.parse_expr(tokens, 0_u32);
		let mut v: Vec<SQLExpr> = Vec::new();
		v.push(first);
		while let Some(Token::Punctuator(p)) = tokens.peek().cloned() {
			if p == "," {
				tokens.next();
				v.push(self.parse_expr(tokens, 0_u32));
			} else {
				break;
			}
		}
		SQLExpr::SQLExprList(v)
	}

	fn parse_order_by_list(&self, tokens: &mut Peekable<Tokens>) -> SQLExpr {
		println!("parse_order_by_list()");
		let mut v: Vec<SQLExpr> = Vec::new();
		v.push(self.parse_order_by_expr(tokens));
		while let Some(Token::Punctuator(p)) = tokens.peek().cloned() {
			if p == "," {
				tokens.next();
				v.push(self.parse_order_by_expr(tokens));
			} else {
				break;
			}
		}
		SQLExpr::SQLExprList(v)
	}

	fn parse_order_by_expr(&self, tokens: &mut Peekable<Tokens>) -> SQLExpr {
		let e = self.parse_expr(tokens, 0_u32);
		SQLExpr::SQLOrderBy {expr: Box::new(e), is_asc: self.is_asc(tokens)}
	}

	fn is_asc(&self, tokens: &mut Peekable<Tokens>) -> bool {
		if self.consume_keyword(&"DESC", tokens) {
			false
		} else {
			self.consume_keyword(&"ASC", tokens);
			true
		}
	}

	fn parse_binary(&self, left: SQLExpr, tokens: &mut Peekable<Tokens>) -> SQLExpr {
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

		SQLExpr::SQLBinary {left: Box::new(left), op: operator, right: Box::new(self.parse_expr(tokens, precedence))}
	}

	fn parse_identifier(&self, tokens: &mut Peekable<Tokens>) -> SQLExpr {
		println!("parse_identifier()");
		match tokens.next().unwrap() {
			Token::Identifier(v) => SQLExpr::SQLIdentifier(v),
			_ => panic!("Illegal state")
		}
	}

	fn parse_nested(&self, tokens: &mut Peekable<Tokens>) -> SQLExpr {
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

		SQLExpr::SQLNested(Box::new(nested))
	}

	fn parse_unary(&self, tokens: & mut Peekable<Tokens>) -> SQLExpr {
		let precedence = self.get_precedence(tokens);
		let op = match tokens.next() {
			Some(Token::Operator(o)) => match &o as &str {
				"+" => SQLOperator::ADD,
				"-" => SQLOperator::SUB,
				_ => panic!("Illegal operator for unary {}", o)
			},
			_ => panic!("Illegal state")
		};
		SQLExpr::SQLUnary{operator: op, expr: Box::new(self.parse_expr(tokens, precedence))}

	}

	fn parse_union(&self, left: SQLExpr, tokens: &mut Peekable<Tokens>) -> SQLExpr {
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

		let right = Box::new(self.parse_expr(tokens, 0));

		SQLExpr::SQLUnion{left: Box::new(left), union_type: union_type, right: right}

	}

	fn parse_join(&self, left: SQLExpr, tokens: &mut Peekable<Tokens>) -> SQLExpr {
		// TODO better protection on expected keyword sequences
		let join_type = {
			if self.consume_keyword("JOIN", tokens) || self.consume_keyword("INNER", tokens) {
				self.consume_keyword("JOIN", tokens);
				SQLJoinType::INNER
			} else if self.consume_keyword("LEFT", tokens) {
				self.consume_keyword("OUTER", tokens);
				self.consume_keyword("JOIN", tokens);
				SQLJoinType::LEFT
			} else if self.consume_keyword("RIGHT", tokens) {
				self.consume_keyword("OUTER", tokens);
				self.consume_keyword("JOIN", tokens);
				SQLJoinType::RIGHT
			} else if self.consume_keyword("FULL", tokens) {
				self.consume_keyword("OUTER", tokens);
				self.consume_keyword("JOIN", tokens);
				SQLJoinType::FULL
			} else if self.consume_keyword("CROSS", tokens) {
				self.consume_keyword("JOIN", tokens);
				SQLJoinType::LEFT
			} else {
				panic!("Unsupported join keyword {:?}", tokens.peek())
			}
		};

		let right = Box::new(self.parse_expr(tokens, 0));

		let on = {
			if self.consume_keyword("ON", tokens) {
				Some(Box::new(self.parse_expr(tokens, 0)))
			} else if join_type != SQLJoinType::CROSS {
				panic!("Expected ON, received token {:?}", tokens.peek())
			} else {
				None
			}
		};

		SQLExpr::SQLJoin {left: Box::new(left), join_type: join_type, right: right, on_expr: on}
	}

	fn parse_alias(&self, left: SQLExpr, tokens: &mut Peekable<Tokens>) -> SQLExpr {
		if self.consume_keyword(&"AS", tokens) {
			SQLExpr::SQLAlias{expr: Box::new(left), alias: Box::new(self.parse_identifier(tokens))}
		} else {
			panic!("Illegal state, expected AS, received token {:?}", tokens.peek())
		}
	}

	// TODO more helper methods like consume_keyword_sequence, required_keyword_sequence, etc
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

	fn consume_punctuator(&self, text: &str, tokens: &mut Peekable<Tokens>) -> bool {
		match tokens.peek().cloned() {
			Some(Token::Punctuator(v)) => {
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


#[cfg(test)]
mod tests {
	use super::{AnsiSQLParser, SQLExpr, LiteralExpr};
	use super::super::sql_writer;

	#[test]
	fn sqlparser() {
		let parser = AnsiSQLParser {};
		// assert_eq!(
		// 	SQLExpr::SQLLiteral(LiteralExpr::LiteralLong(0_u64)),
		// 	parser.parse("SELECT 1 + 1, a")
		// );
		let sql = "SELECT 1 + 1 + 1,
			a AS alias,
			(3 * (1 + 2)),
			-1 AS unary,
			(SELECT a, b, c FROM tTwo WHERE c = a) AS subselect
			FROM (SELECT a, b, c FROM tThree) AS l
			WHERE a > 10 AND b = true
			ORDER BY a DESC, (a + b) ASC, c";
		let parsed = parser.parse(sql);

		println!("{:#?}", parser.parse(sql));

		let rewritten = sql_writer::write(parsed);

		println!("Rewritten: {:?}", rewritten);

	}

	#[test]
	fn sql_join() {
		let parser = AnsiSQLParser {};
		// assert_eq!(
		// 	SQLExpr::SQLLiteral(LiteralExpr::LiteralLong(0_u64)),
		// 	parser.parse("SELECT 1 + 1, a")
		// );
		let sql = "SELECT l.a, r.b, l.c FROM tOne AS l
			JOIN (SELECT a, b, c FROM tTwo WHERE a > 0) AS r
			ON l.a = r.a
			WHERE l.b > r.b
			ORDER BY r.c DESC";
		let parsed = parser.parse(sql);

		println!("{:#?}", parser.parse(sql));

		let rewritten = sql_writer::write(parsed);

		println!("Rewritten: {:?}", rewritten);
	}

	#[test]
	fn nasty() {
		let parser = AnsiSQLParser {};
		let sql = "((((SELECT a, b, c FROM tOne UNION (SELECT a, b, c FROM tTwo))))) UNION (((SELECT a, b, c FROM tThree) UNION ((SELECT a, b, c FROM tFour))))";

		let parsed = parser.parse(sql);

		println!("{:#?}", parser.parse(sql));

		let rewritten = sql_writer::write(parsed);

		println!("Rewritten: {:?}", rewritten);
	}

	#[test]
	fn insert() {
		let parser = AnsiSQLParser {};
		let sql = "INSERT INTO foo (a, b, c) VALUES(1, 20.45, 'abcdefghijk')";

		let parsed = parser.parse(sql);

		println!("{:#?}", parser.parse(sql));

		let rewritten = sql_writer::write(parsed);

		println!("Rewritten: {:?}", rewritten);

	}
}
