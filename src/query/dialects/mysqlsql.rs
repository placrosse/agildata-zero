use super::super::*;
use super::ansisql::*;

use std::iter::Peekable;
use std::str::Chars;
use std::str::FromStr;

static KEYWORDS: &'static [&'static str] = &["SHOW", "CREATE", "TABLE", "PRECISION",
	"PRIMARY", "KEY", "UNIQUE", "FULLTEXT", "FOREIGN", "REFERENCES", "CONSTRAINT"];

pub struct MySQLDialect<'d>{
	ansi: &'d AnsiSQLDialect
}

impl <'d> Dialect for MySQLDialect<'d> {

	fn get_keywords(&self) -> &'static [&'static str] {
        KEYWORDS
    }

	fn get_token(&self, chars: &mut Peekable<Chars>, keywords: &Vec<&'static str>) -> Result<Option<Token>, String> {
		match chars.peek() {
			Some(&ch) => match ch {
				'`' => {
					chars.next();
					let mut text = String::new();
	                while let Some(&c) = chars.peek() { // will break when it.peek() => None

						if c != '`' {
							text.push(c);
						} else {
							chars.next();
							break;
						}
	                }

					Ok(Some(Token::Identifier(text)))
				},
				_ => self.ansi.get_token(chars, keywords)
			},
			_ => self.ansi.get_token(chars, keywords)
		}
	}

	fn parse_prefix<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) ->
            Result<Option<ASTNode>, String> {

        match tokens.peek() {
			Some(&Token::Keyword(ref v)) => match &v as &str {
				"CREATE" => Ok(Some(self.parse_create(tokens)?)),
				_ => self.ansi.parse_prefix(tokens)
			},
			_ => self.ansi.parse_prefix(tokens)
		}
    }

    fn get_precedence<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>)-> Result<u8, String> {
        self.ansi.get_precedence(tokens)
    }

    fn parse_infix<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>, left: ASTNode, precedence: u8)-> Result<Option<ASTNode>, String> {
        self.ansi.parse_infix(tokens, left, precedence)
    }

}

