use super::tokenizer::*;
use std::iter::Peekable;
use std::str::FromStr;
use std::ascii::AsciiExt;

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
	SQLCreateTable{
		table: Box<SQLExpr>,
		column_list: Vec<SQLExpr>,
		keys: Vec<SQLExpr>,
		table_options: Vec<SQLExpr>
	},
	SQLColumnDef{column: Box<SQLExpr>, data_type: Box<SQLExpr>, qualifiers: Option<Vec<SQLExpr>>},
	SQLKeyDef(KeyDef),
	SQLColumnQualifier(ColumnQualifier),
	SQLDataType(DataType),
	SQLTableOption(TableOption),
	SQLUse(String)
}

#[derive(Debug, PartialEq)]
pub enum KeyDef {
	Primary{symbol: Option<Box<SQLExpr>>, name: Option<Box<SQLExpr>>, columns: Vec<SQLExpr>},
	Unique{symbol: Option<Box<SQLExpr>>, name: Option<Box<SQLExpr>>, columns: Vec<SQLExpr>},
	Foreign{symbol: Option<Box<SQLExpr>>, name: Option<Box<SQLExpr>>, columns: Vec<SQLExpr>, reference_table: Box<SQLExpr>, reference_columns: Vec<SQLExpr>},
	FullText{name: Option<Box<SQLExpr>>, columns: Vec<SQLExpr>},
	Index{name: Option<Box<SQLExpr>>, columns: Vec<SQLExpr>}
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
	NChar{length: Option<u32>},
	CharByte{length: Option<u32>},
	Varchar{length: Option<u32>},
	NVarchar{length: Option<u32>},
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
pub enum TableOption {
	Engine(Box<SQLExpr>),
	Charset(Box<SQLExpr>),
	Comment(Box<SQLExpr>),
	AutoIncrement(Box<SQLExpr>)
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
	// GTEQ,
	// LTEQ,
	EQ,
	// NEQ,
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
		let mut stream = tvec.iter().peekable();
		self.parse_expr(&mut stream, 0u32)
	}

	pub fn parse_expr<'a, It>(&self, stream: &mut Peekable<It>, precedence: u32) -> Result<SQLExpr,  String>
		where It: Iterator<Item=&'a Token> {

		let mut expr = self.parse_prefix(stream)?;

