use super::super::*;
use super::super::dialects::ansisql::*;

#[test]
fn simple_tokenize() {
	let dialect = AnsiSQLDialect::new();
	let tokens = String::from("SELECT 1 + 1").tokenize(&dialect).unwrap();
	assert_eq!(
		vec![Token::Keyword("SELECT".to_string()),
			Token::Literal(0),
			Token::Operator("+".to_string()),
			Token::Literal(1)
		],
		tokens.tokens
	);

	// let parsed = tokens.parse().unwrap();
}

#[test]
fn tokenize_with_null() {
	let dialect = AnsiSQLDialect::new();
	let tokens = String::from("SELECT NULL, null").tokenize(&dialect).unwrap();
	assert_eq!(
		vec![Token::Keyword("SELECT".to_string()),
			Token::Literal(0),
			Token::Punctuator(",".to_string()),
			Token::Literal(1)
		],
		tokens.tokens
	);

	// let parsed = tokens.parse().unwrap();
}

#[test]
fn complex_tokenize() {
	let dialect = AnsiSQLDialect::new();
    assert_eq!(
        vec![Token::Keyword("SELECT".to_string()),
            Token::Identifier("a".to_string()),
            Token::Punctuator(",".to_string()),
            Token::Literal(0),
            Token::Keyword("FROM".to_string()),
            Token::Identifier("tOne".to_string()),
            Token::Keyword("WHERE".to_string()),
            Token::Identifier("b".to_string()),
            Token::Operator(">".to_string()),
            Token::Literal(1),
            Token::Operator("AND".to_string()),
            Token::Identifier("c".to_string()),
            Token::Operator("!=".to_string()),
            Token::Literal(2)
		],
        String::from("SELECT a, 'hello' FROM tOne WHERE b > 2.22 AND c != true").tokenize(&dialect).unwrap().tokens
    );

}
