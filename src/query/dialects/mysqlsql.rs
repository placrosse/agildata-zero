// Copyright 2016 AgilData
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http:// www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::super::*;
use super::ansisql::*;
use error::ZeroError;
use std::iter::Peekable;
use std::str::Chars;
use std::str::FromStr;
use std::fmt::Write;

static KEYWORDS: &'static [&'static str] = &["SHOW", "CREATE", "DROP", "TABLE", "PRECISION",
    "PRIMARY", "KEY", "UNIQUE", "FULLTEXT", "FOREIGN", "REFERENCES", "CONSTRAINT", "USE",
    "COMMIT", "ROLLBACK", "BEGIN"];



pub struct MySQLDialect<'d>{
    ansi: &'d AnsiSQLDialect
}

impl <'d> Dialect for MySQLDialect<'d> {

    fn get_keywords(&self) -> Vec<&'static str> {
        let mut k = self.ansi.get_keywords();
        k.extend_from_slice(KEYWORDS);
        k
    }

    fn get_token(&self, chars: &mut Peekable<Chars>, keywords: &Vec<&'static str>, literals: &mut Vec<LiteralToken>) -> Result<Option<Token>, Box<ZeroError>> {
        match chars.peek() {
            Some(&ch) => match ch {
                '`' => {
                    chars.next();
                    let mut text = String::new();
                    while let Some(c) = chars.next() { // will break when it.peek() => None

                        if c != '`' {
                            text.push(c);
                        } else {
                            break;
                        }
                    }

                    Ok(Some(Token::Identifier(text)))
                },
                _ => self.ansi.get_token(chars, keywords, literals)
            },
            _ => self.ansi.get_token(chars, keywords, literals)
        }
    }

    fn parse_prefix<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) ->
            Result<Option<ASTNode>,  Box<ZeroError>> {

        match tokens.peek() {
            Some(&Token::Keyword(ref v)) => match &v as &str {
                "CREATE" => Ok(Some(self.parse_create(tokens)?)),
				"DROP" => Ok(Some(self.parse_drop(tokens)?)),
                "USE" => Ok(Some(self.parse_use(tokens)?)),
                _ => self.ansi.parse_prefix(tokens)
            },
            _ => self.ansi.parse_prefix(tokens)
        }
    }

    fn get_precedence<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>)-> Result<u8,  Box<ZeroError>> {
        self.ansi.get_precedence(tokens)
    }

    fn parse_infix<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>, left: ASTNode, precedence: u8)-> Result<Option<ASTNode>,  Box<ZeroError>> {
        self.ansi.parse_infix(tokens, left, precedence)
    }

}