		if expr.is_some() {
			while let Some(_) = stream.peek() {
				let next_precedence = self.get_precedence(stream);

				if precedence >= next_precedence {
					break;
				}

				expr = self.parse_infix(expr.unwrap(), stream, next_precedence)?;
			}
			Ok(expr.unwrap())
		} else {
			Err(String::from("Failed to parse expr TBD"))
		}
	}

	fn parse_prefix<'a, It>(&self, tokens: &mut Peekable<It>) -> Result<Option<SQLExpr>,  String>
		where It: Iterator<Item=&'a Token> {

		println!("parse_prefix()");
		match tokens.peek() {
			Some(&t) => match t {
				&Token::Keyword(ref v) => match &v as &str {
					"SELECT" => Ok(Some(try!(self.parse_select(tokens)))),
					"INSERT" => Ok(Some(try!(self.parse_insert(tokens)))),
					"UPDATE" => Ok(Some(try!(self.parse_update(tokens)))),
					"CREATE" => Ok(Some(try!(self.parse_create(tokens)))),
					"USE" => Ok(Some(try!(self.parse_use(tokens)))),
					_ => Err(format!("Unsupported prefix {:?}", v))
				},
				&Token::Literal(ref v) => match v {
					&LiteralToken::LiteralLong(i, ref value) => {
						tokens.next();
						Ok(Some(SQLExpr::SQLLiteral(LiteralExpr::LiteralLong(i, u64::from_str(&value).unwrap()))))
					},
					&LiteralToken::LiteralBool(i, ref value) => {
						tokens.next();
						Ok(Some(SQLExpr::SQLLiteral(LiteralExpr::LiteralBool(i, bool::from_str(&value).unwrap()))))
					},
					&LiteralToken::LiteralDouble(i, ref value) => {
						tokens.next();
						Ok(Some(SQLExpr::SQLLiteral(LiteralExpr::LiteralDouble(i, f64::from_str(&value).unwrap()))))
					},
					&LiteralToken::LiteralString(i, ref value) => {
						tokens.next();
						Ok(Some(SQLExpr::SQLLiteral(LiteralExpr::LiteralString(i, value.clone()))))
					}
					//_ => panic!("Unsupported literal {:?}", v)
				},
				&Token::Identifier(_) => Ok(Some(try!(self.parse_identifier(tokens)))),//Some(self.parse_identifier(tokens)),
				&Token::Punctuator(ref v) => match &v as &str {
					"(" => {
						Ok(Some(try!(self.parse_nested(tokens))))
					},
					_ => Err(format!("Unsupported prefix for punctuator {:?}", &v))
				},
				&Token::Operator(ref v) => match &v as &str {
					"+" | "-" => Ok(Some(try!(self.parse_unary(tokens)))),
					"*" => Ok(Some(try!(self.parse_identifier(tokens)))),
					_ => Err(format!("Unsupported operator as prefix {:?}", &v))
				},
				_ => Err(format!("parse_prefix() {:?}", &t))
			},
			None => Ok(None)
		}
	}

	fn parse_infix<'a, It>(&self, left: SQLExpr, stream: &mut Peekable<It>, precedence: u32) -> Result<Option<SQLExpr>,  String>
		where It: Iterator<Item=&'a Token> {

			println!("parse_infix() {}", precedence);
		match stream.peek() {
			Some(&token) => match token {
				&Token::Operator(_) => Ok(Some(try!(self.parse_binary(left, stream)))),//Some(self.parse_binary(left, stream)),
				&Token::Keyword(ref t) => match &t as &str {
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

	fn get_precedence<'a, It>(&self, stream: &mut Peekable<It>) -> u32
		where It: Iterator<Item=&'a Token> {

		println!("get_precedence() token={:?}", stream.peek());
		match stream.peek() {
			Some(&token) => match token {
				&Token::Operator(ref t) => match &t as &str {
					"<" | "<=" | ">" | ">=" | "<>" | "!=" => 20,
					"-" | "+" => 33,
					"*" | "/" => 40,
					"=" => 11,
					"AND" => 9,
					"OR" => 7,

					_ => panic!("Unsupported operator {}", t)
				},
				&Token::Keyword(ref t) => match &t as &str {
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

	fn parse_create<'a, It>(&self, tokens: &mut Peekable<It>) -> Result<SQLExpr, String>
		where It: Iterator<Item=&'a Token> {

		self.consume_keyword("CREATE", tokens);

		if self.consume_keyword("TABLE", tokens) {
			let table = try!(self.parse_identifier(tokens));
			self.consume_punctuator("(", tokens);

			let mut columns: Vec<SQLExpr> = Vec::new();
			let mut keys: Vec<SQLExpr> = Vec::new();

			columns.push(try!(self.parse_column_def(tokens)));
			while self.consume_punctuator(",", tokens) {
				match tokens.peek() {
					Some(&&Token::Keyword(ref v)) => match &v as &str {
						"PRIMARY" | "KEY" | "UNIQUE" | "FULLTEXT" | "FOREIGN" | "CONSTRAINT" => keys.push(try!(self.parse_key_def(tokens))),
						_ => columns.push(try!(self.parse_column_def(tokens)))
					},
					_ => columns.push(try!(self.parse_column_def(tokens)))
				}
			}

			if !self.consume_punctuator(")", tokens) {
				return Err(String::from(format!("Expected token ) received token {:?}", tokens.peek())))
			}

			let table_options = self.parse_table_options(tokens)?;

			match tokens.peek() {
				None => Ok(SQLExpr::SQLCreateTable{
					table: Box::new(table),
					column_list: columns,
					keys: keys,
					table_options: table_options
				 }),
				_ => Err(String::from(format!("Expected end of statement, received {:?}", tokens.peek())))
			}

		} else {
			Err(String::from(format!("Unexpected token after CREATE {:?}", tokens.peek())))
		}

	}

	fn parse_use<'a, It>(&self, tokens: &mut Peekable<It>) -> Result<SQLExpr, String>
		where It: Iterator<Item=&'a Token> {

		assert!(self.consume_keyword("USE", tokens));

		match tokens.next() {
			Some(&Token::Identifier(ref id)) => Ok(SQLExpr::SQLUse(id.clone())),
			_ => Err(String::from("Expected identifier after USE"))
		}

	}

	fn parse_table_options<'a, It>(&self, tokens: &mut Peekable<It>) -> Result<Vec<SQLExpr>, String>
		where It: Iterator<Item=&'a Token> {

		let mut ret: Vec<SQLExpr> = Vec::new();

		while let Some(o) = self.parse_table_option(tokens)? {
			ret.push(o);
		}
		Ok(ret)
	}

	fn parse_table_option<'a, It>(&self, tokens: &mut Peekable<It>) -> Result<Option<SQLExpr>, String>
		where It: Iterator<Item=&'a Token> {

		match tokens.peek() {
			Some(&&Token::Keyword(ref v)) | Some(&&Token::Identifier(ref v)) => match &v.to_uppercase() as &str {
				"ENGINE" => {
					tokens.next();
					self.consume_operator("=", tokens);
					Ok(Some(SQLExpr::SQLTableOption(TableOption::Engine(Box::new(self.parse_expr(tokens, 0)?)))))
				},
				"DEFAULT" => { // [DEFAULT] [CHARACTER SET | COLLATE]
					tokens.next();
					self.parse_table_option(tokens)
				},
				"CHARACTER" | "CHARSET" => {
					tokens.next();
					self.consume_keyword("SET", tokens);
					Ok(Some(SQLExpr::SQLTableOption(TableOption::Charset(Box::new(self.parse_expr(tokens, 0)?)))))
				},
				"COMMENT" => {
					tokens.next();
					Ok(Some(SQLExpr::SQLTableOption(TableOption::Comment(Box::new(self.parse_expr(tokens, 0)?)))))
				},
				"AUTO_INCREMENT" => {
					tokens.next();
					Ok(Some(SQLExpr::SQLTableOption(TableOption::AutoIncrement(Box::new(self.parse_expr(tokens, 0)?)))))
				},
				// "COLLATE"
				_ => Err(String::from(format!("Unsupported Table Option {}", v)))
			},
			_ => Ok(None)
		}
	}

	fn parse_key_def<'a, It>(&self, tokens: &mut Peekable<It>) -> Result<SQLExpr, String>
		where It: Iterator<Item=&'a Token> {

		println!("parse_key_def()");

		let symbol = if self.consume_keyword("CONSTRAINT", tokens) {
			Some(Box::new(self.parse_identifier(tokens)?))
		} else {
			None
		};

		let t = tokens.next();

		match t {
			Some(&Token::Keyword(ref v)) => match &v as &str {
				"PRIMARY" => {
					self.consume_keyword("KEY", tokens);
					Ok(SQLExpr::SQLKeyDef(KeyDef::Primary{
						symbol: symbol,
						name: self.parse_optional_key_name(tokens)?,
						columns: self.parse_key_column_list(tokens)?
					}))
				},
				"UNIQUE" => {
					self.consume_keyword("KEY", tokens);
					Ok(SQLExpr::SQLKeyDef(KeyDef::Unique{
						symbol: symbol,
						name: self.parse_optional_key_name(tokens)?,
						columns: self.parse_key_column_list(tokens)?
					}))
				},
				"FOREIGN" => {
					self.consume_keyword("KEY", tokens);
					let name = self.parse_optional_key_name(tokens)?;
					let columns = self.parse_key_column_list(tokens)?;
					self.consume_keyword("REFERENCES", tokens);

					Ok(SQLExpr::SQLKeyDef(KeyDef::Foreign{
						symbol: symbol,
						name: name,
						columns: columns,
						reference_table: Box::new(self.parse_identifier(tokens)?),
						reference_columns: self.parse_key_column_list(tokens)?
					}))
				},
				"FULLTEXT" => {
					self.consume_keyword("KEY", tokens);
					Ok(SQLExpr::SQLKeyDef(KeyDef::FullText{
						name: self.parse_optional_key_name(tokens)?,
						columns: self.parse_key_column_list(tokens)?
					}))
				},
				"KEY" => {
					self.consume_keyword("KEY", tokens);
					Ok(SQLExpr::SQLKeyDef(KeyDef::Index{
						name: self.parse_optional_key_name(tokens)?,
						columns: self.parse_key_column_list(tokens)?
					}))
				},
				_ => Err(String::from(format!("Unsupported key definition prefix {}", v)))
			},
			_ => Err(String::from(format!("Expected key definition received token {:?}", t)))
		}
	}

	fn parse_optional_key_name<'a, It>(&self, tokens: &mut Peekable<It>) -> Result<Option<Box<SQLExpr>>, String>
		where It: Iterator<Item=&'a Token> {

		match tokens.peek() {
			Some(&&Token::Identifier(_)) => Ok(Some(Box::new(self.parse_identifier(tokens)?))),
			_ => Ok(None)
		}
	}

	fn parse_key_column_list<'a, It>(&self, tokens: &mut Peekable<It>) -> Result<Vec<SQLExpr>, String>
		where It: Iterator<Item=&'a Token> {

		self.consume_punctuator("(", tokens);

		let mut columns: Vec<SQLExpr> = Vec::new();
		columns.push(self.parse_identifier(tokens)?);
		while self.consume_punctuator(",", tokens) {
			columns.push(self.parse_identifier(tokens)?);
		}
		self.consume_punctuator(")", tokens);

		Ok(columns)
	}

	fn parse_column_def<'a, It>(&self, tokens: &mut Peekable<It>) -> Result<SQLExpr, String>
		where It: Iterator<Item=&'a Token> {

		let column = try!(self.parse_identifier(tokens));
		let data_type = try!(self.parse_data_type(tokens));
		let qualifiers = try!(self.parse_column_qualifiers(tokens));
		match tokens.peek() {
			Some(&&Token::Punctuator(ref p)) => match &p as &str {
				"," | ")" => {},
				_ => return Err(String::from(format!("Unsupported token in column definition: {:?}", tokens.peek())))
			},
			_ => return Err(String::from(format!("Unsupported token in column definition: {:?}", tokens.peek())))
		}

		Ok(SQLExpr::SQLColumnDef{column: Box::new(column), data_type: Box::new(data_type), qualifiers: qualifiers})
	}

	fn parse_column_qualifiers<'a, It>(&self, tokens: &mut Peekable<It>) ->  Result<Option<Vec<SQLExpr>>, String>
		where It: Iterator<Item=&'a Token> {

		let mut ret: Vec<SQLExpr> = Vec::new();

		while let Some(cq) = try!(self.parse_column_qualifier(tokens)) {
			ret.push(cq);
		}

		if ret.len() > 0 {
			Ok(Some(ret))
		} else {
			Ok(None)
		}
	}

	fn parse_column_qualifier<'a, It>(&self, tokens: &mut Peekable<It>) ->  Result<Option<SQLExpr>, String>
		where It: Iterator<Item=&'a Token> {

		println!("parse_column_qualifier() {:?}", tokens.peek());
		match tokens.peek() {
			Some(&&Token::Keyword(ref v)) | Some(&&Token::Identifier(ref v)) => match &v.to_uppercase() as &str {
				"NOT" => {
					tokens.next();
					if self.consume_keyword("NULL", tokens) {
						Ok(Some(SQLExpr::SQLColumnQualifier(ColumnQualifier::NotNull)))
					} else {
						Err(format!("Expected NOT NULL, received NOT {:?}", tokens.peek()))
					}
				},
				"NULL" => {
					tokens.next();
					Ok(Some(SQLExpr::SQLColumnQualifier(ColumnQualifier::Null)))
				},
				"AUTO_INCREMENT" => {
					tokens.next();
					Ok(Some(SQLExpr::SQLColumnQualifier(ColumnQualifier::AutoIncrement)))
				},
				"PRIMARY" => {
					tokens.next();
					if self.consume_keyword("KEY", tokens) {
						Ok(Some(SQLExpr::SQLColumnQualifier(ColumnQualifier::PrimaryKey)))
					} else {
						Err(format!("Expected PRIMARY KEY, received PRIMARY {:?}", tokens.peek()))
					}
				},
				"UNIQUE" => {
					tokens.next();
					Ok(Some(SQLExpr::SQLColumnQualifier(ColumnQualifier::UniqueKey)))
				},
				"DEFAULT" => {
					tokens.next();
					Ok(Some(SQLExpr::SQLColumnQualifier(ColumnQualifier::Default(Box::new(try!(self.parse_expr(tokens, 0)))))))
				},
				"CHARACTER" => {
					tokens.next();
					if self.consume_keyword("SET", tokens) {
						Ok(Some(SQLExpr::SQLColumnQualifier(ColumnQualifier::CharacterSet(Box::new(try!(self.parse_expr(tokens, 0)))))))
					} else {
						Err(format!("Expected PRIMARY KEY, received PRIMARY {:?}", tokens.peek()))
					}
				},
				"COLLATE" => {
					tokens.next();
					Ok(Some(SQLExpr::SQLColumnQualifier(ColumnQualifier::Collate(Box::new(try!(self.parse_expr(tokens, 0)))))))
				},
				"SIGNED" => {
					tokens.next();
					Ok(Some(SQLExpr::SQLColumnQualifier(ColumnQualifier::Signed)))
				},
				"UNSIGNED" => {
					tokens.next();
					Ok(Some(SQLExpr::SQLColumnQualifier(ColumnQualifier::Unsigned)))
				},
				"ON" => {
					tokens.next();
					if self.consume_keyword("UPDATE", tokens) {
						Ok(Some(SQLExpr::SQLColumnQualifier(ColumnQualifier::OnUpdate(Box::new(try!(self.parse_expr(tokens, 0)))))))
					} else {
						Err(format!("Expected ON UPDATE, received ON {:?}", tokens.peek()))
					}
				},
				"COMMENT" => {
					tokens.next();
					Ok(Some(SQLExpr::SQLColumnQualifier(ColumnQualifier::Comment(Box::new(try!(self.parse_expr(tokens, 0)))))))
				}
				_ => Ok(None)
			},
			_ => Ok(None)
		}
	}

	fn parse_data_type<'a, It>(&self, tokens: &mut Peekable<It>) ->  Result<SQLExpr, String>
		where It: Iterator<Item=&'a Token> {

		let data_token = tokens.next();
		match data_token {

			Some(&Token::Keyword(ref t)) | Some(&Token::Identifier(ref t)) => match &t.to_uppercase() as &str {
				"BIT" => Ok(SQLExpr::SQLDataType(DataType::Bit{display: try!(self.parse_optional_display(tokens))})),
				"TINYINT" => Ok(SQLExpr::SQLDataType(DataType::TinyInt{display: try!(self.parse_optional_display(tokens))})),
				"SMALLINT" => Ok(SQLExpr::SQLDataType(DataType::SmallInt{display: try!(self.parse_optional_display(tokens))})),
				"MEDIUMINT" => Ok(SQLExpr::SQLDataType(DataType::MediumInt{display: try!(self.parse_optional_display(tokens))})),
				"INT" | "INTEGER" => Ok(SQLExpr::SQLDataType(DataType::Int{display: try!(self.parse_optional_display(tokens))})),
				"BIGINT" => Ok(SQLExpr::SQLDataType(DataType::BigInt{display: try!(self.parse_optional_display(tokens))})),
				"DECIMAL" | "DEC" => {
					match try!(self.parse_optional_precision_and_scale(tokens)) {
						Some((p, s)) => Ok(SQLExpr::SQLDataType(DataType::Decimal{precision: Some(p), scale: s})),
						None => Ok(SQLExpr::SQLDataType(DataType::Decimal{precision: None, scale: None}))
					}
				},
				"FLOAT" => {
					match try!(self.parse_optional_precision_and_scale(tokens)) {
						Some((p, s)) => Ok(SQLExpr::SQLDataType(DataType::Float{precision: Some(p), scale: s})),
						None => Ok(SQLExpr::SQLDataType(DataType::Float{precision: None, scale: None}))
					}
				},
				"DOUBLE" => {
					match try!(self.parse_optional_precision_and_scale(tokens)) {
						Some((p, s)) => Ok(SQLExpr::SQLDataType(DataType::Double{precision: Some(p), scale: s})),
						None => Ok(SQLExpr::SQLDataType(DataType::Double{precision: None, scale: None}))
					}
				},
				"BOOL" | "BOOLEAN" => Ok(SQLExpr::SQLDataType(DataType::Bool)),
				"DATE" => Ok(SQLExpr::SQLDataType(DataType::Date)),
				"DATETIME" => Ok(SQLExpr::SQLDataType(DataType::DateTime{fsp: try!(self.parse_optional_display(tokens))})),
				"TIMESTAMP" => Ok(SQLExpr::SQLDataType(DataType::Timestamp{fsp: try!(self.parse_optional_display(tokens))})),
				"TIME" => Ok(SQLExpr::SQLDataType(DataType::Time{fsp: try!(self.parse_optional_display(tokens))})),
				"YEAR" => Ok(SQLExpr::SQLDataType(DataType::Year{display: try!(self.parse_optional_display(tokens))})),
				// TODO do something with NATIONAL, NCHAR, etc
				"NATIONAL" => {
					if self.consume_keyword(&"CHAR", tokens) {
						Ok(SQLExpr::SQLDataType(DataType::NChar{length: try!(self.parse_optional_display(tokens))}))
					} else if self.consume_keyword(&"VARCHAR", tokens) {
						Ok(SQLExpr::SQLDataType(DataType::NVarchar{length: try!(self.parse_optional_display(tokens))}))
					} else if self.consume_keyword(&"CHARACTER", tokens) {
						if self.consume_keyword(&"VARYING", tokens) {
							Ok(SQLExpr::SQLDataType(DataType::NVarchar{length: try!(self.parse_optional_display(tokens))}))
						} else {
							Ok(SQLExpr::SQLDataType(DataType::NChar{length: try!(self.parse_optional_display(tokens))}))
						}
					} else {
						Err(format!("Expected NATIONAL CHAR|VARCHAR|CHARACTER [VARYING], received NATIONAL {:?}", tokens.peek()))
					}
				},
				"CHAR" => {
					let length = try!(self.parse_optional_display(tokens));
					if self.consume_keyword(&"BYTE", tokens) {
						Ok(SQLExpr::SQLDataType(DataType::CharByte{length: length}))
					} else {
						Ok(SQLExpr::SQLDataType(DataType::Char{length: length}))
					}
				},
				"NCHAR" => {
					let ret = Ok(SQLExpr::SQLDataType(DataType::NChar{length: try!(self.parse_optional_display(tokens))}));
					ret
				},
				"CHARACTER" => {
					if self.consume_keyword("VARYING", tokens) {
						Ok(SQLExpr::SQLDataType(DataType::Varchar{length: try!(self.parse_optional_display(tokens))}))
					} else {
						Ok(SQLExpr::SQLDataType(DataType::Char{length: try!(self.parse_optional_display(tokens))}))
					}
				},
				"VARCHAR" => Ok(SQLExpr::SQLDataType(DataType::Varchar{length: try!(self.parse_optional_display(tokens))})),
				"NVARCHAR" => Ok(SQLExpr::SQLDataType(DataType::NVarchar{length: try!(self.parse_optional_display(tokens))})),
				"BINARY" => Ok(SQLExpr::SQLDataType(DataType::Binary{length: try!(self.parse_optional_display(tokens))})),
				"VARBINARY" => Ok(SQLExpr::SQLDataType(DataType::VarBinary{length: try!(self.parse_optional_display(tokens))})),
				"TINYBLOB" => Ok(SQLExpr::SQLDataType(DataType::TinyBlob)),
				"TINYTEXT" => Ok(SQLExpr::SQLDataType(DataType::TinyText)),
				"MEDIUMBLOB" => Ok(SQLExpr::SQLDataType(DataType::MediumBlob)),
				"MEDIUMTEXT" => Ok(SQLExpr::SQLDataType(DataType::MediumText)),
				"LONGBLOB" => Ok(SQLExpr::SQLDataType(DataType::LongBlob)),
				"LONGTEXT" => Ok(SQLExpr::SQLDataType(DataType::LongText)),
				"BLOB" => Ok(SQLExpr::SQLDataType(DataType::Blob{length: try!(self.parse_optional_display(tokens))})),
				"TEXT" => Ok(SQLExpr::SQLDataType(DataType::Text{length: try!(self.parse_optional_display(tokens))})),
				"ENUM" => {
					self.consume_punctuator("(", tokens);
					let values = try!(self.parse_expr_list(tokens));
					self.consume_punctuator(")", tokens);
					Ok(SQLExpr::SQLDataType(DataType::Enum{values: Box::new(values)}))
				},
				"SET" => {
					self.consume_punctuator("(", tokens);
					let values = try!(self.parse_expr_list(tokens));
					self.consume_punctuator(")", tokens);
					Ok(SQLExpr::SQLDataType(DataType::Set{values: Box::new(values)}))
				},
				_ => Err(format!("Data type not recognized {}", t))
			},
			_ => Err(format!("Expected data type, received token {:?}", tokens.peek()))
		}
	}

	fn parse_optional_display<'a, It>(&self, tokens: &mut Peekable<It>) -> Result<Option<u32>, String>
		where It: Iterator<Item=&'a Token> {

		if self.consume_punctuator("(", tokens) {
			match tokens.peek() {
				Some(&&Token::Literal(LiteralToken::LiteralLong(_, ref v))) => {
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

	fn parse_optional_precision_and_scale<'a, It>(&self, tokens: &mut Peekable<It>) -> Result<Option<(u32,Option<u32>)>, String>
		where It: Iterator<Item=&'a Token> {

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

	fn parse_long<'a, It>(&self, tokens: &mut Peekable<It>) -> Result<u32, String>
		where It: Iterator<Item=&'a Token> {

		match tokens.peek() {
			Some(&&Token::Literal(LiteralToken::LiteralLong(_, ref v))) => {
				tokens.next();
				Ok(u32::from_str(&v).unwrap())
			},
			_ => Err(String::from(format!("Expected LiteralLong token, received {:?}", tokens.peek())))
		}
	}

	fn parse_insert<'a, It>(&self, tokens: &mut Peekable<It>) -> Result<SQLExpr,  String>
		where It: Iterator<Item=&'a Token> {

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

	fn parse_select<'a, It>(&self, tokens: &mut Peekable<It>) -> Result<SQLExpr,  String>
		where It: Iterator<Item=&'a Token> {

		println!("parse_select()");
		// consume the SELECT
		tokens.next();
		let proj = Box::new(try!(self.parse_expr_list(tokens)));

		let from = match tokens.peek() {
			Some(&&Token::Keyword(ref t)) => match &t as &str {
				"FROM" => {
					tokens.next();
					Some(Box::new(try!(self.parse_relation(tokens))))
				},
				_ => None
			},
			_ => return Err(format!("unexpected token {:?}", tokens.peek()))
		};

		let whr = match tokens.peek() {
			Some(&&Token::Keyword(ref t)) => match &t as &str {
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

	fn parse_update<'a, It>(&self, tokens: &mut Peekable<It>) -> Result<SQLExpr, String>
		where It: Iterator<Item=&'a Token> {

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
	fn parse_relation<'a, It>(&self, tokens: &mut Peekable<It>) -> Result<SQLExpr,  String>
		where It: Iterator<Item=&'a Token> {

		self.parse_expr(tokens, 4)
	}

	fn parse_expr_list<'a, It>(&self, tokens: &mut Peekable<It>) -> Result<SQLExpr,  String>
		where It: Iterator<Item=&'a Token> {

		println!("parse_expr_list()");
		let first = try!(self.parse_expr(tokens, 0_u32));
		let mut v: Vec<SQLExpr> = Vec::new();
		v.push(first);
		while let Some(&&Token::Punctuator(ref p)) = tokens.peek() {
			if p == "," {
				tokens.next();
				v.push(try!(self.parse_expr(tokens, 0_u32)));
			} else {
				break;
			}
		}
		Ok(SQLExpr::SQLExprList(v))
	}

	fn parse_order_by_list<'a, It>(&self, tokens: &mut Peekable<It>) -> Result<SQLExpr,  String>
		where It: Iterator<Item=&'a Token> {

		println!("parse_order_by_list()");
		let mut v: Vec<SQLExpr> = Vec::new();
		v.push(try!(self.parse_order_by_expr(tokens)));
		while let Some(&&Token::Punctuator(ref p)) = tokens.peek() {
			if p == "," {
				tokens.next();
				v.push(try!(self.parse_order_by_expr(tokens)));
			} else {
				break;
			}
		}
		Ok(SQLExpr::SQLExprList(v))
	}

	fn parse_order_by_expr<'a, It>(&self, tokens: &mut Peekable<It>) -> Result<SQLExpr,  String>
		where It: Iterator<Item=&'a Token> {

		let e = try!(self.parse_expr(tokens, 0_u32));
		Ok(SQLExpr::SQLOrderBy {expr: Box::new(e), is_asc: self.is_asc(tokens)})
	}

	fn is_asc<'a, It>(&self, tokens: &mut Peekable<It>) -> bool
		where It: Iterator<Item=&'a Token> {

		if self.consume_keyword(&"DESC", tokens) {
			false
		} else {
			self.consume_keyword(&"ASC", tokens);
			true
		}
	}

	fn parse_binary<'a, It>(&self, left: SQLExpr, tokens: &mut Peekable<It>) -> Result<SQLExpr,  String>
		where It: Iterator<Item=&'a Token> {

		println!("parse_binary()");
		let precedence = self.get_precedence(tokens);
		// determine operator
		let operator = match tokens.next().unwrap() {
			&Token::Operator(ref t) => match &t as &str {
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

	fn parse_identifier<'a, It>(&self, tokens: &mut Peekable<It>) -> Result<SQLExpr,  String>
		where It: Iterator<Item=&'a Token> {

		println!("parse_identifier()");
		match tokens.next().unwrap() {
			&Token::Identifier(ref v) => Ok(SQLExpr::SQLIdentifier(v.clone())),
			&Token::Operator(ref o) => match &o as &str {
				"*" => Ok(SQLExpr::SQLIdentifier(o.clone())),
				_ => Err(format!("Unsupported operator as identifier {}", o))
			},
			_ => Err(format!("Illegal state"))
		}
	}

	fn parse_nested<'a, It>(&self, tokens: &mut Peekable<It>) -> Result<SQLExpr,  String>
		where It: Iterator<Item=&'a Token> {

		//consume (
		tokens.next();
		let nested = try!(self.parse_expr(tokens, 0));
		// consume )
		match tokens.peek() {
			Some(&&Token::Punctuator(ref v)) => match &v as &str {
				")" => {tokens.next();},
				_ => return Err(format!("Expected , punctuator, received {}", v))
			},
			_ => return Err(format!("Illegal state, expected , received {:?}", tokens.peek()))
		}

		Ok(SQLExpr::SQLNested(Box::new(nested)))
	}

	fn parse_unary<'a, It>(&self, tokens: & mut Peekable<It>) -> Result<SQLExpr,  String>
		where It: Iterator<Item=&'a Token> {

		let precedence = self.get_precedence(tokens);
		let op = match tokens.next() {
			Some(&Token::Operator(ref o)) => match &o as &str {
				"+" => SQLOperator::ADD,
				"-" => SQLOperator::SUB,
				_ => return Err(format!("Illegal operator for unary {}", o))
			},
			_ => return Err(format!("Illegal state"))
		};
		Ok(SQLExpr::SQLUnary{operator: op, expr: Box::new(try!(self.parse_expr(tokens, precedence)))})

	}

	fn parse_union<'a, It>(&self, left: SQLExpr, tokens: &mut Peekable<It>) -> Result<SQLExpr,  String>
		where It: Iterator<Item=&'a Token> {

		// consume the UNION
		tokens.next();

		let union_type = match tokens.peek() {
			Some(&&Token::Keyword(ref t)) => match &t as &str {
				"ALL" => SQLUnionType::ALL,
				"DISTINCT" => SQLUnionType::DISTINCT,
				_ => SQLUnionType::UNION
			},
			_ => SQLUnionType::UNION
		};

		let right = Box::new(try!(self.parse_expr(tokens, 0)));

		Ok(SQLExpr::SQLUnion{left: Box::new(left), union_type: union_type, right: right})

	}

	fn parse_join<'a, It>(&self, left: SQLExpr, tokens: &mut Peekable<It>) -> Result<SQLExpr,  String>
		where It: Iterator<Item=&'a Token> {

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

	fn parse_alias<'a, It>(&self, left: SQLExpr, tokens: &mut Peekable<It>) -> Result<SQLExpr,  String>
		where It: Iterator<Item=&'a Token> {

		if self.consume_keyword(&"AS", tokens) {
			Ok(SQLExpr::SQLAlias{expr: Box::new(left), alias: Box::new(try!(self.parse_identifier(tokens)))})
		} else {
			Err(format!("Illegal state, expected AS, received token {:?}", tokens.peek()))
		}
	}

	// TODO more helper methods like consume_keyword_sequence, required_keyword_sequence, etc
	fn consume_keyword<'a, It>(&self, text: &str, tokens: &mut Peekable<It>) -> bool
		where It: Iterator<Item=&'a Token> {

		match tokens.peek() {
			Some(&&Token::Keyword(ref v)) | Some(&&Token::Identifier(ref v)) => {
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

	fn consume_punctuator<'a, It>(&self, text: &str, tokens: &mut Peekable<It>) -> bool
		where It: Iterator<Item=&'a Token> {

		match tokens.peek() {
			Some(&&Token::Punctuator(ref v)) => {
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

	fn consume_operator<'a, It>(&self, text: &str, tokens: &mut Peekable<It>) -> bool
		where It: Iterator<Item=&'a Token> {

		match tokens.peek() {
			Some(&&Token::Operator(ref v)) => {
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
