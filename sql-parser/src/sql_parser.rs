use super::tokenizer::*;
use std::iter::Peekable;
use std::str::FromStr;
use std::ascii::AsciiExt;
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
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
	SQLUpdate {
		table: Box<SQLExpr>,
		assignments: Box<SQLExpr>,
		selection: Option<Box<SQLExpr>>
	},
	SQLUnion{left: Box<SQLExpr>, union_type: SQLUnionType, right: Box<SQLExpr>},
	SQLJoin{left: Box<SQLExpr>, join_type: SQLJoinType, right: Box<SQLExpr>, on_expr: Option<Box<SQLExpr>>},
	SQLCreateTable{table: Box<SQLExpr>, column_list: Vec<SQLExpr>},
	SQLColumnDef{column: Box<SQLExpr>, data_type: DataType, qualifiers: Option<Vec<ColumnQualifier>>}
}

#[derive(Debug, PartialEq)]
pub enum DataType {
	Bit{display: Option<u32>},
	TinyInt{display: Option<u32>},
	SmallInt{display: Option<u32>},
	MediumInt{display: Option<u32>},
	Int{display: Option<u32>},
	BigInt{display: Option<u32>},
	Decimal{precision: Option<u32>, scale: Option<u32>},
	Float{precision: Option<u32>, scale: Option<u32>},
	Double{precision: Option<u32>, scale: Option<u32>},
	Bool,
	Date,
	DateTime{fsp: Option<u32>},
	Timestamp{fsp: Option<u32>},
	Time{fsp: Option<u32>},
	Year{display: Option<u32>},
	Char{length: Option<u32>},
	Varchar{length: Option<u32>},
	Binary{length: Option<u32>},
	VarBinary{length: Option<u32>},
	TinyBlob,
	TinyText,
	Blob{length: Option<u32>},
	Text{length: Option<u32>},
	MediumBlob,
	MediumText,
	LongBlob,
	LongText,
	Enum{values: Box<SQLExpr>},
	Set{values: Box<SQLExpr>}
}

#[derive(Debug, PartialEq)]
pub enum ColumnQualifier {
	CharacterSet(Box<SQLExpr>),
	Collate(Box<SQLExpr>),
	Default(Box<SQLExpr>),
	Signed,
	Unsigned,
	Null,
	NotNull,
	AutoIncrement,
	PrimaryKey,
	UniqueKey,
	OnUpdate(Box<SQLExpr>),
	Comment(Box<SQLExpr>)
}

#[derive(Debug, PartialEq)]
pub enum LiteralExpr {
	LiteralLong(u32, u64),
	LiteralBool(u32, bool),
	LiteralDouble(u32, f64),
	LiteralString(u32, String)
}

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
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

impl AnsiSQLParser {

	pub fn parse(&self, sql: &str) -> Result<SQLExpr,  String> {
		let tvec = try!(String::from(sql).tokenize());
		let mut stream = (Tokens {tokens: tvec, index: 0}).peekable();
		self.parse_expr(&mut stream, 0u32)
	}

	pub fn parse_expr(&self, stream: &mut Peekable<Tokens>, precedence: u32) -> Result<SQLExpr,  String> {
		let mut expr = self.parse_prefix(stream).unwrap();

		if expr.is_some() {
			while let Some(next) = stream.peek().cloned() {
				let next_precedence = self.get_precedence(stream);

				if precedence >= next_precedence {
					break;
				}

				expr = self.parse_infix(expr.unwrap(), stream, next_precedence).unwrap();
			}
			Ok(expr.unwrap())
		} else {
			Err(String::from("Failed to parse expr TBD"))
		}
	}

