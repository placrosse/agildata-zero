use super::super::ASTNode::*;
use super::super::{Tokenizer, Parser};
use super::super::dialects::ansisql::*;

#[test]
fn select_wildcard() {
	let dialects = vec![AnsiSQLDialect::new()];
	let sql = String::from("SELECT * FROM foo");
	let tokens = sql.tokenize(&dialects).unwrap();

	println!("TOKENS {:?}", tokens.tokens);
	let parsed = tokens.parse().unwrap();

	assert_eq!(
		SQLSelect {
			expr_list: Box::new(SQLExprList(vec![SQLIdentifier{id: String::from("*"), parts: vec![String::from("*")]}])),
			relation: Some(Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]})),
			selection: None,
			order: None
		},
		parsed
	);

	println!("{:#?}", parsed);

	// let writer = SQLWriter::default();
	// let rewritten = writer.write(&parsed).unwrap();
	// assert_eq!(format_sql(&rewritten), format_sql(&sql));
	//
	// println!("Rewritten: {:?}", rewritten);

}
