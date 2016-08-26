use super::super::tokenizer::*;
use super::super::dialects::ansisql::*;

#[test]
fn simple_tokenize() {
	let dialects = vec![AnsiSQLDialect::new()];
	assert_eq!(
		vec![Token::Keyword("SELECT".to_string()),
			Token::TokenExtension(SQLToken::Literal(LiteralToken::LiteralLong(0, "1".to_string()))),
			Token::Operator("+".to_string()),
			Token::TokenExtension(SQLToken::Literal(LiteralToken::LiteralLong(1, "1".to_string())))
		],
		String::from("SELECT 1 + 1").tokenize(&dialects).unwrap()
	);
}