impl<'d> MySQLDialect<'d> {
    pub fn new(ansi: &'d AnsiSQLDialect) -> Self {MySQLDialect{ansi: ansi}}

    fn parse_use<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<ASTNode, Box<ZeroError>> {

        assert!(tokens.consume_keyword("USE"));
        Ok(ASTNode::MySQLUse(Box::new(self.ansi.parse_identifier(tokens)?)))
    }

    fn parse_drop<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<ASTNode,  Box<ZeroError>>
    {
        tokens.consume_keyword("DROP");

        // optional keyword
        let temp = tokens.consume_keyword("TEMPORARY");

        if tokens.consume_keyword("TABLE") {

            // optional keywords
            let if_exists = tokens.consume_keyword("IF") && tokens.consume_keyword("EXISTS");

            let mut tables : Vec<ASTNode> = vec![];

            loop {
                tables.push(self.ansi.parse_identifier(tokens)?);
                if !tokens.consume_punctuator(",") {
                    break;
                }
            }

            // optional keywords (can only be one or the other)
            let restrict = tokens.consume_keyword("RESTRICT");
            let cascade = tokens.consume_keyword("CASCASE");

            Ok(ASTNode::MySQLDropTable {
                temporary: temp,
                if_exists: if_exists,
                tables: tables,
                restrict: restrict,
                cascade: cascade
            })

        } else {
            return  Err(ZeroError::ParseError{
                message: format!("Expected keyword TABLE after DROP, received token {:?}", tokens.peek()).into(),
                code: "1064".into()
            }.into())
        }
    }

    fn parse_create<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<ASTNode,  Box<ZeroError>>
         {

        tokens.consume_keyword("CREATE");
        tokens.consume_keyword("TEMPORARY");
        
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
                return  Err(ZeroError::ParseError{
                    message: format!("Expected token ) received token {:?}", tokens.peek()).into(),
                    code: "1064".into()
                }.into())
            }

            let table_options = self.parse_table_options(tokens)?;

            match tokens.peek() {
                None => Ok(ASTNode::MySQLCreateTable{
                    table: Box::new(table),
                    column_list: columns,
                    keys: keys,
                    table_options: table_options
                 }),
                _ => Err(ZeroError::ParseError{
                    message: format!("Expected end of statement, received {:?}", tokens.peek()).into(),
                    code: "1064".into()
                }.into())
            }

        } else {
            Err(ZeroError::ParseError{
                message: format!("Unexpected token after CREATE {:?}", tokens.peek()).into(),
                code: "1064".into()
            }.into())
        }

    }

    fn parse_table_options<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<Vec<ASTNode>,  Box<ZeroError>>
         {

        let mut ret: Vec<ASTNode> = Vec::new();

        while let Some(o) = self.parse_table_option(tokens)? {
            ret.push(o);
        }
        Ok(ret)
    }

    fn parse_table_option<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<Option<ASTNode>,  Box<ZeroError>>
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
                    tokens.consume_operator("=");
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
                _ =>  Err(ZeroError::ParseError{
                     message: format!("Unsupported Table Option {}", v).into(),
                     code: "1064".into()
                 }.into())
            },
            _ => Ok(None)
        }
    }

    fn parse_key_def<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<ASTNode, Box<ZeroError>>
         {

        debug!("parse_key_def()");

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
                _ => Err(ZeroError::ParseError{
                     message: format!("Unsupported key definition prefix {}", v).into(),
                     code: "1064".into()
                 }.into())

            },
            _ =>  Err(ZeroError::ParseError{
                 message: format!("Expected key definition received token {:?}", t).into(),
                 code: "1064".into()
             }.into())

        }
    }

    fn parse_optional_key_name<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<Option<Box<ASTNode>>,  Box<ZeroError>>
         {

        match tokens.peek() {
            Some(&Token::Identifier(_)) => Ok(Some(Box::new(self.ansi.parse_identifier(tokens)?))),
            _ => Ok(None)
        }
    }

    fn parse_key_column_list<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<Vec<ASTNode>,  Box<ZeroError>>
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

    fn parse_column_def<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<ASTNode,  Box<ZeroError>>
         {

        let column = try!(self.ansi.parse_identifier(tokens));
        let data_type = try!(self.parse_data_type(tokens));
        let qualifiers = try!(self.parse_column_qualifiers(tokens));
        match tokens.peek() {
            Some(&Token::Punctuator(ref p)) => match &p as &str {
                "," | ")" => {},
                _ => return Err(ZeroError::ParseError{
                         message: format!("Unsupported token in column definition: {:?}", tokens.peek()).into(),
                         code: "1064".into()
                     }.into())


            },
            _ => return Err(ZeroError::ParseError{
                     message: format!("Unsupported token in column definition: {:?}", tokens.peek()).into(),
                     code: "1064".into()
                 }.into())
        }

        Ok(ASTNode::MySQLColumnDef{column: Box::new(column), data_type: Box::new(data_type), qualifiers: qualifiers})
    }

    pub fn parse_column_qualifiers<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) ->  Result<Option<Vec<ASTNode>>,  Box<ZeroError>>
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

    pub fn parse_column_qualifier<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) ->  Result<Option<ASTNode>,  Box<ZeroError>>
         {

        debug!("parse_column_qualifier() {:?}", tokens.peek());
        match tokens.peek() {
            Some(&Token::Keyword(ref v)) | Some(&Token::Identifier(ref v)) => match &v.to_uppercase() as &str {
                "NOT" => {
                    tokens.next();
                    if tokens.consume_literal_null() {
                        Ok(Some(ASTNode::MySQLColumnQualifier(MySQLColumnQualifier::NotNull)))
                    } else {
                        Err(ZeroError::ParseError{
                            message: format!("Expected NOT NULL, received NOT {:?}", tokens.peek()).into(),
                            code: "1064".into()
                        }.into())
                    }
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
                        Err(ZeroError::ParseError{
                            message: format!("Expected PRIMARY KEY, received PRIMARY {:?}", tokens.peek()).into(),
                            code: "1064".into()
                        }.into())
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
                        Err(ZeroError::ParseError{
                            message: format!("Expected PRIMARY KEY, received PRIMARY {:?}", tokens.peek()).into(),
                            code: "1064".into()
                        }.into())
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
                        Err(ZeroError::ParseError{
                            message: format!("Expected ON UPDATE, received ON {:?}", tokens.peek()).into(),
                            code: "1064".into()
                        }.into())
                    }
                },
                "COMMENT" => {
                    tokens.next();
                    Ok(Some(ASTNode::MySQLColumnQualifier(MySQLColumnQualifier::Comment(Box::new(try!(tokens.parse_expr(0)))))))
                }
                _ => Ok(None)
            },
            Some(&Token::Literal(_)) => {
                if tokens.consume_literal_null() {
                    Ok(Some(ASTNode::MySQLColumnQualifier(MySQLColumnQualifier::Null)))
                } else {
                    Ok(None)
                }
            },
            _ => Ok(None)
        }
    }

    pub fn parse_data_type<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) ->  Result<ASTNode,  Box<ZeroError>>
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
                        Err(ZeroError::ParseError{
                            message: format!("Expected NATIONAL CHAR|VARCHAR|CHARACTER [VARYING], received NATIONAL {:?}", tokens.peek()).into(),
                            code: "1064".into()
                        }.into())
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
                _ => Err(ZeroError::ParseError{
                     message: format!("Data type not recognized {}", t).into(),
                     code: "1064".into()
                 }.into())
            },
            _ => Err(ZeroError::ParseError{
                 message: format!("Expected data type, received token {:?}", tokens.peek()).into(),
                 code: "1064".into()
            }.into())

        }
    }

    fn parse_optional_display<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<Option<u32>,  Box<ZeroError>>
         {

        if tokens.consume_punctuator("(") {
            match tokens.peek() {
                Some(&Token::Literal(index)) => match tokens.get_literal(index) {
                    Some(&LiteralToken::LiteralLong(_, ref v)) => {
                        tokens.next();
                        let ret = Ok(Some(u32::from_str(&v).unwrap()));
                        tokens.consume_punctuator(")");
                        ret
                    },
                    t =>  Err(ZeroError::ParseError{
                        message: format!("Expected LiteralLong token, received {:?}", t).into(),
                        code: "1064".into()
                    }.into())
                },
                _ =>  Err(ZeroError::ParseError{
                    message: format!("Expected LiteralLong token, received {:?}", tokens.peek()).into(),
                    code: "1064".into()
                }.into())
            }
        } else {
            Ok(None)
        }

    }

    fn parse_optional_precision_and_scale<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<Option<(u32,Option<u32>)>,  Box<ZeroError>>
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

    fn parse_long<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<u32, Box<ZeroError>> {
        match tokens.peek() {
            Some(&Token::Literal(index)) => match tokens.get_literal(index) {
                Some(&LiteralToken::LiteralLong(_, ref v)) => {
                    tokens.next();
                    Ok(u32::from_str(&v).unwrap())
                },
                t =>  Err(ZeroError::ParseError{
                    message: format!("Expected LiteralLong token, received {:?}", t).into(),
                    code: "1064".into()
                }.into())
            },
            _ =>  Err(ZeroError::ParseError{
                message: format!("Expected LiteralLong token, received {:?}", tokens.peek()).into(),
                code: "1064".into()
            }.into())
        }
    }
}


