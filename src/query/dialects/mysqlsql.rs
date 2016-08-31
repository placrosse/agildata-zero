use super::super::*;
use super::ansisql::*;

use std::iter::Peekable;
use std::str::Chars;

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
				"CREATE" => Err(String::from("CREATE not supported")),
				_ => self.ansi.parse_prefix(tokens)
			},
			_ => self.ansi.parse_prefix(tokens)
		}
    }

    fn get_precedence<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>)-> Result<u8, String> {
        Err(String::from("get_precedence() not implemented"))
    }

    fn parse_infix<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>, left: ASTNode, precedence: u8)-> Result<Option<ASTNode>, String> {
        Err(String::from("parse_infix() not implemented"))
    }

}

impl<'d> MySQLDialect<'d> {
	pub fn new(ansi: &'d AnsiSQLDialect) -> Self {MySQLDialect{ansi: ansi}}
}
