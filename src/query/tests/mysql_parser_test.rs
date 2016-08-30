// use super::super::ASTNode::*;
// use super::super::Operator::*;
// use super::super::LiteralExpr::*;
// use super::super::JoinType::*;
// use super::super::UnionType::*;
// use super::super::MySQLKeyDef::*;
// use super::super::MySQLDataType::*;
// use super::super::{Tokenizer, Parser, Dialect};
// use super::super::dialects::ansisql::*;
// use super::super::dialects::mysqlsql::*;
//
// #[test]
// fn create_numeric() {
// 	let dialects = vec![box MySQLDialect::new() as Box<Dialect>, box AnsiSQLDialect::new() as Box<Dialect>];
// 	let sql = String::from("CREATE TABLE foo (
// 	      a BIT,
// 	      b BIT(2),
// 	      c TINYINT,
// 	      d TINYINT(10),
// 	      e BOOL,
// 	      f BOOLEAN,
// 	      g SMALLINT,
// 	      h SMALLINT(100),
// 	      i INT,
// 	      j INT(64),
// 	      k INTEGER,
// 	      l INTEGER(64),
// 	      m BIGINT,
// 	      n BIGINT(100),
// 	      o DECIMAL,
// 	      p DECIMAL(10),
// 	      q DECIMAL(10,2),
// 	      r DEC,
// 	      s DEC(10),
// 	      t DEC(10, 2),
// 	      u FLOAT,
// 	      v FLOAT(10),
// 	      w FLOAT(10,2),
// 	      x DOUBLE,
// 	      y DOUBLE(10),
// 	      z DOUBLE(10,2),
// 		  aa DOUBLE PRECISION,
// 		  ab DOUBLE PRECISION (10),
// 		  ac DOUBLE PRECISION (10, 2)
// 	      )");
//
// 	let tokens = sql.tokenize(&dialects).unwrap();
// 	let parsed = tokens.parse().unwrap();
//
// 	assert_eq!(
// 		MySQLCreateTable {
// 			table: Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]}),
// 			column_list: vec![
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
// 					data_type: Box::new(MySQLDataType(Bit { display: None })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]}),
// 					data_type: Box::new(MySQLDataType(Bit { display: Some(2) })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}),
// 					data_type: Box::new(MySQLDataType(TinyInt { display: None })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("d"), parts: vec![String::from("d")]}),
// 					data_type: Box::new(MySQLDataType(TinyInt { display: Some(10) })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("e"), parts: vec![String::from("e")]}),
// 					data_type: Box::new(MySQLDataType(Bool)),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("f"), parts: vec![String::from("f")]}),
// 					data_type: Box::new(MySQLDataType(Bool)),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("g"), parts: vec![String::from("g")]}),
// 					data_type: Box::new(MySQLDataType(SmallInt { display: None })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("h"), parts: vec![String::from("h")]}),
// 					data_type: Box::new(MySQLDataType(SmallInt { display: Some(100) })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("i"), parts: vec![String::from("i")]}),
// 					data_type: Box::new(MySQLDataType(Int { display: None })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("j"), parts: vec![String::from("j")]}),
// 					data_type: Box::new(MySQLDataType(Int { display: Some(64) })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("k"), parts: vec![String::from("k")]}),
// 					data_type: Box::new(MySQLDataType(Int { display: None })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("l"), parts: vec![String::from("l")]}),
// 					data_type: Box::new(MySQLDataType(Int { display: Some(64) })),
// 					qualifiers: None
// 				}, MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("m"), parts: vec![String::from("m")]}),
// 					data_type: Box::new(MySQLDataType(BigInt { display: None })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("n"), parts: vec![String::from("n")]}),
// 					data_type: Box::new(MySQLDataType(BigInt { display: Some(100) })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("o"), parts: vec![String::from("o")]}),
// 					data_type: Box::new(MySQLDataType(Decimal { precision: None, scale: None })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("p"), parts: vec![String::from("p")]}),
// 					data_type: Box::new(MySQLDataType(Decimal { precision: Some(10), scale: None })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("q"), parts: vec![String::from("q")]}),
// 					data_type: Box::new(MySQLDataType(Decimal { precision: Some(10), scale: Some(2) })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("r"), parts: vec![String::from("r")]}),
// 					data_type: Box::new(MySQLDataType(Decimal { precision: None, scale: None })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("s"), parts: vec![String::from("s")]}),
// 					data_type: Box::new(MySQLDataType(Decimal { precision: Some(10), scale: None })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("t"), parts: vec![String::from("t")]}),
// 					data_type: Box::new(MySQLDataType(Decimal { precision: Some(10), scale: Some(2) })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("u"), parts: vec![String::from("u")]}),
// 					data_type: Box::new(MySQLDataType(Float { precision: None, scale: None })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("v"), parts: vec![String::from("v")]}),
// 					data_type: Box::new(MySQLDataType(Float { precision: Some(10), scale: None })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("w"), parts: vec![String::from("w")]}),
// 					data_type: Box::new(MySQLDataType(Float { precision: Some(10), scale: Some(2) })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("x"), parts: vec![String::from("x")]}),
// 					data_type: Box::new(MySQLDataType(Double { precision: None, scale: None })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("y"), parts: vec![String::from("y")]}),
// 					data_type: Box::new(MySQLDataType(Double { precision: Some(10), scale: None })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("z"), parts: vec![String::from("z")]}),
// 					data_type: Box::new(MySQLDataType(Double { precision: Some(10), scale: Some(2) })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("aa"), parts: vec![String::from("aa")]}),
// 					data_type: Box::new(MySQLDataType(Double { precision: None, scale: None })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("ab"), parts: vec![String::from("ab")]}),
// 					data_type: Box::new(MySQLDataType(Double { precision: Some(10), scale: None })),
// 					qualifiers: None
// 				},
// 				MySQLColumnDef {
// 					column: Box::new(SQLIdentifier{id: String::from("ac"), parts: vec![String::from("ac")]}),
// 					data_type: Box::new(MySQLDataType(Double { precision: Some(10), scale: Some(2) })),
// 					qualifiers: None
// 				}
// 			],
// 			keys: vec![],
// 			table_options: vec![]
// 		},
// 		parsed
// 	);
//
// 	println!("{:#?}", parsed);
//
// 	// let writer = SQLWriter::default();
// 	// let rewritten = writer.write(&parsed).unwrap();
// 	// assert_eq!(format_sql(&rewritten), format_sql(&sql));
// 	//
// 	// println!("Rewritten: {}", rewritten);
//
// }