pub struct MySQLWriter{}

impl ExprWriter for MySQLWriter {
    fn write(&self, writer: &Writer, builder: &mut String, node: &ASTNode) -> Result<bool,  Box<ZeroError>> {
        match node {
            &ASTNode::MySQLDropTable { temporary, if_exists, ref tables, restrict, cascade } => {
                builder.push_str("DROP ");
                if temporary {
                    builder.push_str("TEMPORARY ");
                }
                builder.push_str("TABLE ");
                if if_exists {
                    builder.push_str("IF EXISTS ");
                }
                let mut i = 0;
                for table in tables.iter() {
                    if i > 0 {
                        builder.push_str(", ");
                    }
                    i += 1;
                    writer._write(builder, table)?;
                }
                if restrict {
                    builder.push_str("RESTRICT");
                } else if cascade {
                    builder.push_str("CASCADE");
                }
            },
            &ASTNode::MySQLCreateTable{box ref table, ref column_list, ref keys, ref table_options} => {
                builder.push_str("CREATE TABLE");
                writer._write(builder, table)?;

                builder.push_str(&" (");
                let mut sep = "";
                for c in column_list {
                    builder.push_str(sep);
                    writer._write(builder, c)?;
                    sep = ", ";
                }

                for k in keys {
                    builder.push_str(sep);
                    writer._write(builder, k)?;
                    sep = ", ";
                }

                builder.push_str(&")");

                sep = " ";
                for o in table_options {
                    builder.push_str(sep);
                    writer._write(builder, o)?;
                }
            },
            &ASTNode::MySQLColumnDef{box ref column, box ref data_type, ref qualifiers} => {
                writer._write(builder, column)?;
                writer._write(builder, data_type)?;
                match qualifiers {
                    &Some(ref e) => {
                        for q in e.iter() {
                            writer._write(builder, q)?;
                        }
                    },
                    &None => {}
                }

            },
            &ASTNode::MySQLDataType(ref data_type) => {
                self._write_data_type(writer, builder, data_type)?;
            },
            &ASTNode::MySQLKeyDef(ref k) => {
                self._write_key_definition(writer, builder, k)?;
            },
            &ASTNode::MySQLTableOption(ref o) => {
                self._write_table_option(writer, builder, o)?;
            },
            &ASTNode::MySQLColumnQualifier(ref q) => {
                self._write_column_qualifier(writer, builder, q)?;
            },
            _ => return Ok(false)
        }

        Ok(true)
    }
}

