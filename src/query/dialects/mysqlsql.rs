use super::super::*;

use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug,PartialEq,Clone)]
pub enum MySQLToken {
	MySQLIdentifier(String) // encapsulated with ``
}
impl IToken for MySQLToken {}

#[derive(Debug,PartialEq,Clone)]
pub enum MySQLAST {
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
impl IAST for MySQLAST{}

#[derive(Debug,PartialEq,Clone)]
pub enum MySQLRel {}
impl IRel for MySQLRel {}

struct MySQLSQLDialect{}

impl Dialect<MySQLToken, MySQLAST, MySQLRel> for MySQLSQLDialect {

	fn get_token(&self, chars: &mut Peekable<Chars>) -> Result<Option<Token<MySQLToken>>, String> {
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

					Ok(Some(Token::TokenExtension(MySQLToken::MySQLIdentifier(text))))
				},
				_ => Ok(None)
			},
			_ => Ok(None)
		}
	}

	fn parse_prefix<'a, D: Dialect<MySQLToken, MySQLAST, MySQLRel>>
        (&self, tokens: &Tokens<'a, D, MySQLToken, MySQLAST, MySQLRel>) ->
            Result<Option<ASTNode<MySQLAST>>, String> {

        match tokens.peek() {
			Some(&Token::Identifier(ref v))
				| Some(&Token::TokenExtension(MySQLToken::MySQLIdentifier(ref v)))
				| Some(&Token::Keyword(ref v)) => match &v as &str {

				"CREATE" => Err(String::from("CREATE not supported")),
				_ => Ok(None)
			},
			_ => Ok(None)
		}
    }

    fn get_precedence<'a, D:  Dialect<MySQLToken, MySQLAST, MySQLRel>>
        (&self, tokens: &Tokens<'a, D, MySQLToken, MySQLAST, MySQLRel>)
            -> Result<u8, String> {
        Err(String::from("get_precedence() not implemented"))
    }

    fn parse_infix<'a, D: Dialect<MySQLToken, MySQLAST, MySQLRel>>
        (&self, tokens: &Tokens<'a, D, MySQLToken, MySQLAST, MySQLRel>, left: ASTNode<MySQLAST>, precedence: u8)
            -> Result<Option<ASTNode<MySQLAST>>, String> {
        Err(String::from("parse_infix() not implemented"))
    }

}
