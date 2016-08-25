use super::sql_parser::{AnsiSQLParser};
use super::sql_parser::SQLExpr::*;
use super::sql_parser::LiteralExpr::*;
use super::sql_parser::SQLOperator::*;
use super::sql_parser::SQLJoinType::*;
use super::sql_parser::SQLUnionType::*;
use super::sql_parser::DataType::*;
use super::sql_parser::ColumnQualifier::*;
use super::sql_parser::KeyDef::*;
use super::sql_parser::TableOption;
use super::sql_writer::*;
use std::collections::HashMap;

#[test]
fn sqlparser() {
	let parser = AnsiSQLParser {};
	let sql = "SELECT 1 + 1 + 1,
		a AS alias,
		(3 * (1 + 2)),
		-1 AS unary,
		(SELECT a, b, c FROM tTwo WHERE c = a) AS subselect
		FROM (SELECT a, b, c FROM tThree) AS l
		WHERE a > 10 AND b = true
		ORDER BY a DESC, (a + b) ASC, c";
	let parsed = parser.parse(sql).unwrap();

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

	println!("{:#?}", parser.parse(sql));

	let writer = SQLWriter::default();
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {:?}", rewritten);

}

#[test]
fn sql_join() {
	let parser = AnsiSQLParser {};
	let sql = "SELECT l.a, r.b, l.c FROM tOne AS l
		JOIN (SELECT a, b, c FROM tTwo WHERE a > 0) AS r
		ON l.a = r.a
		WHERE l.b > r.b
		ORDER BY r.c DESC";
	let parsed = parser.parse(sql).unwrap();

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

	println!("{:#?}", parser.parse(sql));

	let writer = SQLWriter::default();
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {:?}", rewritten);
}

#[test]
fn nasty() {
	let parser = AnsiSQLParser {};
	let sql = "((((SELECT a, b, c FROM tOne UNION (SELECT a, b, c FROM tTwo))))) UNION (((SELECT a, b, c FROM tThree) UNION ((SELECT a, b, c FROM tFour))))";

	let parsed = parser.parse(sql).unwrap();

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

	println!("{:#?}", parser.parse(sql));

	let writer = SQLWriter::default();
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {:?}", rewritten);
}

#[test]
fn insert() {
	let parser = AnsiSQLParser {};
	let sql = "INSERT INTO foo (a, b, c) VALUES(1, 20.45, 'abcdefghijk')";

	let parsed = parser.parse(sql).unwrap();

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

	println!("{:#?}", parser.parse(sql));

	let writer = SQLWriter::default();
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {:?}", rewritten);

}

#[test]
fn select_wildcard() {
	let parser = AnsiSQLParser {};
	let sql = "SELECT * FROM foo";

	let parsed = parser.parse(sql).unwrap();

	assert_eq!(
		SQLSelect {
			expr_list: Box::new(SQLExprList(vec![SQLIdentifier{id: String::from("*"), parts: vec![String::from("*")]}])),
			relation: Some(Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]})),
			selection: None,
			order: None
		},
		parsed
	);

	println!("{:#?}", parser.parse(sql));

	let writer = SQLWriter::default();
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {:?}", rewritten);

}

#[test]
fn update() {
	let parser = AnsiSQLParser {};
	let sql = "UPDATE foo SET a = 'hello', b = 12345 WHERE c > 10";

	let parsed = parser.parse(sql).unwrap();

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

	println!("{:#?}", parser.parse(sql));

	let writer = SQLWriter::default();
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {}", rewritten);

}