impl MySQLWriter {

    fn _write_data_type(&self, writer: &Writer, builder: &mut String, data_type: &MySQLDataType) -> Result<(),  Box<ZeroError>> {
        match data_type {
            &MySQLDataType::Bit{ref display} => {
                builder.push_str(" BIT");
                self._write_optional_display(builder, display);
            },
            &MySQLDataType::TinyInt{ref display} => {
                builder.push_str(" TINYINT");
                self._write_optional_display(builder, display);
            },
            &MySQLDataType::SmallInt{ref display} => {
                builder.push_str(" SMALLINT");
                self._write_optional_display(builder, display);
            },
            &MySQLDataType::MediumInt{ref display} => {
                builder.push_str(" MEDIUMINT");
                self._write_optional_display(builder, display);
            },
            &MySQLDataType::Int{ref display} => {
                builder.push_str(" INTEGER");
                self._write_optional_display(builder, display);
            },
            &MySQLDataType::BigInt{ref display} => {
                builder.push_str(" BIGINT");
                self._write_optional_display(builder, display);
            },
            &MySQLDataType::Decimal{ref precision, ref scale} => {
                builder.push_str(" DECIMAL");
                self._write_optional_precision_and_scale(builder, precision, scale);
            },
            &MySQLDataType::Float{ref precision, ref scale} => {
                builder.push_str(" FLOAT");
                self._write_optional_precision_and_scale(builder, precision, scale);
            },
            &MySQLDataType::Double{ref precision, ref scale} => {
                builder.push_str(" DOUBLE");
                self._write_optional_precision_and_scale(builder, precision, scale);
            },
            &MySQLDataType::Bool => {
                builder.push_str(" BOOLEAN");
            },
            &MySQLDataType::Date => {
                builder.push_str(" DATE");
            },
            &MySQLDataType::DateTime{ref fsp} => {
                builder.push_str(" DATETIME");
                self._write_optional_display(builder, fsp);
            },
            &MySQLDataType::Timestamp{ref fsp} => {
                builder.push_str(" TIMESTAMP");
                self._write_optional_display(builder, fsp);
            },
            &MySQLDataType::Time{ref fsp} => {
                builder.push_str(" TIME");
                self._write_optional_display(builder, fsp);
            },
            &MySQLDataType::Year{ref display} => {
                builder.push_str(" YEAR");
                self._write_optional_display(builder, display);
            },
            &MySQLDataType::Char{ref length} => {
                builder.push_str(" CHAR");
                self._write_optional_display(builder, length);
            },
            &MySQLDataType::NChar{ref length} => {
                builder.push_str(" NCHAR");
                self._write_optional_display(builder, length);
            },
            &MySQLDataType::CharByte{ref length} => {
                builder.push_str(" CHAR");
                self._write_optional_display(builder, length);
                builder.push_str(" BYTE");
            },
            &MySQLDataType::Varchar{ref length} => {
                builder.push_str(" VARCHAR");
                self._write_optional_display(builder, length);
            },
            &MySQLDataType::NVarchar{ref length} => {
                builder.push_str(" NVARCHAR");
                self._write_optional_display(builder, length);
            },
            &MySQLDataType::Binary{ref length} => {
                builder.push_str(" BINARY");
                self._write_optional_display(builder, length);
            },
            &MySQLDataType::VarBinary{ref length} => {
                builder.push_str(" VARBINARY");
                self._write_optional_display(builder, length);
            },
            &MySQLDataType::Blob{ref length} => {
                builder.push_str(" BLOB");
                self._write_optional_display(builder, length);
            },
            &MySQLDataType::Text{ref length} => {
                builder.push_str(" TEXT");
                self._write_optional_display(builder, length);
            },
            &MySQLDataType::TinyBlob => {
                builder.push_str(" TINYBLOB");
            },
            &MySQLDataType::TinyText => {
                builder.push_str(" TINYTEXT");
            },
            &MySQLDataType::MediumBlob => {
                builder.push_str(" MEDIUMBLOB");
            },
            &MySQLDataType::MediumText => {
                builder.push_str(" MEDIUMTEXT");
            },
            &MySQLDataType::LongBlob => {
                builder.push_str(" LONGBLOB");
            },
            &MySQLDataType::LongText => {
                builder.push_str(" LONGTEXT");
            },
            &MySQLDataType::Enum{box ref values} => {
                builder.push_str(" ENUM(");
                writer._write(builder, values)?;
                builder.push_str(")");
            },
            &MySQLDataType::Set{box ref values} => {
                builder.push_str(" SET(");
                writer._write(builder, values)?;
                builder.push_str(")");
            },
            // _ => panic!("Unsupported data type {:?}", data_type)

        }

        Ok(())
    }

