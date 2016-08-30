use super::super::*;


use std::iter::Peekable;
use std::str::Chars;
use std::sync::atomic::{AtomicU32, Ordering};
use std::ascii::AsciiExt;

static KEYWORDS: &'static [&'static str] = &["SELECT", "FROM", "WHERE", "AND", "OR", "UNION", "FROM", "AS",
    "WHERE", "ORDER", "BY", "HAVING", "GROUP", "ASC", "DESC", "JOIN", "INNER", "LEFT", "RIGHT", "CROSS",
    "FULL", "ON", "INSERT", "UPDATE", "SET", "VALUES", "INTO", "SHOW", "CREATE", "TABLE", "PRECISION",
    "PRIMARY", "KEY", "UNIQUE", "FULLTEXT", "FOREIGN", "REFERENCES", "CONSTRAINT"];

pub struct AnsiSQLDialect {
	lit_index: AtomicU32
}

impl AnsiSQLDialect {
	pub fn new() -> Self {AnsiSQLDialect{lit_index: AtomicU32::new(0)}}
}

pub enum SQLAST {
    // SQLExprList(Vec<SQLAST>),
    // SQLBinary{left: Box<SQLAST>, op: SQLOperator, right: Box<SQLAST>},
    // SQLLiteral(LiteralExpr),
    // SQLIdentifier{id: String, parts: Vec<String>},
    // SQLAlias{expr: Box<SQLAST>, alias: Box<SQLAST>},
    // SQLNested(Box<SQLAST>),
    // SQLUnary{operator: SQLOperator, expr: Box<SQLAST>},
    // SQLOrderBy{expr: Box<SQLAST>, is_asc: bool},
    // SQLSelect{
    //     expr_list: Box<SQLAST>,
    //     relation: Option<Box<SQLAST>>,
    //     selection: Option<Box<SQLAST>>,
    //     order: Option<Box<SQLAST>>
    // },
    // SQLInsert {
    //     table: Box<SQLAST>,
    //     column_list: Box<SQLAST>,
    //     values_list: Box<SQLAST>
    // },
    // SQLUpdate {
    //     table: Box<SQLAST>,
    //     assignments: Box<SQLAST>,
    //     selection: Option<Box<SQLAST>>
    // },
    // SQLUnion{left: Box<SQLAST>, union_type: SQLUnionType, right: Box<SQLAST>},
    // SQLJoin{left: Box<SQLAST>, join_type: SQLJoinType, right: Box<SQLAST>, on_expr: Option<Box<SQLAST>>},
    // SQLCreateTable{
    //     table: Box<SQLAST>,
    //     column_list: Vec<SQLAST>,
    //     keys: Vec<SQLAST>,
    //     table_options: Vec<SQLAST>
    // },
    // SQLColumnDef{column: Box<SQLAST>, data_type: Box<SQLAST>, qualifiers: Option<Vec<SQLAST>>},
    // SQLKeyDef(KeyDef),
    // SQLColumnQualifier(ColumnQualifier),
    // SQLDataType(DataType),
    // SQLTableOption(TableOption)
}

impl Dialect for AnsiSQLDialect {

	fn get_token(&self, chars: &mut Peekable<Chars>) -> Result<Option<Token>, String> {
		match chars.peek() {
	        Some(&ch) => match ch {
	            ' ' | '\t' | '\n' => {
	                chars.next(); // consumer the char
	                Ok(Some(Token::Whitespace))
	            },
	            '+' | '-' | '/' | '*' | '%' | '=' => {
	                chars.next(); // consume one
	                Ok(Some(Token::Operator(ch.to_string()))) // after consume because return val
	            },
	            '>' | '<' | '!' => {

	                let mut op = chars.next().unwrap().to_string();

	                match chars.peek() {
	                    Some(&c) => match c {
	                        '=' => {
	                            op.push(c);
	                            chars.next(); // consume one
	                        }
	                        _ => {}
	                    },
	                    None => return Err(String::from("Expected token received None"))
	                }
	                Ok(Some(Token::Operator(op)))
	            },
	            '0'...'9' | '.' => {
	                let mut text = String::new();
	                while let Some(&c) = chars.peek() { // will break when it.peek() => None

	                    if c.is_numeric() || '.' == c  {
	                        text.push(c);
	                    } else {
	                        break; // leave the loop early
	                    }

	                    chars.next(); // consume one
	                }

	                if text.as_str().contains('.') {
						Ok(Some(Token::Literal(LiteralToken::LiteralDouble(self.lit_index.fetch_add(1, Ordering::SeqCst), text))))
	                } else {
						Ok(Some(Token::Literal(LiteralToken::LiteralLong(self.lit_index.fetch_add(1, Ordering::SeqCst), text))))
	                }
	            },
	            'a'...'z' | 'A'...'Z' => { // TODO this should really be any valid char for an identifier..
	                let mut text = String::new();
	                while let Some(&c) = chars.peek() { // will break when it.peek() => None

	                    if c.is_alphabetic() || c.is_numeric() || c == '.' || c == '_' {
	                        text.push(c);
	                    } else {
	                        break; // leave the loop early
	                    }

	                    chars.next(); // consume one
	                }

	                if "true".eq_ignore_ascii_case(&text) || "false".eq_ignore_ascii_case(&text) {
	                    Ok(Some(Token::Literal(LiteralToken::LiteralBool(self.lit_index.fetch_add(1, Ordering::SeqCst), text))))
	                } else if KEYWORDS.iter().position(|&r| r.eq_ignore_ascii_case(&text)).is_none() {
	                    Ok(Some(Token::Identifier(text)))
	                } else if "AND".eq_ignore_ascii_case(&text) || "OR".eq_ignore_ascii_case(&text) {
	                    Ok(Some(Token::Operator(text)))
	                } else {
	                    Ok(Some(Token::Keyword(text.to_uppercase())))
	                }
	            },
	            '\'' => {
	                chars.next();
	                let mut s = String::new();
	                loop {
	                    match chars.peek() {
	                        Some(&c) => match c {
	                            '\\' => {
	                                s.push(c);
	                                chars.next();
	                                match chars.peek() {
	                                    Some(&n) => match n {
	                                        '\'' => {
	                                            s.push(n);
	                                            chars.next();
	                                        },
	                                        _ => continue,
	                                    },
	                                    None => return Err(String::from("Unexpected end of string"))
	                                }
	                            },
	                            '\'' => {
	                                chars.next();
	                                break;
	                            },
	                            _ => {
	                                s.push(c);
	                                chars.next();
	                            }
	                        },
	                        None => return Err(String::from("Unexpected end of string"))
	                    }
	                }

					Ok(Some(Token::Literal(LiteralToken::LiteralString(self.lit_index.fetch_add(1, Ordering::SeqCst), s))))
	            },
	            ',' | '(' | ')' => {
	                chars.next();
	                Ok(Some(Token::Punctuator(ch.to_string())))
	            },
	            _ => {
	                Err(format!("Unsupported char {:?}", ch))
	            }
	        },
	        None => Ok(None),
	    }
	}

    fn parse_prefix<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) ->
            Result<Option<ASTNode>, String> {

        Err(String::from("parse_prefix() not implemented"))
    }

    fn get_precedence<'a, D:  Dialect>
        (&self, tokens: &Tokens<'a, D>)
            -> Result<u8, String> {
        Err(String::from("get_precedence() not implemented"))
    }

    fn parse_infix<'a, D: Dialect>
        (&self, tokens: &Tokens<'a, D>, left: ASTNode, precedence: u8)
            -> Result<Option<ASTNode>, String> {
        Err(String::from("parse_infix() not implemented"))
    }

}