#[test]
fn create_numeric() {
	let parser = AnsiSQLParser {};
	let sql = "CREATE TABLE foo (
	      a BIT,
	      b BIT(2),
	      c TINYINT,
	      d TINYINT(10),
	      e BOOL,
	      f BOOLEAN,
	      g SMALLINT,
	      h SMALLINT(100),
	      i INT,
	      j INT(64),
	      k INTEGER,
	      l INTEGER(64),
	      m BIGINT,
	      n BIGINT(100),
	      o DECIMAL,
	      p DECIMAL(10),
	      q DECIMAL(10,2),
	      r DEC,
	      s DEC(10),
	      t DEC(10, 2),
	      u FLOAT,
	      v FLOAT(10),
	      w FLOAT(10,2),
	      x DOUBLE,
	      y DOUBLE(10),
	      z DOUBLE(10,2),
		  aa DOUBLE PRECISION,
		  ab DOUBLE PRECISION (10),
		  ac DOUBLE PRECISION (10, 2)
	      )";

	let parsed = parser.parse(sql).unwrap();

	assert_eq!(
		SQLCreateTable {
			table: Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]}),
			column_list: vec![
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
					data_type: Box::new(SQLDataType(Bit { display: None })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]}),
					data_type: Box::new(SQLDataType(Bit { display: Some(2) })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}),
					data_type: Box::new(SQLDataType(TinyInt { display: None })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("d"), parts: vec![String::from("d")]}),
					data_type: Box::new(SQLDataType(TinyInt { display: Some(10) })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("e"), parts: vec![String::from("e")]}),
					data_type: Box::new(SQLDataType(Bool)),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("f"), parts: vec![String::from("f")]}),
					data_type: Box::new(SQLDataType(Bool)),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("g"), parts: vec![String::from("g")]}),
					data_type: Box::new(SQLDataType(SmallInt { display: None })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("h"), parts: vec![String::from("h")]}),
					data_type: Box::new(SQLDataType(SmallInt { display: Some(100) })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("i"), parts: vec![String::from("i")]}),
					data_type: Box::new(SQLDataType(Int { display: None })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("j"), parts: vec![String::from("j")]}),
					data_type: Box::new(SQLDataType(Int { display: Some(64) })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("k"), parts: vec![String::from("k")]}),
					data_type: Box::new(SQLDataType(Int { display: None })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("l"), parts: vec![String::from("l")]}),
					data_type: Box::new(SQLDataType(Int { display: Some(64) })),
					qualifiers: None
				}, SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("m"), parts: vec![String::from("m")]}),
					data_type: Box::new(SQLDataType(BigInt { display: None })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("n"), parts: vec![String::from("n")]}),
					data_type: Box::new(SQLDataType(BigInt { display: Some(100) })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("o"), parts: vec![String::from("o")]}),
					data_type: Box::new(SQLDataType(Decimal { precision: None, scale: None })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("p"), parts: vec![String::from("p")]}),
					data_type: Box::new(SQLDataType(Decimal { precision: Some(10), scale: None })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("q"), parts: vec![String::from("q")]}),
					data_type: Box::new(SQLDataType(Decimal { precision: Some(10), scale: Some(2) })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("r"), parts: vec![String::from("r")]}),
					data_type: Box::new(SQLDataType(Decimal { precision: None, scale: None })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("s"), parts: vec![String::from("s")]}),
					data_type: Box::new(SQLDataType(Decimal { precision: Some(10), scale: None })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("t"), parts: vec![String::from("t")]}),
					data_type: Box::new(SQLDataType(Decimal { precision: Some(10), scale: Some(2) })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("u"), parts: vec![String::from("u")]}),
					data_type: Box::new(SQLDataType(Float { precision: None, scale: None })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("v"), parts: vec![String::from("v")]}),
					data_type: Box::new(SQLDataType(Float { precision: Some(10), scale: None })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("w"), parts: vec![String::from("w")]}),
					data_type: Box::new(SQLDataType(Float { precision: Some(10), scale: Some(2) })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("x"), parts: vec![String::from("x")]}),
					data_type: Box::new(SQLDataType(Double { precision: None, scale: None })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("y"), parts: vec![String::from("y")]}),
					data_type: Box::new(SQLDataType(Double { precision: Some(10), scale: None })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("z"), parts: vec![String::from("z")]}),
					data_type: Box::new(SQLDataType(Double { precision: Some(10), scale: Some(2) })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("aa"), parts: vec![String::from("aa")]}),
					data_type: Box::new(SQLDataType(Double { precision: None, scale: None })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("ab"), parts: vec![String::from("ab")]}),
					data_type: Box::new(SQLDataType(Double { precision: Some(10), scale: None })),
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("ac"), parts: vec![String::from("ac")]}),
					data_type: Box::new(SQLDataType(Double { precision: Some(10), scale: Some(2) })),
					qualifiers: None
				}
			],
			keys: vec![],
			table_options: vec![]
		},
		parsed
	);

	println!("{:#?}", parser.parse(sql));

	let writer = SQLWriter::default();
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {}", rewritten);

}

