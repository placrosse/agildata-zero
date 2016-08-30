use super::super::ASTNode::*;
use super::super::Operator::*;
use super::super::LiteralExpr::*;
use super::super::{Tokenizer, Parser};
use super::super::dialects::ansisql::*;

#[test]
fn select_wildcard() {
	let dialects = vec![AnsiSQLDialect::new()];
	let sql = String::from("SELECT * FROM foo");
	let tokens = sql.tokenize(&dialects).unwrap();
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

#[test]
fn sqlparser() {
	let dialects = vec![AnsiSQLDialect::new()];
	let sql = String::from("SELECT 1 + 1 + 1,
		a AS alias,
		(3 * (1 + 2)),
		-1 AS unary,
		(SELECT a, b, c FROM tTwo WHERE c = a) AS subselect
		FROM (SELECT a, b, c FROM tThree) AS l
		WHERE a > 10 AND b = true
		ORDER BY a DESC, (a + b) ASC, c");
	let tokens = sql.tokenize(&dialects).unwrap();
	let parsed = tokens.parse().unwrap();

	assert_eq!(
		SQLSelect {
			expr_list: Box::new(
				SQLExprList(vec![
					SQLBinary {
						left:  Box::new(SQLBinary{
							left: Box::new(SQLLiteral(LiteralLong(0, 1_u64))),
							op: ADD,
							right:  Box::new(SQLLiteral(LiteralLong(1, 1_u64)))
						}),
						op: ADD,
						right:  Box::new(SQLLiteral(LiteralLong(2, 1_u64)))
					},
					SQLAlias{
						expr:  Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
						alias:  Box::new(SQLIdentifier{id: String::from("alias"), parts: vec![String::from("alias")]})
					},
					SQLNested(
						 Box::new(SQLBinary {
							left:  Box::new(SQLLiteral(LiteralLong(3, 3_u64))),
							op: MULT,
							right:  Box::new(SQLNested(
								 Box::new(SQLBinary{
									left:  Box::new(SQLLiteral(LiteralLong(4, 1_u64))),
									op: ADD,
									right:  Box::new(SQLLiteral(LiteralLong(5, 2_u64)))
								})
							))
						})
					),
					SQLAlias{
						expr:  Box::new(SQLUnary{
							operator: SUB,
							expr:  Box::new(SQLLiteral(LiteralLong(6, 1_u64)))
						}),
						alias:  Box::new(SQLIdentifier{id: String::from("unary"), parts: vec![String::from("unary")]})
					},
					SQLAlias {
						expr:  Box::new(SQLNested(
							 Box::new(SQLSelect{
								expr_list:  Box::new(SQLExprList(
									vec![
										SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]},
										SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]},
										SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}
									]
								)),
								relation: Some( Box::new(SQLIdentifier{id: String::from("tTwo"), parts: vec![String::from("tTwo")]})),
								selection: Some( Box::new(SQLBinary{
									left:  Box::new(SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}),
									op: EQ,
									right:  Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]})
								})),
								order: None
							})
						)),
						alias:  Box::new(SQLIdentifier{id: String::from("subselect"), parts: vec![String::from("subselect")]})
					}
					]
				)
			),
			relation: Some( Box::new(SQLAlias{
				expr:  Box::new(SQLNested(
					 Box::new(SQLSelect {
						expr_list:  Box::new(SQLExprList(
							vec![
								SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]},
								SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]},
								SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}
							]
						)),
						relation: Some( Box::new(SQLIdentifier{id: String::from("tThree"), parts: vec![String::from("tThree")]})),
						selection: None,
						order: None
					})
				)),
				alias:  Box::new(SQLIdentifier{id: String::from("l"), parts: vec![String::from("l")]})
			})),
			selection: Some( Box::new(SQLBinary {
				left:  Box::new(SQLBinary{
					left:  Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
					op: GT,
					right:  Box::new(SQLLiteral(LiteralLong(7, 10_u64)))
				}),
				op: AND,
				right:  Box::new(SQLBinary{
					left:  Box::new(SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]}),
					op: EQ,
					right:  Box::new(SQLLiteral(LiteralBool(8, true)))
				})
			})),
			order: Some( Box::new(SQLExprList(
				vec![
					SQLOrderBy{
						expr:  Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
						is_asc: false
					},
					SQLOrderBy{
						expr:  Box::new(SQLNested(
							 Box::new(SQLBinary{
								left:  Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
								op: ADD,
								right:  Box::new(SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]})
							})
						)),
						is_asc: true
					},
					SQLOrderBy{
						expr:  Box::new(SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}),
						is_asc: true
					},
				]
			)))
		},
		parsed
	);

	println!("{:#?}", parsed);
	//
	// let writer = SQLWriter::default();
	// let rewritten = writer.write(&parsed).unwrap();
	// assert_eq!(format_sql(&rewritten), format_sql(&sql));
	//
	// println!("Rewritten: {:?}", rewritten);

}
