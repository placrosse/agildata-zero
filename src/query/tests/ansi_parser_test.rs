use super::super::ASTNode::*;
use super::super::Operator::*;
use super::super::LiteralExpr::*;
use super::super::JoinType::*;
use super::super::UnionType::*;
use super::super::{Tokenizer, Parser, SQLWriter, Writer};
use super::super::dialects::ansisql::*;
use super::test_helper::*;
use std::error::Error;

#[test]
fn select_wildcard() {
	let dialect = AnsiSQLDialect::new();
	let sql = String::from("SELECT * FROM foo");
	let tokens = sql.tokenize(&dialect).unwrap();
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

	let ansi_writer = AnsiSQLWriter{};
	let writer = SQLWriter::new(vec![&ansi_writer]);
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {:?}", rewritten);

}

#[test]
fn sqlparser() {
	let dialect = AnsiSQLDialect::new();
	let sql = String::from("SELECT 1 + 1 + 1,
		a AS alias,
		(3 * (1 + 2)),
		-1 AS unary,
		(SELECT a, b, c FROM tTwo WHERE c = a) AS subselect
		FROM (SELECT a, b, c FROM tThree) AS l
		WHERE a > 10 AND b = true
		ORDER BY a DESC, (a + b) ASC, c");
	let tokens = sql.tokenize(&dialect).unwrap();
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
	let ansi_writer = AnsiSQLWriter{};
	let writer = SQLWriter::new(vec![&ansi_writer]);
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {:?}", rewritten);

}

#[test]
fn sql_join() {

	let dialect = AnsiSQLDialect::new();
	let sql = String::from("SELECT l.a, r.b, l.c FROM tOne AS l
		JOIN (SELECT a, b, c FROM tTwo WHERE a > 0) AS r
		ON l.a = r.a
		WHERE l.b > r.b
		ORDER BY r.c DESC");

	let tokens = sql.tokenize(&dialect).unwrap();
	let parsed = tokens.parse().unwrap();

	assert_eq!(
		SQLSelect {
			expr_list: Box::new(SQLExprList(
				vec![
					SQLIdentifier{id: String::from("l.a"), parts: vec![String::from("l"), String::from("a")]},
					SQLIdentifier{id: String::from("r.b"), parts: vec![String::from("r"), String::from("b")]},
					SQLIdentifier{id: String::from("l.c"), parts: vec![String::from("l"), String::from("c")]}
				]
			)),
			relation: Some(Box::new(SQLJoin {
				left: Box::new(
					SQLAlias {
						expr: Box::new(SQLIdentifier{id: String::from("tOne"), parts: vec![String::from("tOne")]}),
						alias: Box::new(SQLIdentifier{id: String::from("l"), parts: vec![String::from("l")]})
					}
				),
				join_type: INNER,
				right: Box::new(
					SQLAlias {
						expr: Box::new(SQLNested(
							Box::new(SQLSelect{
								expr_list: Box::new(SQLExprList(
									vec![
									SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]},
									SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]},
									SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}
									]
								)),
								relation: Some(Box::new(SQLIdentifier{id: String::from("tTwo"), parts: vec![String::from("tTwo")]})),
								selection: Some(Box::new(SQLBinary{
									left: Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
									op: GT,
									right: Box::new(SQLLiteral(LiteralLong(0, 0_u64)))
								})),
								order: None
							})
						)),
						alias: Box::new(SQLIdentifier{id: String::from("r"), parts: vec![String::from("r")]})
					}
				),
				on_expr: Some(Box::new(SQLBinary {
					left: Box::new(SQLIdentifier{id: String::from("l.a"), parts: vec![String::from("l"), String::from("a")]}),
					op: EQ,
					right: Box::new(SQLIdentifier{id: String::from("r.a"), parts: vec![String::from("r"), String::from("a")]})
				}))
			})),
			selection: Some(Box::new(SQLBinary{
				left: Box::new(SQLIdentifier{id: String::from("l.b"), parts: vec![String::from("l"), String::from("b")]}),
				op: GT,
				right: Box::new(SQLIdentifier{id: String::from("r.b"), parts: vec![String::from("r"), String::from("b")]})
			})),
			order: Some(Box::new(SQLExprList(
				vec![
					SQLOrderBy{
						expr: Box::new(SQLIdentifier{id: String::from("r.c"), parts: vec![String::from("r"), String::from("c")]}),
						is_asc: false
					}
				]
			)))
		},
		parsed
	);

	println!("{:#?}", parsed);

	let ansi_writer = AnsiSQLWriter{};
	let writer = SQLWriter::new(vec![&ansi_writer]);
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {:?}", rewritten);
}