#[test]
fn create_temporal() {
	let parser = AnsiSQLParser {};

	let sql = "CREATE TABLE foo (
	      a DATE,
	      b DATETIME,
	      c DATETIME(6),
	      d TIMESTAMP,
	      e TIMESTAMP(6),
	      f TIME,
	      g TIME(6),
	      h YEAR,
	      i YEAR(4)
	  )";

	let parsed = parser.parse(sql).unwrap();

	assert_eq!(
		SQLCreateTable {
		    table: Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]}),
		    column_list: vec![
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
		            data_type: Box::new(SQLDataType(Date)),
					qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]}),
		            data_type: Box::new(SQLDataType(DateTime {fsp: None})),
					qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}),
		            data_type: Box::new(SQLDataType(DateTime {fsp: Some(6)})),
					qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("d"), parts: vec![String::from("d")]}),
		            data_type: Box::new(SQLDataType(Timestamp {fsp: None})),
					qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("e"), parts: vec![String::from("e")]}),
		            data_type: Box::new(SQLDataType(Timestamp {fsp: Some(6)})),
					qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("f"), parts: vec![String::from("f")]}),
		            data_type: Box::new(SQLDataType(Time {fsp: None})),
					qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("g"), parts: vec![String::from("g")]}),
		            data_type: Box::new(SQLDataType(Time {fsp: Some(6)})),
					qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("h"), parts: vec![String::from("h")]}),
		            data_type: Box::new(SQLDataType(Year {display: None})),
					qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("i"), parts: vec![String::from("i")]}),
		            data_type: Box::new(SQLDataType(Year {display: Some(4)})),
					qualifiers: None
		        }
		    ],
			keys: vec![],
			table_options: vec![]
		},
		parsed
	);

	println!("{:#?}", parsed);

	let writer = SQLWriter::default();
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {}", rewritten);
}