	fn parse_prefix(&self, tokens: &mut Peekable<Tokens>) -> Result<Option<SQLExpr>,  String>{
		println!("parse_prefix()");
		// TODO need a better solution than cloned()
		match tokens.peek().cloned() {
			Some(t) => match t {
				Token::Keyword(ref v) => match &v as &str {
					"SELECT" => Ok(Some(try!(self.parse_select(tokens)))),
					"INSERT" => Ok(Some(try!(self.parse_insert(tokens)))),
					"UPDATE" => Ok(Some(try!(self.parse_update(tokens)))),
					"CREATE" => Ok(Some(try!(self.parse_create(tokens)))),
					_ => Err(format!("Unsupported prefix {:?}", v))
				},
				Token::Literal(v) => match v {
					LiteralToken::LiteralLong(i, value) => {
						tokens.next();
						Ok(Some(SQLExpr::SQLLiteral(LiteralExpr::LiteralLong(i, u64::from_str(&value).unwrap()))))
					},
					LiteralToken::LiteralBool(i, value) => {
						tokens.next();
						Ok(Some(SQLExpr::SQLLiteral(LiteralExpr::LiteralBool(i, bool::from_str(&value).unwrap()))))
					},
					LiteralToken::LiteralDouble(i, value) => {
						tokens.next();
						Ok(Some(SQLExpr::SQLLiteral(LiteralExpr::LiteralDouble(i, f64::from_str(&value).unwrap()))))
					},
					LiteralToken::LiteralString(i, value) => {
						tokens.next();
						Ok(Some(SQLExpr::SQLLiteral(LiteralExpr::LiteralString(i, value.clone()))))
					}
					//_ => panic!("Unsupported literal {:?}", v)
				},
				Token::Identifier(v) => Ok(Some(try!(self.parse_identifier(tokens)))),//Some(self.parse_identifier(tokens)),
				Token::Punctuator(v) => match &v as &str {
					"(" => {
						Ok(Some(try!(self.parse_nested(tokens))))
					},
					_ => Err(format!("Unsupported prefix for punctuator {:?}", &v))
				},
				Token::Operator(v) => match &v as &str {
					"+" | "-" => Ok(Some(try!(self.parse_unary(tokens)))),
					"*" => Ok(Some(try!(self.parse_identifier(tokens)))),
					_ => Err(format!("Unsupported operator as prefix {:?}", &v))
				},
				_ => Err(format!("parse_prefix() {:?}", &t))
			},
			None => Ok(None)
		}
	}