#[test]
fn nasty() {

	let dialect = AnsiSQLDialect::new();
	let sql = String::from("((((SELECT a, b, c FROM tOne UNION (SELECT a, b, c FROM tTwo))))) UNION (((SELECT a, b, c FROM tThree) UNION ((SELECT a, b, c FROM tFour))))");
	let tokens = sql.tokenize(&dialect).unwrap();
	let parsed = tokens.parse().unwrap();

	assert_eq!(
		SQLUnion{
			left: Box::new(SQLNested(
				Box::new(SQLNested(
					Box::new(SQLNested(
						Box::new(SQLNested(
							Box::new(SQLUnion{
								left: Box::new(SQLSelect{
									expr_list: Box::new(SQLExprList(vec![
										SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]},
										SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]},
										SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}
									])),
									relation: Some(Box::new(SQLIdentifier{id: String::from("tOne"), parts: vec![String::from("tOne")]})),
									selection: None,
									order: None
								}),
								union_type: UNION,
								right: Box::new(SQLNested(
									Box::new(SQLSelect{
										expr_list: Box::new(SQLExprList(vec![
											SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]},
											SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]},
											SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}
										])),
										relation: Some(Box::new(SQLIdentifier{id: String::from("tTwo"), parts: vec![String::from("tTwo")]})),
										selection: None,
										order: None
									})
								))
							})
						))
					))
				))
			)),
			union_type: UNION,
			right: Box::new(SQLNested(
				Box::new(SQLNested(
					Box::new(SQLUnion{
						left: Box::new(SQLNested(
							Box::new(SQLSelect{
								expr_list: Box::new(SQLExprList(vec![
									SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]},
									SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]},
									SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}
								])),
								relation: Some(Box::new(SQLIdentifier{id: String::from("tThree"), parts: vec![String::from("tThree")]})),
								selection: None,
								order: None
							})
						)),
						union_type: UNION,
						right: Box::new(SQLNested(
							Box::new(SQLNested(
								Box::new(SQLSelect{
									expr_list: Box::new(SQLExprList(vec![
										SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]},
										SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]},
										SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}
									])),
									relation: Some(Box::new(SQLIdentifier{id: String::from("tFour"), parts: vec![String::from("tFour")]})),
									selection: None,
									order: None
								})
							))
						))
					})
				))
			))
		},
		parsed
	);

	println!("{:#?}", parsed);

	let ansi_writer = AnsiSQLWriter{};
	let writer = SQLWriter::new(vec![&ansi_writer]);
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {:?}", rewritten);
}

#[test]
fn insert() {

	let dialect = AnsiSQLDialect::new();
	let sql = String::from("INSERT INTO foo (a, b, c) VALUES(1, 20.45, 'abcdefghijk')");
	let tokens = sql.tokenize(&dialect).unwrap();
	let parsed = tokens.parse().unwrap();

	assert_eq!(
		SQLInsert{
			table: Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]}),
			column_list: Box::new(SQLExprList(
				vec![
					SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]},
					SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]},
					SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}
				]
			)),
			values_list: Box::new(SQLExprList(
				vec![
					SQLLiteral(LiteralLong(0, 1_u64)),
					SQLLiteral(LiteralDouble(1, 20.45_f64)),
					SQLLiteral(LiteralString(2, String::from("abcdefghijk")))
				]
			))
		},
		parsed
	);

	println!("{:#?}", parsed);

	let ansi_writer = AnsiSQLWriter{};
	let writer = SQLWriter::new(vec![&ansi_writer]);
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {:?}", rewritten);
}

#[test]
fn insert_invalid() {

	let dialect = AnsiSQLDialect::new();
	let sql = String::from("INSERT INTO foo VALUES(1, 20.45, 'abcdefghijk')");
	let tokens = sql.tokenize(&dialect).unwrap();
	let parsed = tokens.parse();

	println!("{:#?}", parsed);

	assert!(parsed.is_err());
	assert_eq!(parsed.err().unwrap().to_string(), "Expected column list paren, received Some(Keyword(\"VALUES\"))");
}


#[test]
fn update() {

{
	let dialect = AnsiSQLDialect::new();
	let sql = String::from("UPDATE foo SET a = 'hello', b = 12345 WHERE c > 10");
	let tokens = sql.tokenize(&dialect).unwrap();
	let parsed = tokens.parse().unwrap();

	assert_eq!(
		SQLUpdate {
			table: Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]}),
			assignments: Box::new(SQLExprList(
				vec![
					SQLBinary{
						left: Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
						op: EQ,
						right: Box::new(SQLLiteral(LiteralString(0, String::from("hello"))))
					},
					SQLBinary{
						left: Box::new(SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]}),
						op: EQ,
						right: Box::new(SQLLiteral(LiteralLong(1, 12345_u64)))
					}
				]
			)),
			selection: Some(Box::new(SQLBinary{
				left: Box::new(SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}),
				op: GT,
				right : Box::new(SQLLiteral(LiteralLong(2, 10_u64)))
			}))
		},
		parsed
	);

	println!("{:#?}", parsed);

	let ansi_writer = AnsiSQLWriter{};
	let writer = SQLWriter::new(vec![&ansi_writer]);
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {:?}", rewritten);
}
{
	let dialect = AnsiSQLDialect::new();
	let sql = String::from("UPDATE warehouse SET w_ytd = w_ytd + 2117.1 WHERE w_id = 1");
	let tokens = sql.tokenize(&dialect).unwrap();
	let parsed = tokens.parse().unwrap();
	
	let upd = SQLUpdate{
		table: Box::new(SQLIdentifier{id: String::from("warehouse"), parts: vec![String::from("warehouse")] }),
			assignments: Box::new(
				SQLExprList(vec![
					SQLBinary {
						left: Box::new(SQLIdentifier{id: String::from("w_ytd"), parts: vec![String::from("w_ytd")]}),
						op:EQ,
						right: Box::new(
							SQLBinary {
								left: Box::new(SQLIdentifier{id: String::from("w_ytd"), 
										parts: vec![String::from("w_ytd")]}),
								op: ADD,
								right: Box::new(SQLLiteral(LiteralDouble(0, 2117.1_f64)))})}]
		)
	),
	selection: Some(Box::new(
			SQLBinary {
				left: Box::new(SQLIdentifier{id: String::from("w_id"), parts: vec![String::from("w_id")]}),
				op: EQ,
				right: Box::new(SQLLiteral(LiteralLong(1,1_u64)))
			})
	)
	};
	
	println!("{:#?}", parsed);
	assert_eq!(upd, parsed);

	let ansi_writer = AnsiSQLWriter{};
	let writer = SQLWriter::new(vec![&ansi_writer]);
	let rewritten = writer.write(&parsed).unwrap();

	assert_eq!(format_sql(&rewritten), format_sql(&sql));

    println!("Rewritten: {:?}", rewritten);
}

}