#[test]
fn create_character() {
	let parser = AnsiSQLParser {};

	let sql = "CREATE TABLE foo (
	      a NATIONAL CHAR,
	      b CHAR,
	      c CHAR(255),
	      d NCHAR,
	      e NCHAR(255),
	      f NATIONAL CHARACTER,
	      g CHARACTER,
	      h CHARACTER(255),
	      i NATIONAL VARCHAR(50),
	      j VARCHAR(50),
	      k NVARCHAR(50),
	      l CHARACTER VARYING(50),
	      m BINARY,
	      n BINARY(50),
	      o VARBINARY(50),
	      p TINYBLOB,
	      q TINYTEXT,
	      r BLOB,
	      s BLOB(50),
	      t TEXT,
	      u TEXT(100),
	      v MEDIUMBLOB,
	      w MEDIUMTEXT,
	      x LONGBLOB,
	      y LONGTEXT,
	      z ENUM('val1', 'val2', 'val3'),
	      aa SET('val1', 'val2', 'val3'),
	      ab CHAR BYTE,
	      ac CHAR(50) BYTE
	)";

	let parsed = parser.parse(sql).unwrap();

	assert_eq!(
		SQLCreateTable {
		    table: Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]}),
		    column_list: vec![
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
		            data_type: Box::new(SQLDataType(NChar {length: None})),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]}),
		            data_type: Box::new(SQLDataType(Char {length: None})),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}),
		            data_type: Box::new(SQLDataType(Char {length: Some(255)})),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("d"), parts: vec![String::from("d")]}),
		            data_type: Box::new(SQLDataType(NChar {length: None})),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("e"), parts: vec![String::from("e")]}),
		            data_type: Box::new(SQLDataType(NChar {length: Some(255)})),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("f"), parts: vec![String::from("f")]}),
		            data_type: Box::new(SQLDataType(NChar {length: None})),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("g"), parts: vec![String::from("g")]}),
		            data_type: Box::new(SQLDataType(Char {length: None})),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("h"), parts: vec![String::from("h")]}),
		            data_type: Box::new(SQLDataType(Char {length: Some(255)})),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("i"), parts: vec![String::from("i")]}),
		            data_type: Box::new(SQLDataType(NVarchar {length: Some(50)})),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("j"), parts: vec![String::from("j")]}),
		            data_type: Box::new(SQLDataType(Varchar {length: Some(50)})),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("k"), parts: vec![String::from("k")]}),
		            data_type: Box::new(SQLDataType(NVarchar {length: Some(50)})),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("l"), parts: vec![String::from("l")]}),
		            data_type: Box::new(SQLDataType(Varchar {length: Some(50)})),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("m"), parts: vec![String::from("m")]}),
		            data_type: Box::new(SQLDataType(Binary {length: None})),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("n"), parts: vec![String::from("n")]}),
		            data_type: Box::new(SQLDataType(Binary {length: Some(50)})),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("o"), parts: vec![String::from("o")]}),
		            data_type: Box::new(SQLDataType(VarBinary {length: Some(50)})),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("p"), parts: vec![String::from("p")]}),
		            data_type: Box::new(SQLDataType(TinyBlob)),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("q"), parts: vec![String::from("q")]}),
		            data_type: Box::new(SQLDataType(TinyText)),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("r"), parts: vec![String::from("r")]}),
		            data_type: Box::new(SQLDataType(Blob {length: None})),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("s"), parts: vec![String::from("s")]}),
		            data_type: Box::new(SQLDataType(Blob {length: Some(50)})),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("t"), parts: vec![String::from("t")]}),
		            data_type: Box::new(SQLDataType(Text {length: None})),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("u"), parts: vec![String::from("u")]}),
		            data_type: Box::new(SQLDataType(Text {length: Some(100)})),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("v"), parts: vec![String::from("v")]}),
		            data_type: Box::new(SQLDataType(MediumBlob)),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("w"), parts: vec![String::from("w")]}),
		            data_type: Box::new(SQLDataType(MediumText)),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("x"), parts: vec![String::from("x")]}),
		            data_type: Box::new(SQLDataType(LongBlob)),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("y"), parts: vec![String::from("y")]}),
		            data_type: Box::new(SQLDataType(LongText)),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("z"), parts: vec![String::from("z")]}),
		            data_type: Box::new(SQLDataType(Enum {values: Box::new(SQLExprList(vec![
		                        SQLLiteral(LiteralString(11,String::from("val1"))),
		                        SQLLiteral(LiteralString(12,String::from("val2"))),
		                        SQLLiteral(LiteralString(13,String::from("val3")))
		                    ]
		                ))
		            })),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("aa"), parts: vec![String::from("aa")]}),
		            data_type: Box::new(SQLDataType(Set {values: Box::new(SQLExprList(vec![
		                        SQLLiteral(LiteralString(14,String::from("val1"))),
		                        SQLLiteral(LiteralString(15,String::from("val2"))),
		                        SQLLiteral(LiteralString(16,String::from("val3")))
		                    ]
		                ))
		            })),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("ab"), parts: vec![String::from("ab")]}),
		            data_type: Box::new(SQLDataType(CharByte {length: None})),
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("ac"), parts: vec![String::from("ac")]}),
		            data_type: Box::new(SQLDataType(CharByte {length: Some(50)})),
		            qualifiers: None
		        }
		    ],
			keys: vec![],
			table_options: vec![]
		},
		parsed
	);

	println!("{:#?}", parsed);

	let writer = SQLWriter::default();
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {}", rewritten);
}