	fn parse_infix(&self, left: SQLExpr, stream: &mut Peekable<Tokens>, precedence: u32) -> Result<Option<SQLExpr>,  String>{
		println!("parse_infix() {}", precedence);
		match stream.peek().cloned() {
			Some(token) => match token {
				Token::Operator(t) => Ok(Some(try!(self.parse_binary(left, stream)))),//Some(self.parse_binary(left, stream)),
				Token::Keyword(t) => match &t as &str {
					"UNION" => Ok(Some(try!(self.parse_union(left, stream)))),
					"JOIN" | "INNER" | "RIGHT" | "LEFT" | "CROSS" | "FULL" => Ok(Some(try!(self.parse_join(left, stream)))),
					"AS" => Ok(Some(try!(self.parse_alias(left, stream)))),
					_ => {
						println!("Returning no infix for keyword {:?}", t);
						Ok(None)
					}
				},
				_ => {
					println!("Returning no infix for token {:?}", token);
					Ok(None)
				}

			},
			None => Ok(None)
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
					"UNION" => 3,
					"JOIN" | "INNER" | "RIGHT" | "LEFT" | "CROSS" | "FULL" => 5,
					"AS" => 6,
					_ => 0
				},
				_ => 0
			},
			None => 0
		}
	}

	fn parse_create(&self, tokens: &mut Peekable<Tokens>) -> Result<SQLExpr, String> {
		self.consume_keyword("CREATE", tokens);

		if self.consume_keyword("TABLE", tokens) {
			let table = try!(self.parse_identifier(tokens));
			self.consume_punctuator("(", tokens);

			let mut columns: Vec<SQLExpr> = Vec::new();
			columns.push(try!(self.parse_column_def(tokens)));
			while self.consume_punctuator(",", tokens) {
				columns.push(try!(self.parse_column_def(tokens)));
			}

			//let column_list = try!(self.parse_expr_list(tokens));
			self.consume_punctuator(")", tokens);

			Ok(SQLExpr::SQLCreateTable{table: Box::new(table), column_list: columns })
		} else {
			Err(String::from(format!("Unexpected token after CREATE {:?}", tokens.peek())))
		}

	}

	fn parse_column_def(&self, tokens: &mut Peekable<Tokens>) -> Result<SQLExpr, String> {
		let column = try!(self.parse_identifier(tokens));
		let data_type: DataType = try!(self.parse_data_type(tokens));

		match tokens.peek().cloned() {
			Some(Token::Punctuator(p)) => match &p as &str {
				"," | ")" => {},
				_ => return Err(String::from(format!("Unsupported token in column definition: {:?}", tokens.peek())))
			},
			_ => return Err(String::from(format!("Unsupported token in column definition: {:?}", tokens.peek())))
		}

		Ok(SQLExpr::SQLColumnDef{column: Box::new(column), data_type: data_type, qualifiers: None})
	}

	fn parse_data_type(&self, tokens: &mut Peekable<Tokens>) ->  Result<DataType, String> {
		let data_token = tokens.next();
		match data_token {
			Some(Token::Keyword(t)) | Some(Token::Identifier(t)) => match &t.to_uppercase() as &str {
				"BIT" => Ok(DataType::Bit{display: try!(self.parse_optional_display(tokens))}),
				"TINYINT" => Ok(DataType::TinyInt{display: try!(self.parse_optional_display(tokens))}),
				"SMALLINT" => Ok(DataType::SmallInt{display: try!(self.parse_optional_display(tokens))}),
				"MEDIUMINT" => Ok(DataType::MediumInt{display: try!(self.parse_optional_display(tokens))}),
				"INT" | "INTEGER" => Ok(DataType::Int{display: try!(self.parse_optional_display(tokens))}),
				"BIGINT" => Ok(DataType::BigInt{display: try!(self.parse_optional_display(tokens))}),
				"DECIMAL" | "DEC" => {
					match try!(self.parse_optional_precision_and_scale(tokens)) {
						Some((p, s)) => Ok(DataType::Decimal{precision: Some(p), scale: s}),
						None => Ok(DataType::Decimal{precision: None, scale: None})
					}
				},
				"FLOAT" => {
					match try!(self.parse_optional_precision_and_scale(tokens)) {
						Some((p, s)) => Ok(DataType::Float{precision: Some(p), scale: s}),
						None => Ok(DataType::Float{precision: None, scale: None})
					}
				},
				"DOUBLE" => {
					match try!(self.parse_optional_precision_and_scale(tokens)) {
						Some((p, s)) => Ok(DataType::Double{precision: Some(p), scale: s}),
						None => Ok(DataType::Double{precision: None, scale: None})
					}
				},
				"BOOL" | "BOOLEAN" => Ok(DataType::Bool),
				"DATE" => Ok(DataType::Date),
				"DATETIME" => Ok(DataType::DateTime{fsp: try!(self.parse_optional_display(tokens))}),
				"TIMESTAMP" => Ok(DataType::Timestamp{fsp: try!(self.parse_optional_display(tokens))}),
				"TIME" => Ok(DataType::Time{fsp: try!(self.parse_optional_display(tokens))}),
				"YEAR" => Ok(DataType::Year{display: try!(self.parse_optional_display(tokens))}),
				// TODO do something with NATIONAL, NCHAR, etc
				"NATIONAL" => self.parse_data_type(tokens),
				"CHAR" | "NCHAR" => {
					let ret = Ok(DataType::Char{length: try!(self.parse_optional_display(tokens))});
					// TODO do something with CHAR BYTE
					self.consume_keyword("BYTE", tokens);
					ret
				},
				"CHARACTER" => {
					if self.consume_keyword("VARYING", tokens) {
						Ok(DataType::Varchar{length: try!(self.parse_optional_display(tokens))})
					} else {
						Ok(DataType::Char{length: try!(self.parse_optional_display(tokens))})
					}
				},
				"VARCHAR" | "NVARCHAR" => Ok(DataType::Varchar{length: try!(self.parse_optional_display(tokens))}),
				"BINARY" => Ok(DataType::Binary{length: try!(self.parse_optional_display(tokens))}),
				"VARBINARY" => Ok(DataType::VarBinary{length: try!(self.parse_optional_display(tokens))}),
				"TINYBLOB" => Ok(DataType::TinyBlob),
				"TINYTEXT" => Ok(DataType::TinyText),
				"MEDIUMBLOB" => Ok(DataType::MediumBlob),
				"MEDIUMTEXT" => Ok(DataType::MediumText),
				"LONGBLOB" => Ok(DataType::LongBlob),
				"LONGTEXT" => Ok(DataType::LongText),
				"BLOB" => Ok(DataType::Blob{length: try!(self.parse_optional_display(tokens))}),
				"TEXT" => Ok(DataType::Text{length: try!(self.parse_optional_display(tokens))}),
				"ENUM" => {
					self.consume_punctuator("(", tokens);
					let values = try!(self.parse_expr_list(tokens));
					self.consume_punctuator(")", tokens);
					Ok(DataType::Enum{values: Box::new(values)})
				},
				"SET" => {
					self.consume_punctuator("(", tokens);
					let values = try!(self.parse_expr_list(tokens));
					self.consume_punctuator(")", tokens);
					Ok(DataType::Set{values: Box::new(values)})
				},
				_ => Err(format!("Data type not recognized {}", t))
			},
			_ => Err(format!("Expected data type, received token {:?}", tokens.peek()))
		}
	}

	fn parse_optional_display(&self, tokens: &mut Peekable<Tokens>) -> Result<Option<u32>, String> {
		if self.consume_punctuator("(", tokens) {
			match tokens.peek().cloned() {
				Some(Token::Literal(LiteralToken::LiteralLong(i, v))) => {
					tokens.next();
					let ret = Ok(Some(u32::from_str(&v).unwrap()));
					self.consume_punctuator(")", tokens);
					ret
				},
				_ => Err(String::from(format!("Expected LiteralLong token, received {:?}", tokens.peek())))
			}
		} else {
			Ok(None)
		}

	}

	fn parse_optional_precision_and_scale(&self, tokens: &mut Peekable<Tokens>) -> Result<Option<(u32,Option<u32>)>, String> {
		self.consume_keyword("PRECISION", tokens);

		if self.consume_punctuator("(", tokens) {
			let p = try!(self.parse_long(tokens));
			let s = if self.consume_punctuator(",", tokens) {
				Some(try!(self.parse_long(tokens)))
			} else {
				None
			};
			self.consume_punctuator(")", tokens);
			Ok(Some((p, s)))
		} else {
			Ok(None)
		}

	}

	fn parse_long(&self, tokens: &mut Peekable<Tokens>) -> Result<u32, String> {
		match tokens.peek().cloned() {
			Some(Token::Literal(LiteralToken::LiteralLong(i, v))) => {
				tokens.next();
				Ok(u32::from_str(&v).unwrap())
			},
			_ => Err(String::from(format!("Expected LiteralLong token, received {:?}", tokens.peek())))
		}
	}

	fn parse_insert(&self, tokens: &mut Peekable<Tokens>) -> Result<SQLExpr,  String> {
		println!("parse_insert()");

		// TODO validation
		self.consume_keyword("INSERT", tokens);
		self.consume_keyword("INTO", tokens);

		let table = try!(self.parse_identifier(tokens));

		let columns = if self.consume_punctuator("(", tokens) {
			let ret = try!(self.parse_expr_list(tokens));
			self.consume_punctuator(")", tokens);
			ret
		} else {
			return Err(format!("Expected column list paren, received {:?}", &tokens.peek()));
		};

		self.consume_keyword("VALUES", tokens);
		self.consume_punctuator("(", tokens);
		let values = try!(self.parse_expr_list(tokens));
		self.consume_keyword(")", tokens);

		Ok(SQLExpr::SQLInsert {
			table: Box::new(table),
			column_list: Box::new(columns),
			values_list: Box::new(values)
		})

	}

	fn parse_select(&self, tokens: &mut Peekable<Tokens>) -> Result<SQLExpr,  String> {
		println!("parse_select()");
		// consume the SELECT
		tokens.next();
		let proj = Box::new(try!(self.parse_expr_list(tokens)));

		let from = match tokens.peek().cloned() {
			Some(Token::Keyword(t)) => match &t as &str {
				"FROM" => {
					tokens.next();
					Some(Box::new(try!(self.parse_relation(tokens))))
				},
				_ => None
			},
			_ => return Err(format!("unexpected token {:?}", tokens.peek()))
		};

		let whr = match tokens.peek().cloned() {
			Some(Token::Keyword(t)) => match &t as &str {
				"WHERE" => {
					tokens.next();
					Some(Box::new(try!(self.parse_expr(tokens, 0))))
				},
				_ => None
			},
			_ => None
		};

		let ob = {
			if self.consume_keyword(&"ORDER", tokens) {
				if self.consume_keyword(&"BY", tokens) {
					Some(Box::new(try!(self.parse_order_by_list(tokens))))
				} else {
					return Err(format!("Expected ORDER BY, found ORDER {:?}", tokens.peek()));
				}
			} else {
				None
			}
		};

		Ok(SQLExpr::SQLSelect{expr_list: proj, relation: from, selection: whr, order: ob})
	}

	fn parse_update(&self, tokens: &mut Peekable<Tokens>) -> Result<SQLExpr, String> {
		self.consume_keyword("UPDATE", tokens);

		let table = try!(self.parse_identifier(tokens));

		self.consume_keyword("SET", tokens);

		let assignments = try!(self.parse_expr_list(tokens));

		let selection = if self.consume_keyword("WHERE", tokens) {
			Some(Box::new(try!(self.parse_expr(tokens, 0))))
		} else {
			None
		};

		Ok(SQLExpr::SQLUpdate {
			table: Box::new(table),
			assignments: Box::new(assignments),
			selection: selection
		})
	}

	// TODO real parse_relation
	fn parse_relation(&self, tokens: &mut Peekable<Tokens>) -> Result<SQLExpr,  String> {
		self.parse_expr(tokens, 4)
	}

	fn parse_expr_list(&self, tokens: &mut Peekable<Tokens>) -> Result<SQLExpr,  String> {
		println!("parse_expr_list()");
		let first = try!(self.parse_expr(tokens, 0_u32));
		let mut v: Vec<SQLExpr> = Vec::new();
		v.push(first);
		while let Some(Token::Punctuator(p)) = tokens.peek().cloned() {
			if p == "," {
				tokens.next();
				v.push(try!(self.parse_expr(tokens, 0_u32)));
			} else {
				break;
			}
		}
		Ok(SQLExpr::SQLExprList(v))
	}

	fn parse_order_by_list(&self, tokens: &mut Peekable<Tokens>) -> Result<SQLExpr,  String> {
		println!("parse_order_by_list()");
		let mut v: Vec<SQLExpr> = Vec::new();
		v.push(try!(self.parse_order_by_expr(tokens)));
		while let Some(Token::Punctuator(p)) = tokens.peek().cloned() {
			if p == "," {
				tokens.next();
				v.push(try!(self.parse_order_by_expr(tokens)));
			} else {
				break;
			}
		}
		Ok(SQLExpr::SQLExprList(v))
	}

	fn parse_order_by_expr(&self, tokens: &mut Peekable<Tokens>) -> Result<SQLExpr,  String> {
		let e = try!(self.parse_expr(tokens, 0_u32));
		Ok(SQLExpr::SQLOrderBy {expr: Box::new(e), is_asc: self.is_asc(tokens)})
	}

	fn is_asc(&self, tokens: &mut Peekable<Tokens>) -> bool {
		if self.consume_keyword(&"DESC", tokens) {
			false
		} else {
			self.consume_keyword(&"ASC", tokens);
			true
		}
	}

	fn parse_binary(&self, left: SQLExpr, tokens: &mut Peekable<Tokens>) -> Result<SQLExpr,  String> {
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
				_ => return Err(format!("Unsupported operator {}", t))
			},
			_ => return Err(format!("Expected operator, received something else"))
		};

		Ok(SQLExpr::SQLBinary {left: Box::new(left), op: operator, right: Box::new(try!(self.parse_expr(tokens, precedence)))})
	}

	fn parse_identifier(&self, tokens: &mut Peekable<Tokens>) -> Result<SQLExpr,  String> {
		println!("parse_identifier()");
		match tokens.next().unwrap() {
			Token::Identifier(v) => Ok(SQLExpr::SQLIdentifier(v)),
			Token::Operator(o) => match &o as &str {
				"*" => Ok(SQLExpr::SQLIdentifier(o)),
				_ => Err(format!("Unsupported operator as identifier {}", o))
			},
			_ => Err(format!("Illegal state"))
		}
	}

	fn parse_nested(&self, tokens: &mut Peekable<Tokens>) -> Result<SQLExpr,  String> {
		//consume (
		tokens.next();
		let nested = try!(self.parse_expr(tokens, 0));
		// consume )
		match tokens.peek().cloned() {
			Some(Token::Punctuator(v)) => match &v as &str {
				")" => {tokens.next();},
				_ => return Err(format!("Expected , punctuator, received {}", v))
			},
			_ => return Err(format!("Illegal state, expected , received {:?}", tokens.peek()))
		}

		Ok(SQLExpr::SQLNested(Box::new(nested)))
	}

	fn parse_unary(&self, tokens: & mut Peekable<Tokens>) -> Result<SQLExpr,  String> {
		let precedence = self.get_precedence(tokens);
		let op = match tokens.next() {
			Some(Token::Operator(o)) => match &o as &str {
				"+" => SQLOperator::ADD,
				"-" => SQLOperator::SUB,
				_ => return Err(format!("Illegal operator for unary {}", o))
			},
			_ => return Err(format!("Illegal state"))
		};
		Ok(SQLExpr::SQLUnary{operator: op, expr: Box::new(try!(self.parse_expr(tokens, precedence)))})

	}

	fn parse_union(&self, left: SQLExpr, tokens: &mut Peekable<Tokens>) -> Result<SQLExpr,  String> {
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

		let right = Box::new(try!(self.parse_expr(tokens, 0)));

		Ok(SQLExpr::SQLUnion{left: Box::new(left), union_type: union_type, right: right})

	}

	fn parse_join(&self, left: SQLExpr, tokens: &mut Peekable<Tokens>) -> Result<SQLExpr,  String> {
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
				return Err(format!("Unsupported join keyword {:?}", tokens.peek()))
			}
		};

		let right = Box::new(try!(self.parse_expr(tokens, 0)));

		let on = {
			if self.consume_keyword("ON", tokens) {
				Some(Box::new(try!(self.parse_expr(tokens, 0))))
			} else if join_type != SQLJoinType::CROSS {
				return Err(format!("Expected ON, received token {:?}", tokens.peek()))
			} else {
				None
			}
		};

		Ok(SQLExpr::SQLJoin {left: Box::new(left), join_type: join_type, right: right, on_expr: on})
	}

	fn parse_alias(&self, left: SQLExpr, tokens: &mut Peekable<Tokens>) -> Result<SQLExpr,  String> {
		if self.consume_keyword(&"AS", tokens) {
			Ok(SQLExpr::SQLAlias{expr: Box::new(left), alias: Box::new(try!(self.parse_identifier(tokens)))})
		} else {
			Err(format!("Illegal state, expected AS, received token {:?}", tokens.peek()))
		}
	}

	// TODO more helper methods like consume_keyword_sequence, required_keyword_sequence, etc
	fn consume_keyword(&self, text: &str, tokens: &mut Peekable<Tokens>) -> bool {
		match tokens.peek().cloned() {
			Some(Token::Keyword(v)) | Some(Token::Identifier(v)) => {
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
