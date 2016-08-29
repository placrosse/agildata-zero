use super::super::*;
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
		String::from("SELECT 1 + 1").tokenize(&dialects).unwrap().tokens
	);

	#[test]
    fn complex_tokenize() {
		let dialects = vec![AnsiSQLDialect::new()];
        assert_eq!(
            vec![Token::Keyword("SELECT".to_string()),
                Token::Identifier("a".to_string()),
                Token::Punctuator(",".to_string()),
                Token::TokenExtension(SQLToken::Literal(LiteralToken::LiteralString(0, "hello".to_string()))),
                Token::Keyword("FROM".to_string()),
                Token::Identifier("tOne".to_string()),
                Token::Keyword("WHERE".to_string()),
                Token::Identifier("b".to_string()),
                Token::Operator(">".to_string()),
                Token::TokenExtension(SQLToken::Literal(LiteralToken::LiteralDouble(1, "2.22".to_string()))),
                Token::Operator("AND".to_string()),
                Token::Identifier("c".to_string()),
                Token::Operator("!=".to_string()),
                Token::TokenExtension(SQLToken::Literal(LiteralToken::LiteralBool(2, "true".to_string())))
			],
            String::from("SELECT a, 'hello' FROM tOne WHERE b > 2.22 AND c != true").tokenize(&dialects).unwrap().tokens
        );
    }
}