#[test]
fn create_column_qualifiers() {
	let parser = AnsiSQLParser {};

	let sql = "CREATE TABLE foo (
	      id BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY,
	      a VARCHAR(50) CHARACTER SET utf8 COLLATE utf8_general_ci NULL UNIQUE,
	      b BIGINT SIGNED NOT NULL DEFAULT 123456789,
	      c TINYINT UNSIGNED NULL DEFAULT NULL COMMENT 'Some Comment',
	      d TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
    )";

	let parsed = parser.parse(sql).unwrap();

	assert_eq!(
		SQLCreateTable {
		    table: Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]}),
		    column_list: vec![
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("id"), parts: vec![String::from("id")]}),
		            data_type: Box::new(SQLDataType(BigInt {display: None})),
		            qualifiers: Some(vec![
		                    SQLColumnQualifier(NotNull),
		                    SQLColumnQualifier(AutoIncrement),
		                    SQLColumnQualifier(PrimaryKey)
		                ]
		            )
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
		            data_type: Box::new(SQLDataType(Varchar {length: Some(50)})),
		            qualifiers: Some(vec![
		                    SQLColumnQualifier(CharacterSet(Box::new(SQLIdentifier{id: String::from("utf8"), parts: vec![String::from("utf8")]}))),
		                    SQLColumnQualifier(Collate(Box::new(SQLIdentifier{id: String::from("utf8_general_ci"), parts: vec![String::from("utf8_general_ci")]}))),
		                    SQLColumnQualifier(Null),
		                    SQLColumnQualifier(UniqueKey)
		                ]
		            )
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]}),
		            data_type: Box::new(SQLDataType(BigInt {display: None})),
		            qualifiers: Some(vec![
		                    SQLColumnQualifier(Signed),
		                    SQLColumnQualifier(NotNull),
		                    SQLColumnQualifier(Default(Box::new(SQLLiteral(LiteralLong(1,123456789)))))
		                ]
		            )
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}),
		            data_type: Box::new(SQLDataType(TinyInt {display: None})),
		            qualifiers: Some(vec![
		                    SQLColumnQualifier(Unsigned),
		                    SQLColumnQualifier(Null),
		                    SQLColumnQualifier(Default(Box::new(SQLIdentifier{id: String::from("NULL"), parts: vec![String::from("NULL")]}))), // TODO should be literal null ?
							SQLColumnQualifier(Comment(Box::new(SQLLiteral(LiteralString(2,String::from("Some Comment"))))))
		                ]
		            )
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("d"), parts: vec![String::from("d")]}),
		            data_type: Box::new(SQLDataType(Timestamp {fsp: None})),
		            qualifiers: Some(vec![
		                    SQLColumnQualifier(Default(Box::new(SQLIdentifier{id: String::from("CURRENT_TIMESTAMP"), parts: vec![String::from("CURRENT_TIMESTAMP")]}))),
		                    SQLColumnQualifier(OnUpdate(Box::new(SQLIdentifier{id: String::from("CURRENT_TIMESTAMP"), parts: vec![String::from("CURRENT_TIMESTAMP")]})))
		                ]
		            )
		        }
		    ],
			keys: vec![],
			table_options: vec![]
		},
		parsed
	);

	println!("{:#?}", parsed);

	let writer = SQLWriter::default();
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {}", rewritten);
}

