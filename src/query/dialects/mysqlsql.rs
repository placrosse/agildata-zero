use super::super::*;

use std::iter::Peekable;
use std::str::Chars;

static KEYWORDS: &'static [&'static str] = &["SHOW", "CREATE", "TABLE", "PRECISION",
	"PRIMARY", "KEY", "UNIQUE", "FULLTEXT", "FOREIGN", "REFERENCES", "CONSTRAINT"];

pub struct MySQLDialect{}

impl Dialect for MySQLDialect {

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
				_ => Ok(None)
			},
			_ => Ok(None)
		}
	}

	fn parse_prefix<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) ->
            Result<Option<ASTNode>, String> {

        match tokens.peek() {
			Some(&Token::Identifier(ref v))
				| Some(&Token::Keyword(ref v)) => match &v as &str {

				"CREATE" => Err(String::from("CREATE not supported")),
				_ => Ok(None)
			},
			_ => Ok(None)
		}
    }

    fn get_precedence<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>)-> Result<u8, String> {
        Err(String::from("get_precedence() not implemented"))
    }

    fn parse_infix<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>, left: ASTNode, precedence: u8)-> Result<Option<ASTNode>, String> {
        Err(String::from("parse_infix() not implemented"))
    }

}

impl MySQLDialect {
	pub fn new() -> Self {MySQLDialect{}}
}