impl<'d> MySQLDialect<'d> {
	pub fn new(ansi: &'d AnsiSQLDialect) -> Self {MySQLDialect{ansi: ansi}}

	fn parse_create<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<ASTNode, String>
		 {

		tokens.consume_keyword("CREATE");

		if tokens.consume_keyword("TABLE") {
			let table = self.ansi.parse_identifier(tokens)?;
			tokens.consume_punctuator("(");

			let mut columns: Vec<ASTNode> = Vec::new();
			let mut keys: Vec<ASTNode> = Vec::new();

			columns.push(try!(self.parse_column_def(tokens)));
			while tokens.consume_punctuator(",") {
				match tokens.peek() {
					Some(&Token::Keyword(ref v)) => match &v as &str {
						"PRIMARY" | "KEY" | "UNIQUE" | "FULLTEXT" | "FOREIGN" | "CONSTRAINT" => keys.push(try!(self.parse_key_def(tokens))),
						_ => columns.push(try!(self.parse_column_def(tokens)))
					},
					_ => columns.push(try!(self.parse_column_def(tokens)))
				}
			}

			if !tokens.consume_punctuator(")") {
				return Err(String::from(format!("Expected token ) received token {:?}", tokens.peek())))
			}

			let table_options = self.parse_table_options(tokens)?;

			match tokens.peek() {
				None => Ok(ASTNode::MySQLCreateTable{
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

	fn parse_table_options<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<Vec<ASTNode>, String>
		 {

		let mut ret: Vec<ASTNode> = Vec::new();

		while let Some(o) = self.parse_table_option(tokens)? {
			ret.push(o);
		}
		Ok(ret)
	}

	fn parse_table_option<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<Option<ASTNode>, String>
		 {

		match tokens.peek() {
			Some(&Token::Keyword(ref v)) | Some(&Token::Identifier(ref v)) => match &v.to_uppercase() as &str {
				"ENGINE" => {
					tokens.next();
					tokens.consume_operator("=");
					Ok(Some(ASTNode::MySQLTableOption(MySQLTableOption::Engine(Box::new(tokens.parse_expr(0)?)))))
				},
				"DEFAULT" => { // [DEFAULT] [CHARACTER SET | COLLATE]
					tokens.next();
					self.parse_table_option(tokens)
				},
				"CHARACTER" | "CHARSET" => {
					tokens.next();
					tokens.consume_keyword("SET");
					Ok(Some(ASTNode::MySQLTableOption(MySQLTableOption::Charset(Box::new(tokens.parse_expr(0)?)))))
				},
				"COMMENT" => {
					tokens.next();
					Ok(Some(ASTNode::MySQLTableOption(MySQLTableOption::Comment(Box::new(tokens.parse_expr(0)?)))))
				},
				"AUTO_INCREMENT" => {
					tokens.next();
					Ok(Some(ASTNode::MySQLTableOption(MySQLTableOption::AutoIncrement(Box::new(tokens.parse_expr(0)?)))))
				},
				// "COLLATE"
				_ => Err(String::from(format!("Unsupported Table Option {}", v)))
			},
			_ => Ok(None)
		}
	}

	fn parse_key_def<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<ASTNode, String>
		 {

		println!("parse_key_def()");

		let symbol = if tokens.consume_keyword("CONSTRAINT") {
			Some(Box::new(self.ansi.parse_identifier(tokens)?))
		} else {
			None
		};

		let t = tokens.next();

		match t {
			Some(&Token::Keyword(ref v)) => match &v as &str {
				"PRIMARY" => {
					tokens.consume_keyword("KEY");
					Ok(ASTNode::MySQLKeyDef(MySQLKeyDef::Primary{
						symbol: symbol,
						name: self.parse_optional_key_name(tokens)?,
						columns: self.parse_key_column_list(tokens)?
					}))
				},
				"UNIQUE" => {
					tokens.consume_keyword("KEY");
					Ok(ASTNode::MySQLKeyDef(MySQLKeyDef::Unique{
						symbol: symbol,
						name: self.parse_optional_key_name(tokens)?,
						columns: self.parse_key_column_list(tokens)?
					}))
				},
				"FOREIGN" => {
					tokens.consume_keyword("KEY");
					let name = self.parse_optional_key_name(tokens)?;
					let columns = self.parse_key_column_list(tokens)?;
					tokens.consume_keyword("REFERENCES");

					Ok(ASTNode::MySQLKeyDef(MySQLKeyDef::Foreign{
						symbol: symbol,
						name: name,
						columns: columns,
						reference_table: Box::new(self.ansi.parse_identifier(tokens)?),
						reference_columns: self.parse_key_column_list(tokens)?
					}))
				},
				"FULLTEXT" => {
					tokens.consume_keyword("KEY");
					Ok(ASTNode::MySQLKeyDef(MySQLKeyDef::FullText{
						name: self.parse_optional_key_name(tokens)?,
						columns: self.parse_key_column_list(tokens)?
					}))
				},
				"KEY" => {
					tokens.consume_keyword("KEY");
					Ok(ASTNode::MySQLKeyDef(MySQLKeyDef::Index{
						name: self.parse_optional_key_name(tokens)?,
						columns: self.parse_key_column_list(tokens)?
					}))
				},
				_ => Err(String::from(format!("Unsupported key definition prefix {}", v)))
			},
			_ => Err(String::from(format!("Expected key definition received token {:?}", t)))
		}
	}

	fn parse_optional_key_name<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<Option<Box<ASTNode>>, String>
		 {

		match tokens.peek() {
			Some(&Token::Identifier(_)) => Ok(Some(Box::new(self.ansi.parse_identifier(tokens)?))),
			_ => Ok(None)
		}
	}

	fn parse_key_column_list<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<Vec<ASTNode>, String>
		 {

		tokens.consume_punctuator("(");

		let mut columns: Vec<ASTNode> = Vec::new();
		columns.push(self.ansi.parse_identifier(tokens)?);
		while tokens.consume_punctuator(",") {
			columns.push(self.ansi.parse_identifier(tokens)?);
		}
		tokens.consume_punctuator(")");

		Ok(columns)
	}

	fn parse_column_def<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<ASTNode, String>
		 {

		let column = try!(self.ansi.parse_identifier(tokens));
		let data_type = try!(self.parse_data_type(tokens));
		let qualifiers = try!(self.parse_column_qualifiers(tokens));
		match tokens.peek() {
			Some(&Token::Punctuator(ref p)) => match &p as &str {
				"," | ")" => {},
				_ => return Err(String::from(format!("Unsupported token in column definition: {:?}", tokens.peek())))
			},
			_ => return Err(String::from(format!("Unsupported token in column definition: {:?}", tokens.peek())))
		}

		Ok(ASTNode::MySQLColumnDef{column: Box::new(column), data_type: Box::new(data_type), qualifiers: qualifiers})
	}

	fn parse_column_qualifiers<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) ->  Result<Option<Vec<ASTNode>>, String>
		 {

		let mut ret: Vec<ASTNode> = Vec::new();

		while let Some(cq) = try!(self.parse_column_qualifier(tokens)) {
			ret.push(cq);
		}

		if ret.len() > 0 {
			Ok(Some(ret))
		} else {
			Ok(None)
		}
	}

	fn parse_column_qualifier<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) ->  Result<Option<ASTNode>, String>
		 {

		println!("parse_column_qualifier() {:?}", tokens.peek());
		match tokens.peek() {
			Some(&Token::Keyword(ref v)) | Some(&Token::Identifier(ref v)) => match &v.to_uppercase() as &str {
				"NOT" => {
					tokens.next();
					if tokens.consume_keyword("NULL") {
						Ok(Some(ASTNode::MySQLColumnQualifier(MySQLColumnQualifier::NotNull)))
					} else {
						Err(format!("Expected NOT NULL, received NOT {:?}", tokens.peek()))
					}
				},
				"NULL" => {
					tokens.next();
					Ok(Some(ASTNode::MySQLColumnQualifier(MySQLColumnQualifier::Null)))
				},
				"AUTO_INCREMENT" => {
					tokens.next();
					Ok(Some(ASTNode::MySQLColumnQualifier(MySQLColumnQualifier::AutoIncrement)))
				},
				"PRIMARY" => {
					tokens.next();
					if tokens.consume_keyword("KEY") {
						Ok(Some(ASTNode::MySQLColumnQualifier(MySQLColumnQualifier::PrimaryKey)))
					} else {
						Err(format!("Expected PRIMARY KEY, received PRIMARY {:?}", tokens.peek()))
					}
				},
				"UNIQUE" => {
					tokens.next();
					Ok(Some(ASTNode::MySQLColumnQualifier(MySQLColumnQualifier::UniqueKey)))
				},
				"DEFAULT" => {
					tokens.next();
					Ok(Some(ASTNode::MySQLColumnQualifier(MySQLColumnQualifier::Default(Box::new(try!(tokens.parse_expr(0)))))))
				},
				"CHARACTER" => {
					tokens.next();
					if tokens.consume_keyword("SET") {
						Ok(Some(ASTNode::MySQLColumnQualifier(MySQLColumnQualifier::CharacterSet(Box::new(try!(tokens.parse_expr(0)))))))
					} else {
						Err(format!("Expected PRIMARY KEY, received PRIMARY {:?}", tokens.peek()))
					}
				},
				"COLLATE" => {
					tokens.next();
					Ok(Some(ASTNode::MySQLColumnQualifier(MySQLColumnQualifier::Collate(Box::new(try!(tokens.parse_expr(0)))))))
				},
				"SIGNED" => {
					tokens.next();
					Ok(Some(ASTNode::MySQLColumnQualifier(MySQLColumnQualifier::Signed)))
				},
				"UNSIGNED" => {
					tokens.next();
					Ok(Some(ASTNode::MySQLColumnQualifier(MySQLColumnQualifier::Unsigned)))
				},
				"ON" => {
					tokens.next();
					if tokens.consume_keyword("UPDATE") {
						Ok(Some(ASTNode::MySQLColumnQualifier(MySQLColumnQualifier::OnUpdate(Box::new(try!(tokens.parse_expr(0)))))))
					} else {
						Err(format!("Expected ON UPDATE, received ON {:?}", tokens.peek()))
					}
				},
				"COMMENT" => {
					tokens.next();
					Ok(Some(ASTNode::MySQLColumnQualifier(MySQLColumnQualifier::Comment(Box::new(try!(tokens.parse_expr(0)))))))
				}
				_ => Ok(None)
			},
			_ => Ok(None)
		}
	}

	fn parse_data_type<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) ->  Result<ASTNode, String>
		 {

		let data_token = tokens.next();
		match data_token {

			Some(&Token::Keyword(ref t)) | Some(&Token::Identifier(ref t)) => match &t.to_uppercase() as &str {
				"BIT" => Ok(ASTNode::MySQLDataType(MySQLDataType::Bit{display: try!(self.parse_optional_display(tokens))})),
				"TINYINT" => Ok(ASTNode::MySQLDataType(MySQLDataType::TinyInt{display: try!(self.parse_optional_display(tokens))})),
				"SMALLINT" => Ok(ASTNode::MySQLDataType(MySQLDataType::SmallInt{display: try!(self.parse_optional_display(tokens))})),
				"MEDIUMINT" => Ok(ASTNode::MySQLDataType(MySQLDataType::MediumInt{display: try!(self.parse_optional_display(tokens))})),
				"INT" | "INTEGER" => Ok(ASTNode::MySQLDataType(MySQLDataType::Int{display: try!(self.parse_optional_display(tokens))})),
				"BIGINT" => Ok(ASTNode::MySQLDataType(MySQLDataType::BigInt{display: try!(self.parse_optional_display(tokens))})),
				"DECIMAL" | "DEC" => {
					match try!(self.parse_optional_precision_and_scale(tokens)) {
						Some((p, s)) => Ok(ASTNode::MySQLDataType(MySQLDataType::Decimal{precision: Some(p), scale: s})),
						None => Ok(ASTNode::MySQLDataType(MySQLDataType::Decimal{precision: None, scale: None}))
					}
				},
				"FLOAT" => {
					match try!(self.parse_optional_precision_and_scale(tokens)) {
						Some((p, s)) => Ok(ASTNode::MySQLDataType(MySQLDataType::Float{precision: Some(p), scale: s})),
						None => Ok(ASTNode::MySQLDataType(MySQLDataType::Float{precision: None, scale: None}))
					}
				},
				"DOUBLE" => {
					match try!(self.parse_optional_precision_and_scale(tokens)) {
						Some((p, s)) => Ok(ASTNode::MySQLDataType(MySQLDataType::Double{precision: Some(p), scale: s})),
						None => Ok(ASTNode::MySQLDataType(MySQLDataType::Double{precision: None, scale: None}))
					}
				},
				"BOOL" | "BOOLEAN" => Ok(ASTNode::MySQLDataType(MySQLDataType::Bool)),
				"DATE" => Ok(ASTNode::MySQLDataType(MySQLDataType::Date)),
				"DATETIME" => Ok(ASTNode::MySQLDataType(MySQLDataType::DateTime{fsp: try!(self.parse_optional_display(tokens))})),
				"TIMESTAMP" => Ok(ASTNode::MySQLDataType(MySQLDataType::Timestamp{fsp: try!(self.parse_optional_display(tokens))})),
				"TIME" => Ok(ASTNode::MySQLDataType(MySQLDataType::Time{fsp: try!(self.parse_optional_display(tokens))})),
				"YEAR" => Ok(ASTNode::MySQLDataType(MySQLDataType::Year{display: try!(self.parse_optional_display(tokens))})),
				// TODO do something with NATIONAL, NCHAR, etc
				"NATIONAL" => {
					if tokens.consume_keyword(&"CHAR") {
						Ok(ASTNode::MySQLDataType(MySQLDataType::NChar{length: try!(self.parse_optional_display(tokens))}))
					} else if tokens.consume_keyword(&"VARCHAR") {
						Ok(ASTNode::MySQLDataType(MySQLDataType::NVarchar{length: try!(self.parse_optional_display(tokens))}))
					} else if tokens.consume_keyword(&"CHARACTER") {
						if tokens.consume_keyword(&"VARYING") {
							Ok(ASTNode::MySQLDataType(MySQLDataType::NVarchar{length: try!(self.parse_optional_display(tokens))}))
						} else {
							Ok(ASTNode::MySQLDataType(MySQLDataType::NChar{length: try!(self.parse_optional_display(tokens))}))
						}
					} else {
						Err(format!("Expected NATIONAL CHAR|VARCHAR|CHARACTER [VARYING], received NATIONAL {:?}", tokens.peek()))
					}
				},
				"CHAR" => {
					let length = try!(self.parse_optional_display(tokens));
					if tokens.consume_keyword(&"BYTE") {
						Ok(ASTNode::MySQLDataType(MySQLDataType::CharByte{length: length}))
					} else {
						Ok(ASTNode::MySQLDataType(MySQLDataType::Char{length: length}))
					}
				},
				"NCHAR" => {
					let ret = Ok(ASTNode::MySQLDataType(MySQLDataType::NChar{length: try!(self.parse_optional_display(tokens))}));
					ret
				},
				"CHARACTER" => {
					if tokens.consume_keyword("VARYING") {
						Ok(ASTNode::MySQLDataType(MySQLDataType::Varchar{length: try!(self.parse_optional_display(tokens))}))
					} else {
						Ok(ASTNode::MySQLDataType(MySQLDataType::Char{length: try!(self.parse_optional_display(tokens))}))
					}
				},
				"VARCHAR" => Ok(ASTNode::MySQLDataType(MySQLDataType::Varchar{length: try!(self.parse_optional_display(tokens))})),
				"NVARCHAR" => Ok(ASTNode::MySQLDataType(MySQLDataType::NVarchar{length: try!(self.parse_optional_display(tokens))})),
				"BINARY" => Ok(ASTNode::MySQLDataType(MySQLDataType::Binary{length: try!(self.parse_optional_display(tokens))})),
				"VARBINARY" => Ok(ASTNode::MySQLDataType(MySQLDataType::VarBinary{length: try!(self.parse_optional_display(tokens))})),
				"TINYBLOB" => Ok(ASTNode::MySQLDataType(MySQLDataType::TinyBlob)),
				"TINYTEXT" => Ok(ASTNode::MySQLDataType(MySQLDataType::TinyText)),
				"MEDIUMBLOB" => Ok(ASTNode::MySQLDataType(MySQLDataType::MediumBlob)),
				"MEDIUMTEXT" => Ok(ASTNode::MySQLDataType(MySQLDataType::MediumText)),
				"LONGBLOB" => Ok(ASTNode::MySQLDataType(MySQLDataType::LongBlob)),
				"LONGTEXT" => Ok(ASTNode::MySQLDataType(MySQLDataType::LongText)),
				"BLOB" => Ok(ASTNode::MySQLDataType(MySQLDataType::Blob{length: try!(self.parse_optional_display(tokens))})),
				"TEXT" => Ok(ASTNode::MySQLDataType(MySQLDataType::Text{length: try!(self.parse_optional_display(tokens))})),
				"ENUM" => {
					tokens.consume_punctuator("(");
					let values = try!(self.ansi.parse_expr_list(tokens));
					tokens.consume_punctuator(")");
					Ok(ASTNode::MySQLDataType(MySQLDataType::Enum{values: Box::new(values)}))
				},
				"SET" => {
					tokens.consume_punctuator("(");
					let values = try!(self.ansi.parse_expr_list(tokens));
					tokens.consume_punctuator(")");
					Ok(ASTNode::MySQLDataType(MySQLDataType::Set{values: Box::new(values)}))
				},
				_ => Err(format!("Data type not recognized {}", t))
			},
			_ => Err(format!("Expected data type, received token {:?}", tokens.peek()))
		}
	}

	fn parse_optional_display<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<Option<u32>, String>
		 {

		if tokens.consume_punctuator("(") {
			match tokens.peek() {
				Some(&Token::Literal(LiteralToken::LiteralLong(_, ref v))) => {
					tokens.next();
					let ret = Ok(Some(u32::from_str(&v).unwrap()));
					tokens.consume_punctuator(")");
					ret
				},
				_ => Err(String::from(format!("Expected LiteralLong token, received {:?}", tokens.peek())))
			}
		} else {
			Ok(None)
		}

	}

	fn parse_optional_precision_and_scale<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<Option<(u32,Option<u32>)>, String>
		 {

		tokens.consume_keyword("PRECISION");

		if tokens.consume_punctuator("(") {
			let p = try!(self.parse_long(tokens));
			let s = if tokens.consume_punctuator(",") {
				Some(try!(self.parse_long(tokens)))
			} else {
				None
			};
			tokens.consume_punctuator(")");
			Ok(Some((p, s)))
		} else {
			Ok(None)
		}

	}

	fn parse_long<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<u32, String>
		 {

		match tokens.peek() {
			Some(&Token::Literal(LiteralToken::LiteralLong(_, ref v))) => {
				tokens.next();
				Ok(u32::from_str(&v).unwrap())
			},
			_ => Err(String::from(format!("Expected LiteralLong token, received {:?}", tokens.peek())))
		}
	}
}