#[test]
fn create_tail_keys() {
	let parser = AnsiSQLParser {};

	let sql = "CREATE TABLE foo (
	      id BIGINT AUTO_INCREMENT,
	      a VARCHAR(50) NOT NULL,
	      b TIMESTAMP NOT NULL,
	      PRIMARY KEY (id),
	      UNIQUE KEY keyName1 (id, b),
	      KEY keyName2 (b),
	      FULLTEXT KEY keyName (a),
	      FOREIGN KEY fkeyName (a) REFERENCES bar(id)
  	)";

	let parsed = parser.parse(sql).unwrap();

	assert_eq!(
		SQLCreateTable {
		    table: Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]}),
		    column_list: vec![
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("id"), parts: vec![String::from("id")]}),
		            data_type: Box::new(SQLDataType(BigInt {display: None})),
		            qualifiers: Some(vec![SQLColumnQualifier(AutoIncrement)])
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
		            data_type: Box::new(SQLDataType(Varchar {length: Some(50)})),
		            qualifiers: Some(vec![SQLColumnQualifier(NotNull)])
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]}),
		            data_type: Box::new(SQLDataType(Timestamp {fsp: None})),
		            qualifiers: Some(vec![SQLColumnQualifier(NotNull)])
		        }
		    ],
		    keys: vec![
		        SQLKeyDef(Primary {
					symbol: None,
		            name: None,
		            columns: vec![SQLIdentifier{id: String::from("id"), parts: vec![String::from("id")]}]
		        }),
		        SQLKeyDef(Unique {
					symbol: None,
		            name: Some(Box::new(SQLIdentifier{id: String::from("keyName1"), parts: vec![String::from("keyName1")]})),
		            columns: vec![
		                SQLIdentifier{id: String::from("id"), parts: vec![String::from("id")]},
		                SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]}
		            ]
		        }),
		        SQLKeyDef(Index {
		            name:  Some(Box::new(SQLIdentifier{id: String::from("keyName2"), parts: vec![String::from("keyName2")]})),
		            columns: vec![SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]}]
		        }),
		        SQLKeyDef(FullText {
		            name: Some(Box::new(SQLIdentifier{id: String::from("keyName"), parts: vec![String::from("keyName")]})),
		            columns: vec![SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}]
		        }),
		        SQLKeyDef(Foreign {
					symbol: None,
		            name: Some(Box::new(SQLIdentifier{id: String::from("fkeyName"), parts: vec![String::from("fkeyName")]})),
		            columns: vec![SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}],
		            reference_table: Box::new(SQLIdentifier{id: String::from("bar"), parts: vec![String::from("bar")]}),
		            reference_columns: vec![SQLIdentifier{id: String::from("id"), parts: vec![String::from("id")]}],
		        })
		    ],
			table_options: vec![]
		},
		parsed
	);

	println!("{:#?}", parsed);

	let writer = SQLWriter::default();
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {}", rewritten);
}

#[test]
fn create_tail_constraints() {
	let parser = AnsiSQLParser {};

	let sql = "CREATE TABLE foo (
	      id BIGINT AUTO_INCREMENT,
	      a VARCHAR(50) NOT NULL,
	      b TIMESTAMP NOT NULL,
	      CONSTRAINT symbol1 PRIMARY KEY (id),
	      CONSTRAINT symbol2 UNIQUE KEY keyName1 (a),
	      CONSTRAINT symbol3 FOREIGN KEY fkeyName (a) REFERENCES bar(id)
	)";

	let parsed = parser.parse(sql).unwrap();

	println!("{:#?}", parsed);

	assert_eq!(
		SQLCreateTable {
		    table: Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]}),
		    column_list: vec![
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("id"), parts: vec![String::from("id")]}),
		            data_type: Box::new(SQLDataType(BigInt {display: None})),
		            qualifiers: Some(vec![SQLColumnQualifier(AutoIncrement)])
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
		            data_type: Box::new(SQLDataType(Varchar {length: Some(50)})),
		            qualifiers: Some(vec![SQLColumnQualifier(NotNull)])
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]}),
		            data_type: Box::new(SQLDataType(Timestamp {fsp: None})),
		            qualifiers: Some(vec![SQLColumnQualifier(NotNull)])
		        }
		    ],
		    keys: vec![
				SQLKeyDef(Primary {
					symbol: Some(Box::new(SQLIdentifier{id: String::from("symbol1"), parts: vec![String::from("symbol1")]})),
					name: None,
					columns: vec![SQLIdentifier{id: String::from("id"), parts: vec![String::from("id")]}]
				}),
				SQLKeyDef(Unique {
					symbol: Some(Box::new(SQLIdentifier{id: String::from("symbol2"), parts: vec![String::from("symbol2")]})),
					name: Some(Box::new(SQLIdentifier{id: String::from("keyName1"), parts: vec![String::from("keyName1")]})),
					columns: vec![
						SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}
					]
				}),
				SQLKeyDef(Foreign {
					symbol: Some(Box::new(SQLIdentifier{id: String::from("symbol3"), parts: vec![String::from("symbol3")]})),
					name: Some(Box::new(SQLIdentifier{id: String::from("fkeyName"), parts: vec![String::from("fkeyName")]})),
					columns: vec![SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}],
					reference_table: Box::new(SQLIdentifier{id: String::from("bar"), parts: vec![String::from("bar")]}),
					reference_columns: vec![SQLIdentifier{id: String::from("id"), parts: vec![String::from("id")]}],
				})
			],
			table_options: vec![]
		},
		parsed
	);

	let writer = SQLWriter::default();
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {}", rewritten);
}