    fn _write_key_definition(&self, writer: &Writer, builder:  &mut String, key: &MySQLKeyDef) -> Result<(),  Box<ZeroError>> {
        match key {
            &MySQLKeyDef::Primary{ref symbol, ref name, ref columns} => {

                match symbol {
                    &Some(box ref e) => {
                        builder.push_str(&" CONSTRAINT");
                        writer._write(builder, e)?;
                    },
                    &None => {}
                }

                builder.push_str(&" PRIMARY KEY");
                match name {
                    &Some(box ref e) => {
                        writer._write(builder, e)?;
                    },
                    &None => {}
                }
                self._write_key_column_list(writer, builder, columns)?;
            },
            &MySQLKeyDef::Unique{ref symbol, ref name, ref columns} => {
                match symbol {
                    &Some(box ref e) => {
                        builder.push_str(&" CONSTRAINT");
                        writer._write(builder, e)?;
                    },
                    &None => {}
                }

                builder.push_str(&" UNIQUE KEY");
                match name {
                    &Some(box ref e) => {
                        writer._write(builder, e)?;
                    },
                    &None => {}
                }
                self._write_key_column_list(writer, builder, columns)?;
            },
            &MySQLKeyDef::FullText{ref name, ref columns} => {
                builder.push_str(&" FULLTEXT KEY");
                match name {
                    &Some(box ref e) => {
                        writer._write(builder, e)?;
                    },
                    &None => {}
                }
                self._write_key_column_list(writer, builder, columns)?;
            },
            &MySQLKeyDef::Index{ref name, ref columns} => {
                builder.push_str(&" KEY");
                match name {
                    &Some(box ref e) => {
                        writer._write(builder, e)?;
                    },
                    &None => {}
                }
                self._write_key_column_list(writer, builder, columns)?;
            },
            &MySQLKeyDef::Foreign{ref symbol, ref name, ref columns, box ref reference_table, ref reference_columns} => {
                match symbol {
                    &Some(box ref e) => {
                        builder.push_str(&" CONSTRAINT");
                        writer._write(builder, e)?;
                    },
                    &None => {}
                }

                builder.push_str(&" FOREIGN KEY");
                match name {
                    &Some(box ref e) => {
                        writer._write(builder, e)?;
                    },
                    &None => {}
                }
                self._write_key_column_list(writer, builder, columns)?;

                builder.push_str(&" REFERENCES");
                writer._write(builder, &*reference_table)?;
                self._write_key_column_list(writer, builder, reference_columns)?;
            }
        }

        Ok(())
    }

