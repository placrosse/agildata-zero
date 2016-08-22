#[cfg(test)]

use super::sql_parser::{AnsiSQLParser};
use super::sql_parser::SQLExpr::*;
use super::sql_parser::LiteralExpr::*;
use super::sql_parser::SQLOperator::*;
use super::sql_parser::SQLJoinType::*;
use super::sql_parser::SQLUnionType::*;
use super::sql_parser::DataType::*;
use super::sql_parser::ColumnQualifier::*;
use super::sql_writer;
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
						expr:  Box::new(SQLIdentifier(String::from("a"))),
						alias:  Box::new(SQLIdentifier(String::from("alias")))
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
						alias:  Box::new(SQLIdentifier(String::from("unary")))
					},
					SQLAlias {
						expr:  Box::new(SQLNested(
							 Box::new(SQLSelect{
								expr_list:  Box::new(SQLExprList(
									vec![
										SQLIdentifier(String::from("a")),
										SQLIdentifier(String::from("b")),
										SQLIdentifier(String::from("c"))
									]
								)),
								relation: Some( Box::new(SQLIdentifier(String::from("tTwo")))),
								selection: Some( Box::new(SQLBinary{
									left:  Box::new(SQLIdentifier(String::from("c"))),
									op: EQ,
									right:  Box::new(SQLIdentifier(String::from("a")))
								})),
								order: None
							})
						)),
						alias:  Box::new(SQLIdentifier(String::from("subselect")))
					}
					]
				)
			),
			relation: Some( Box::new(SQLAlias{
				expr:  Box::new(SQLNested(
					 Box::new(SQLSelect {
						expr_list:  Box::new(SQLExprList(
							vec![
								SQLIdentifier(String::from("a")),
								SQLIdentifier(String::from("b")),
								SQLIdentifier(String::from("c"))
							]
						)),
						relation: Some( Box::new(SQLIdentifier(String::from("tThree")))),
						selection: None,
						order: None
					})
				)),
				alias:  Box::new(SQLIdentifier(String::from("l")))
			})),
			selection: Some( Box::new(SQLBinary {
				left:  Box::new(SQLBinary{
					left:  Box::new(SQLIdentifier(String::from("a"))),
					op: GT,
					right:  Box::new(SQLLiteral(LiteralLong(7, 10_u64)))
				}),
				op: AND,
				right:  Box::new(SQLBinary{
					left:  Box::new(SQLIdentifier(String::from("b"))),
					op: EQ,
					right:  Box::new(SQLLiteral(LiteralBool(8, true)))
				})
			})),
			order: Some( Box::new(SQLExprList(
				vec![
					SQLOrderBy{
						expr:  Box::new(SQLIdentifier(String::from("a"))),
						is_asc: false
					},
					SQLOrderBy{
						expr:  Box::new(SQLNested(
							 Box::new(SQLBinary{
								left:  Box::new(SQLIdentifier(String::from("a"))),
								op: ADD,
								right:  Box::new(SQLIdentifier(String::from("b")))
							})
						)),
						is_asc: true
					},
					SQLOrderBy{
						expr:  Box::new(SQLIdentifier(String::from("c"))),
						is_asc: true
					},
				]
			)))
		},
		parsed
	);

	println!("{:#?}", parser.parse(sql));

	let rewritten = sql_writer::write(parsed, &HashMap::new());

	//assert_eq!(rewritten, sql);

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
					SQLIdentifier(String::from("l.a")),
					SQLIdentifier(String::from("r.b")),
					SQLIdentifier(String::from("l.c"))
				]
			)),
			relation: Some(Box::new(SQLJoin {
				left: Box::new(
					SQLAlias {
						expr: Box::new(SQLIdentifier(String::from("tOne"))),
						alias: Box::new(SQLIdentifier(String::from("l")))
					}
				),
				join_type: INNER,
				right: Box::new(
					SQLAlias {
						expr: Box::new(SQLNested(
							Box::new(SQLSelect{
								expr_list: Box::new(SQLExprList(
									vec![
									SQLIdentifier(String::from("a")),
									SQLIdentifier(String::from("b")),
									SQLIdentifier(String::from("c"))
									]
								)),
								relation: Some(Box::new(SQLIdentifier(String::from("tTwo")))),
								selection: Some(Box::new(SQLBinary{
									left: Box::new(SQLIdentifier(String::from("a"))),
									op: GT,
									right: Box::new(SQLLiteral(LiteralLong(0, 0_u64)))
								})),
								order: None
							})
						)),
						alias: Box::new(SQLIdentifier(String::from("r")))
					}
				),
				on_expr: Some(Box::new(SQLBinary {
					left: Box::new(SQLIdentifier(String::from("l.a"))),
					op: EQ,
					right: Box::new(SQLIdentifier(String::from("r.a")))
				}))
			})),
			selection: Some(Box::new(SQLBinary{
				left: Box::new(SQLIdentifier(String::from("l.b"))),
				op: GT,
				right: Box::new(SQLIdentifier(String::from("r.b")))
			})),
			order: Some(Box::new(SQLExprList(
				vec![
					SQLOrderBy{
						expr: Box::new(SQLIdentifier(String::from("r.c"))),
						is_asc: false
					}
				]
			)))
		},
		parsed
	);

	println!("{:#?}", parser.parse(sql));

	let rewritten = sql_writer::write(parsed, &HashMap::new());

	//assert_eq!(rewritten, sql);

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
										SQLIdentifier(String::from("a")),
										SQLIdentifier(String::from("b")),
										SQLIdentifier(String::from("c"))
									])),
									relation: Some(Box::new(SQLIdentifier(String::from("tOne")))),
									selection: None,
									order: None
								}),
								union_type: UNION,
								right: Box::new(SQLNested(
									Box::new(SQLSelect{
										expr_list: Box::new(SQLExprList(vec![
											SQLIdentifier(String::from("a")),
											SQLIdentifier(String::from("b")),
											SQLIdentifier(String::from("c"))
										])),
										relation: Some(Box::new(SQLIdentifier(String::from("tTwo")))),
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
									SQLIdentifier(String::from("a")),
									SQLIdentifier(String::from("b")),
									SQLIdentifier(String::from("c"))
								])),
								relation: Some(Box::new(SQLIdentifier(String::from("tThree")))),
								selection: None,
								order: None
							})
						)),
						union_type: UNION,
						right: Box::new(SQLNested(
							Box::new(SQLNested(
								Box::new(SQLSelect{
									expr_list: Box::new(SQLExprList(vec![
										SQLIdentifier(String::from("a")),
										SQLIdentifier(String::from("b")),
										SQLIdentifier(String::from("c"))
									])),
									relation: Some(Box::new(SQLIdentifier(String::from("tFour")))),
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

	let rewritten = sql_writer::write(parsed, &HashMap::new());

	println!("Rewritten: {:?}", rewritten);
}

#[test]
fn insert() {
	let parser = AnsiSQLParser {};
	let sql = "INSERT INTO foo (a, b, c) VALUES(1, 20.45, 'abcdefghijk')";

	let parsed = parser.parse(sql).unwrap();

	assert_eq!(
		SQLInsert{
			table: Box::new(SQLIdentifier(String::from("foo"))),
			column_list: Box::new(SQLExprList(
				vec![
					SQLIdentifier(String::from("a")),
					SQLIdentifier(String::from("b")),
					SQLIdentifier(String::from("c"))
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

	let rewritten = sql_writer::write(parsed, &HashMap::new());

	println!("Rewritten: {:?}", rewritten);

}

#[test]
fn select_wildcard() {
	let parser = AnsiSQLParser {};
	let sql = "SELECT * FROM foo";

	let parsed = parser.parse(sql).unwrap();

	assert_eq!(
		SQLSelect {
			expr_list: Box::new(SQLExprList(vec![SQLIdentifier(String::from("*"))])),
			relation: Some(Box::new(SQLIdentifier(String::from("foo")))),
			selection: None,
			order: None
		},
		parsed
	);

	println!("{:#?}", parser.parse(sql));

	let rewritten = sql_writer::write(parsed, &HashMap::new());

	assert_eq!(rewritten, sql);
	println!("Rewritten: {:?}", rewritten);

}

#[test]
fn update() {
	let parser = AnsiSQLParser {};
	let sql = "UPDATE foo SET a = 'hello', b = 12345 WHERE c > 10)";

	let parsed = parser.parse(sql).unwrap();

	assert_eq!(
		SQLUpdate {
			table: Box::new(SQLIdentifier(String::from("foo"))),
			assignments: Box::new(SQLExprList(
				vec![
					SQLBinary{
						left: Box::new(SQLIdentifier(String::from("a"))),
						op: EQ,
						right: Box::new(SQLLiteral(LiteralString(0, String::from("hello"))))
					},
					SQLBinary{
						left: Box::new(SQLIdentifier(String::from("b"))),
						op: EQ,
						right: Box::new(SQLLiteral(LiteralLong(1, 12345_u64)))
					}
				]
			)),
			selection: Some(Box::new(SQLBinary{
				left: Box::new(SQLIdentifier(String::from("c"))),
				op: GT,
				right : Box::new(SQLLiteral(LiteralLong(2, 10_u64)))
			}))
		},
		parsed
	);

	println!("{:#?}", parser.parse(sql));

	let rewritten = sql_writer::write(parsed, &HashMap::new());

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
			table: Box::new(SQLIdentifier(String::from("foo"))),
			column_list: vec![
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("a"))),
					data_type: Bit { display: None },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("b"))),
					data_type: Bit { display: Some(2) },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("c"))),
					data_type: TinyInt { display: None },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("d"))),
					data_type: TinyInt { display: Some(10) },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("e"))),
					data_type: Bool,
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("f"))),
					data_type: Bool,
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("g"))),
					data_type: SmallInt { display: None },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("h"))),
					data_type: SmallInt { display: Some(100) },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("i"))),
					data_type: Int { display: None },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("j"))),
					data_type: Int { display: Some(64) },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("k"))),
					data_type: Int { display: None },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("l"))),
					data_type: Int { display: Some(64) },
					qualifiers: None
				}, SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("m"))),
					data_type: BigInt { display: None },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("n"))),
					data_type: BigInt { display: Some(100) },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("o"))),
					data_type: Decimal { precision: None, scale: None },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("p"))),
					data_type: Decimal { precision: Some(10), scale: None },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("q"))),
					data_type: Decimal { precision: Some(10), scale: Some(2) },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("r"))),
					data_type: Decimal { precision: None, scale: None },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("s"))),
					data_type: Decimal { precision: Some(10), scale: None },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("t"))),
					data_type: Decimal { precision: Some(10), scale: Some(2) },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("u"))),
					data_type: Float { precision: None, scale: None },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("v"))),
					data_type: Float { precision: Some(10), scale: None },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("w"))),
					data_type: Float { precision: Some(10), scale: Some(2) },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("x"))),
					data_type: Double { precision: None, scale: None },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("y"))),
					data_type: Double { precision: Some(10), scale: None },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("z"))),
					data_type: Double { precision: Some(10), scale: Some(2) },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("aa"))),
					data_type: Double { precision: None, scale: None },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("ab"))),
					data_type: Double { precision: Some(10), scale: None },
					qualifiers: None
				},
				SQLColumnDef {
					column: Box::new(SQLIdentifier(String::from("ac"))),
					data_type: Double { precision: Some(10), scale: Some(2) },
					qualifiers: None
				}
			]
		},
		parsed
	);

	println!("{:#?}", parser.parse(sql));

	let rewritten = sql_writer::write(parsed, &HashMap::new());

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
		    table: Box::new(SQLIdentifier(String::from("foo"))),
		    column_list: vec![
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("a"))),
		            data_type: Date,
					qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("b"))),
		            data_type: DateTime {fsp: None},
					qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("c"))),
		            data_type: DateTime {fsp: Some(6)},
					qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("d"))),
		            data_type: Timestamp {fsp: None},
					qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("e"))),
		            data_type: Timestamp {fsp: Some(6)},
					qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("f"))),
		            data_type: Time {fsp: None},
					qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("g"))),
		            data_type: Time {fsp: Some(6)},
					qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("h"))),
		            data_type: Year {display: None},
					qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("i"))),
		            data_type: Year {display: Some(4)},
					qualifiers: None
		        }
		    ]
		},
		parsed
	);

	println!("{:#?}", parsed);

	let rewritten = sql_writer::write(parsed, &HashMap::new());

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
		    table: Box::new(SQLIdentifier(String::from("foo"))),
		    column_list: vec![
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("a"))),
		            data_type: NChar {length: None},
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("b"))),
		            data_type: Char {length: None},
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("c"))),
		            data_type: Char {length: Some(255)},
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("d"))),
		            data_type: NChar {length: None},
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("e"))),
		            data_type: NChar {length: Some(255)},
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("f"))),
		            data_type: NChar {length: None},
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("g"))),
		            data_type: Char {length: None},
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("h"))),
		            data_type: Char {length: Some(255)},
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("i"))),
		            data_type: NVarchar {length: Some(50)},
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("j"))),
		            data_type: Varchar {length: Some(50)},
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("k"))),
		            data_type: NVarchar {length: Some(50)},
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("l"))),
		            data_type: Varchar {length: Some(50)},
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("m"))),
		            data_type: Binary {length: None},
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("n"))),
		            data_type: Binary {length: Some(50)},
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("o"))),
		            data_type: VarBinary {length: Some(50)},
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("p"))),
		            data_type: TinyBlob,
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("q"))),
		            data_type: TinyText,
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("r"))),
		            data_type: Blob {length: None},
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("s"))),
		            data_type: Blob {length: Some(50)},
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("t"))),
		            data_type: Text {length: None},
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("u"))),
		            data_type: Text {length: Some(100)},
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("v"))),
		            data_type: MediumBlob,
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("w"))),
		            data_type: MediumText,
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("x"))),
		            data_type: LongBlob,
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("y"))),
		            data_type: LongText,
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("z"))),
		            data_type: Enum {values: Box::new(SQLExprList(vec![
		                        SQLLiteral(LiteralString(11,String::from("val1"))),
		                        SQLLiteral(LiteralString(12,String::from("val2"))),
		                        SQLLiteral(LiteralString(13,String::from("val3")))
		                    ]
		                ))
		            },
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("aa"))),
		            data_type: Set {values: Box::new(SQLExprList(vec![
		                        SQLLiteral(LiteralString(14,String::from("val1"))),
		                        SQLLiteral(LiteralString(15,String::from("val2"))),
		                        SQLLiteral(LiteralString(16,String::from("val3")))
		                    ]
		                ))
		            },
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("ab"))),
		            data_type: Char {length: None},
		            qualifiers: None
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("ac"))),
		            data_type: Char {length: Some(50)},
		            qualifiers: None
		        }
		    ]
		},
		parsed
	);

	println!("{:#?}", parsed);

	let rewritten = sql_writer::write(parsed, &HashMap::new());

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
		    table: Box::new(SQLIdentifier(String::from("foo"))),
		    column_list: vec![
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("id"))),
		            data_type: BigInt {display: None},
		            qualifiers: Some(vec![
		                    NotNull,
		                    AutoIncrement,
		                    PrimaryKey
		                ]
		            )
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("a"))),
		            data_type: Varchar {length: Some(50)},
		            qualifiers: Some(vec![
		                    CharacterSet(Box::new(SQLIdentifier(String::from("utf8")))),
		                    Collate(Box::new(SQLIdentifier(String::from("utf8_general_ci")))),
		                    Null,
		                    UniqueKey
		                ]
		            )
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("b"))),
		            data_type: BigInt {display: None},
		            qualifiers: Some(vec![
		                    Signed,
		                    NotNull,
		                    Default(Box::new(SQLLiteral(LiteralLong(1,123456789)))
		                    )
		                ]
		            )
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("c"))),
		            data_type: TinyInt {display: None},
		            qualifiers: Some(vec![
		                    Unsigned,
		                    Null,
		                    Default(Box::new(SQLIdentifier(String::from("NULL")))), // TODO should be literal null ?
							Comment(Box::new(SQLLiteral(LiteralString(2,String::from("Some Comment")))))
		                ]
		            )
		        },
		        SQLColumnDef {
		            column: Box::new(SQLIdentifier(String::from("d"))),
		            data_type: Timestamp {fsp: None},
		            qualifiers: Some(vec![
		                    Default(Box::new(SQLIdentifier(String::from("CURRENT_TIMESTAMP")))),
		                    OnUpdate(Box::new(SQLIdentifier(String::from("CURRENT_TIMESTAMP"))))
		                ]
		            )
		        }
		    ]
		},
		parsed
	);

	println!("{:#?}", parsed);

	let rewritten = sql_writer::write(parsed, &HashMap::new());

	println!("Rewritten: {}", rewritten);
}