#[test]
fn create_table_options() {
	let parser = AnsiSQLParser {};

	let sql = "CREATE TABLE foo (
	      id BIGINT AUTO_INCREMENT,
	      a VARCHAR(50)
	) Engine InnoDB DEFAULT CHARSET utf8 COMMENT 'Table Comment' AUTO_INCREMENT 12345";

	let parsed = parser.parse(sql).unwrap();

	println!("{:#?}", parsed);

	assert_eq!(
		SQLCreateTable {
		    table: Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]}),
		    column_list: vec![
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("id"), parts: vec![String::from("id")]}),
		            data_type: Box::new(SQLDataType(BigInt {display: None})),
		            qualifiers: Some(vec![SQLColumnQualifier(AutoIncrement)])
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
		            data_type: Box::new(SQLDataType(Varchar {length: Some(50)})),
		            qualifiers: None
		        }
		    ],
		    keys: vec![],
			table_options: vec![
		        SQLTableOption(TableOption::Engine(Box::new(SQLIdentifier{id: String::from("InnoDB"), parts: vec![String::from("InnoDB")]}))),
		        SQLTableOption(TableOption::Charset(Box::new(SQLIdentifier{id: String::from("utf8"), parts: vec![String::from("utf8")]}))),
		        SQLTableOption(TableOption::Comment(Box::new(SQLLiteral(LiteralString(1,String::from("Table Comment")))))),
				SQLTableOption(TableOption::AutoIncrement(Box::new(SQLLiteral(LiteralLong(2,12345_u64)))))
		    ]
		},
		parsed
	);

	let writer = SQLWriter::default();
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {}", rewritten);
}

// used for fomatting sql strings for assertions
fn format_sql(sql: &str) -> String {

	sql.to_uppercase()

		// unify datatype synonymns
		.replace("BOOLEAN", "BOOL").replace("BOOL", "BOOLEAN") // done twice intentionally
		.replace("INTEGER", "INT").replace(" INT", " INTEGER")
		.replace("PRECISION", "")
		.replace("DECIMAL", "DEC").replace("DEC", "DECIMAL")
		.replace("CHARACTER VARYING", "VARCHAR")
		.replace("NATIONAL CHARACTER", "NCHAR")
		.replace("NATIONAL CHAR", "NCHAR")
		.replace("NATIONAL VARCHAR", "NVARCHAR")
		.replace("CHARACTER", "CHAR")

		// optional keywords
		.replace("ASC", "")
		.replace("INNER JOIN", "JOIN")

		// strip whitespace
		.replace(" ", "").replace("\n", "").replace("\r", "").replace("\t", "")


}