    fn _write_table_option(&self, writer: &Writer, builder:  &mut String, option: &MySQLTableOption) -> Result<(),  Box<ZeroError>> {
        match option {
            &MySQLTableOption::Comment(box ref e) => {
                builder.push_str(" COMMENT");
                writer._write(builder, e)?;
            },
            &MySQLTableOption::Charset(box ref e) => {
                builder.push_str(" DEFAULT CHARSET");
                writer._write(builder, e)?;
            },
            &MySQLTableOption::Engine(box ref e) => {
                builder.push_str(" ENGINE");
                writer._write(builder, e)?;
            },
            &MySQLTableOption::AutoIncrement(box ref e) => {
                builder.push_str(" AUTO_INCREMENT");
                writer._write(builder, e)?;
            }
        }

        Ok(())
    }

    fn _write_key_column_list(&self, writer: &Writer, builder: &mut String, list: &Vec<ASTNode>) -> Result<(),  Box<ZeroError>> {
        builder.push_str(&" (");
        let mut sep = "";
        for c in list {
            builder.push_str(sep);
            writer._write(builder, c)?;
            sep = ", ";
        }
        builder.push_str(&")");

        Ok(())
    }

    fn _write_column_qualifier(&self, writer: &Writer, builder:  &mut String, q: &MySQLColumnQualifier) -> Result<(),  Box<ZeroError>> {
        match q {
            &MySQLColumnQualifier::CharacterSet(box ref e) => {
                builder.push_str(&" CHARACTER SET");
                writer._write(builder, e)?;
            },
            &MySQLColumnQualifier::Collate(box ref e) => {
                builder.push_str(&" COLLATE");
                writer._write(builder, e)?;
            },
            &MySQLColumnQualifier::Default(box ref e) => {
                builder.push_str(&" DEFAULT");
                writer._write(builder, e)?;
            },
            &MySQLColumnQualifier::Signed => builder.push_str(&" SIGNED"),
            &MySQLColumnQualifier::Unsigned => builder.push_str(&" UNSIGNED"),
            &MySQLColumnQualifier::Null => builder.push_str(&" NULL"),
            &MySQLColumnQualifier::NotNull => builder.push_str(&" NOT NULL"),
            &MySQLColumnQualifier::AutoIncrement => builder.push_str(&" AUTO_INCREMENT"),
            &MySQLColumnQualifier::PrimaryKey => builder.push_str(&" PRIMARY KEY"),
            &MySQLColumnQualifier::UniqueKey => builder.push_str(&" UNIQUE"),
            &MySQLColumnQualifier::OnUpdate(box ref e) => {
                builder.push_str(&" ON UPDATE");
                writer._write(builder, e)?;
            },
            &MySQLColumnQualifier::Comment(box ref e) => {
                builder.push_str(&" COMMENT");
                writer._write(builder, e)?;
            }
        }

        Ok(())
    }

    fn _write_optional_display(&self, builder: &mut String, display: &Option<u32>) {
        match display {
            &Some(ref d) => {write!(builder, "({})", d).unwrap();},
            &None => {}
        }
    }

    fn _write_optional_precision_and_scale(&self, builder: &mut String, precision: &Option<u32>, scale: &Option<u32>) {
        match precision {
            &Some(ref p) => {
                write!(builder, "({}", p).unwrap();
                if scale.is_some() {
                    write!(builder, ",{}", scale.unwrap()).unwrap();
                }
                builder.push_str(")");
            },
            &None => {}
        }
        ()
    }
}
