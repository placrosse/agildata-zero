use super::super::ASTNode;
use super::super::ASTNode::*;
use super::super::LiteralExpr::*;
use super::super::MySQLKeyDef::*;
use super::super::MySQLDataType::*;
use super::super::MySQLColumnQualifier::*;
use super::super::{Tokenizer, Parser, SQLWriter, Writer};
use super::super::dialects::ansisql::*;
use super::super::dialects::mysqlsql::*;
use super::test_helper::*;

#[test]
fn create_numeric() {
	let ansi = AnsiSQLDialect::new();
	let dialect = MySQLDialect::new(&ansi);
	let sql = String::from("CREATE TABLE foo (
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
	      )");

	let tokens = sql.tokenize(&dialect).unwrap();
	let parsed = tokens.parse().unwrap();

	assert_eq!(
		MySQLCreateTable {
			table: Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]}),
			column_list: vec![
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
					data_type: Box::new(MySQLDataType(Bit { display: None })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]}),
					data_type: Box::new(MySQLDataType(Bit { display: Some(2) })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}),
					data_type: Box::new(MySQLDataType(TinyInt { display: None })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("d"), parts: vec![String::from("d")]}),
					data_type: Box::new(MySQLDataType(TinyInt { display: Some(10) })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("e"), parts: vec![String::from("e")]}),
					data_type: Box::new(MySQLDataType(Bool)),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("f"), parts: vec![String::from("f")]}),
					data_type: Box::new(MySQLDataType(Bool)),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("g"), parts: vec![String::from("g")]}),
					data_type: Box::new(MySQLDataType(SmallInt { display: None })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("h"), parts: vec![String::from("h")]}),
					data_type: Box::new(MySQLDataType(SmallInt { display: Some(100) })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("i"), parts: vec![String::from("i")]}),
					data_type: Box::new(MySQLDataType(Int { display: None })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("j"), parts: vec![String::from("j")]}),
					data_type: Box::new(MySQLDataType(Int { display: Some(64) })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("k"), parts: vec![String::from("k")]}),
					data_type: Box::new(MySQLDataType(Int { display: None })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("l"), parts: vec![String::from("l")]}),
					data_type: Box::new(MySQLDataType(Int { display: Some(64) })),
					qualifiers: None
				}, MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("m"), parts: vec![String::from("m")]}),
					data_type: Box::new(MySQLDataType(BigInt { display: None })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("n"), parts: vec![String::from("n")]}),
					data_type: Box::new(MySQLDataType(BigInt { display: Some(100) })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("o"), parts: vec![String::from("o")]}),
					data_type: Box::new(MySQLDataType(Decimal { precision: None, scale: None })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("p"), parts: vec![String::from("p")]}),
					data_type: Box::new(MySQLDataType(Decimal { precision: Some(10), scale: None })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("q"), parts: vec![String::from("q")]}),
					data_type: Box::new(MySQLDataType(Decimal { precision: Some(10), scale: Some(2) })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("r"), parts: vec![String::from("r")]}),
					data_type: Box::new(MySQLDataType(Decimal { precision: None, scale: None })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("s"), parts: vec![String::from("s")]}),
					data_type: Box::new(MySQLDataType(Decimal { precision: Some(10), scale: None })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("t"), parts: vec![String::from("t")]}),
					data_type: Box::new(MySQLDataType(Decimal { precision: Some(10), scale: Some(2) })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("u"), parts: vec![String::from("u")]}),
					data_type: Box::new(MySQLDataType(Float { precision: None, scale: None })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("v"), parts: vec![String::from("v")]}),
					data_type: Box::new(MySQLDataType(Float { precision: Some(10), scale: None })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("w"), parts: vec![String::from("w")]}),
					data_type: Box::new(MySQLDataType(Float { precision: Some(10), scale: Some(2) })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("x"), parts: vec![String::from("x")]}),
					data_type: Box::new(MySQLDataType(Double { precision: None, scale: None })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("y"), parts: vec![String::from("y")]}),
					data_type: Box::new(MySQLDataType(Double { precision: Some(10), scale: None })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("z"), parts: vec![String::from("z")]}),
					data_type: Box::new(MySQLDataType(Double { precision: Some(10), scale: Some(2) })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("aa"), parts: vec![String::from("aa")]}),
					data_type: Box::new(MySQLDataType(Double { precision: None, scale: None })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("ab"), parts: vec![String::from("ab")]}),
					data_type: Box::new(MySQLDataType(Double { precision: Some(10), scale: None })),
					qualifiers: None
				},
				MySQLColumnDef {
					column: Box::new(SQLIdentifier{id: String::from("ac"), parts: vec![String::from("ac")]}),
					data_type: Box::new(MySQLDataType(Double { precision: Some(10), scale: Some(2) })),
					qualifiers: None
				}
			],
			keys: vec![],
			table_options: vec![]
		},
		parsed
	);

	println!("{:#?}", parsed);

	let ansi_writer = AnsiSQLWriter{};
	let mysql_writer = MySQLWriter{};
	let writer = SQLWriter::new(vec![&mysql_writer, &ansi_writer]);
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {:?}", rewritten);

}

#[test]
fn create_temporal() {
	let ansi = AnsiSQLDialect::new();
	let dialect = MySQLDialect::new(&ansi);

	let sql = String::from("CREATE TABLE foo (
	      a DATE,
	      b DATETIME,
	      c DATETIME(6),
	      d TIMESTAMP,
	      e TIMESTAMP(6),
	      f TIME,
	      g TIME(6),
	      h YEAR,
	      i YEAR(4)
	  )");

	  let tokens = sql.tokenize(&dialect).unwrap();
	  let parsed = tokens.parse().unwrap();

	assert_eq!(
		MySQLCreateTable {
		    table: Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]}),
		    column_list: vec![
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
		            data_type: Box::new(MySQLDataType(Date)),
					qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]}),
		            data_type: Box::new(MySQLDataType(DateTime {fsp: None})),
					qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}),
		            data_type: Box::new(MySQLDataType(DateTime {fsp: Some(6)})),
					qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("d"), parts: vec![String::from("d")]}),
		            data_type: Box::new(MySQLDataType(Timestamp {fsp: None})),
					qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("e"), parts: vec![String::from("e")]}),
		            data_type: Box::new(MySQLDataType(Timestamp {fsp: Some(6)})),
					qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("f"), parts: vec![String::from("f")]}),
		            data_type: Box::new(MySQLDataType(Time {fsp: None})),
					qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("g"), parts: vec![String::from("g")]}),
		            data_type: Box::new(MySQLDataType(Time {fsp: Some(6)})),
					qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("h"), parts: vec![String::from("h")]}),
		            data_type: Box::new(MySQLDataType(Year {display: None})),
					qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("i"), parts: vec![String::from("i")]}),
		            data_type: Box::new(MySQLDataType(Year {display: Some(4)})),
					qualifiers: None
		        }
		    ],
			keys: vec![],
			table_options: vec![]
		},
		parsed
	);

	println!("{:#?}", parsed);

	let ansi_writer = AnsiSQLWriter{};
	let mysql_writer = MySQLWriter{};
	let writer = SQLWriter::new(vec![&mysql_writer, &ansi_writer]);
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {:?}", rewritten);
}

#[test]
fn create_character() {

	let ansi = AnsiSQLDialect::new();
	let dialect = MySQLDialect::new(&ansi);

	let sql = String::from("CREATE TABLE foo (
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
	)");

	let tokens = sql.tokenize(&dialect).unwrap();
	let parsed = tokens.parse().unwrap();


	assert_eq!(
		MySQLCreateTable {
		    table: Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]}),
		    column_list: vec![
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
		            data_type: Box::new(MySQLDataType(NChar {length: None})),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]}),
		            data_type: Box::new(MySQLDataType(Char {length: None})),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}),
		            data_type: Box::new(MySQLDataType(Char {length: Some(255)})),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("d"), parts: vec![String::from("d")]}),
		            data_type: Box::new(MySQLDataType(NChar {length: None})),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("e"), parts: vec![String::from("e")]}),
		            data_type: Box::new(MySQLDataType(NChar {length: Some(255)})),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("f"), parts: vec![String::from("f")]}),
		            data_type: Box::new(MySQLDataType(NChar {length: None})),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("g"), parts: vec![String::from("g")]}),
		            data_type: Box::new(MySQLDataType(Char {length: None})),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("h"), parts: vec![String::from("h")]}),
		            data_type: Box::new(MySQLDataType(Char {length: Some(255)})),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("i"), parts: vec![String::from("i")]}),
		            data_type: Box::new(MySQLDataType(NVarchar {length: Some(50)})),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("j"), parts: vec![String::from("j")]}),
		            data_type: Box::new(MySQLDataType(Varchar {length: Some(50)})),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("k"), parts: vec![String::from("k")]}),
		            data_type: Box::new(MySQLDataType(NVarchar {length: Some(50)})),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("l"), parts: vec![String::from("l")]}),
		            data_type: Box::new(MySQLDataType(Varchar {length: Some(50)})),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("m"), parts: vec![String::from("m")]}),
		            data_type: Box::new(MySQLDataType(Binary {length: None})),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("n"), parts: vec![String::from("n")]}),
		            data_type: Box::new(MySQLDataType(Binary {length: Some(50)})),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("o"), parts: vec![String::from("o")]}),
		            data_type: Box::new(MySQLDataType(VarBinary {length: Some(50)})),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("p"), parts: vec![String::from("p")]}),
		            data_type: Box::new(MySQLDataType(TinyBlob)),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("q"), parts: vec![String::from("q")]}),
		            data_type: Box::new(MySQLDataType(TinyText)),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("r"), parts: vec![String::from("r")]}),
		            data_type: Box::new(MySQLDataType(Blob {length: None})),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("s"), parts: vec![String::from("s")]}),
		            data_type: Box::new(MySQLDataType(Blob {length: Some(50)})),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("t"), parts: vec![String::from("t")]}),
		            data_type: Box::new(MySQLDataType(Text {length: None})),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("u"), parts: vec![String::from("u")]}),
		            data_type: Box::new(MySQLDataType(Text {length: Some(100)})),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("v"), parts: vec![String::from("v")]}),
		            data_type: Box::new(MySQLDataType(MediumBlob)),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("w"), parts: vec![String::from("w")]}),
		            data_type: Box::new(MySQLDataType(MediumText)),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("x"), parts: vec![String::from("x")]}),
		            data_type: Box::new(MySQLDataType(LongBlob)),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("y"), parts: vec![String::from("y")]}),
		            data_type: Box::new(MySQLDataType(LongText)),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("z"), parts: vec![String::from("z")]}),
		            data_type: Box::new(MySQLDataType(Enum {values: Box::new(SQLExprList(vec![
		                        SQLLiteral(LiteralString(11,String::from("val1"))),
		                        SQLLiteral(LiteralString(12,String::from("val2"))),
		                        SQLLiteral(LiteralString(13,String::from("val3")))
		                    ]
		                ))
		            })),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("aa"), parts: vec![String::from("aa")]}),
		            data_type: Box::new(MySQLDataType(Set {values: Box::new(SQLExprList(vec![
		                        SQLLiteral(LiteralString(14,String::from("val1"))),
		                        SQLLiteral(LiteralString(15,String::from("val2"))),
		                        SQLLiteral(LiteralString(16,String::from("val3")))
		                    ]
		                ))
		            })),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("ab"), parts: vec![String::from("ab")]}),
		            data_type: Box::new(MySQLDataType(CharByte {length: None})),
		            qualifiers: None
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("ac"), parts: vec![String::from("ac")]}),
		            data_type: Box::new(MySQLDataType(CharByte {length: Some(50)})),
		            qualifiers: None
		        }
		    ],
			keys: vec![],
			table_options: vec![]
		},
		parsed
	);

	println!("{:#?}", parsed);

	let ansi_writer = AnsiSQLWriter{};
	let mysql_writer = MySQLWriter{};
	let writer = SQLWriter::new(vec![&mysql_writer, &ansi_writer]);
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {:?}", rewritten);
}

#[test]
fn create_column_qualifiers() {

	let ansi = AnsiSQLDialect::new();
	let dialect = MySQLDialect::new(&ansi);

	let sql = String::from("CREATE TABLE foo (
	      id BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY,
	      a VARCHAR(50) CHARACTER SET utf8 COLLATE utf8_general_ci NULL UNIQUE,
	      b BIGINT SIGNED NOT NULL DEFAULT 123456789,
	      c TINYINT UNSIGNED NULL DEFAULT NULL COMMENT 'Some Comment',
	      d TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
    )");

	let tokens = sql.tokenize(&dialect).unwrap();
	let parsed = tokens.parse().unwrap();

	assert_eq!(
		MySQLCreateTable {
		    table: Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]}),
		    column_list: vec![
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("id"), parts: vec![String::from("id")]}),
		            data_type: Box::new(MySQLDataType(BigInt {display: None})),
		            qualifiers: Some(vec![
		                    MySQLColumnQualifier(NotNull),
		                    MySQLColumnQualifier(AutoIncrement),
		                    MySQLColumnQualifier(PrimaryKey)
		                ]
		            )
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
		            data_type: Box::new(MySQLDataType(Varchar {length: Some(50)})),
		            qualifiers: Some(vec![
		                    MySQLColumnQualifier(CharacterSet(Box::new(SQLIdentifier{id: String::from("utf8"), parts: vec![String::from("utf8")]}))),
		                    MySQLColumnQualifier(Collate(Box::new(SQLIdentifier{id: String::from("utf8_general_ci"), parts: vec![String::from("utf8_general_ci")]}))),
		                    MySQLColumnQualifier(Null),
		                    MySQLColumnQualifier(UniqueKey)
		                ]
		            )
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]}),
		            data_type: Box::new(MySQLDataType(BigInt {display: None})),
		            qualifiers: Some(vec![
		                    MySQLColumnQualifier(Signed),
		                    MySQLColumnQualifier(NotNull),
		                    MySQLColumnQualifier(Default(Box::new(SQLLiteral(LiteralLong(1,123456789)))))
		                ]
		            )
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}),
		            data_type: Box::new(MySQLDataType(TinyInt {display: None})),
		            qualifiers: Some(vec![
		                    MySQLColumnQualifier(Unsigned),
		                    MySQLColumnQualifier(Null),
		                    MySQLColumnQualifier(Default(Box::new(SQLIdentifier{id: String::from("NULL"), parts: vec![String::from("NULL")]}))), // TODO should be literal null ?
							MySQLColumnQualifier(Comment(Box::new(SQLLiteral(LiteralString(2,String::from("Some Comment"))))))
		                ]
		            )
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("d"), parts: vec![String::from("d")]}),
		            data_type: Box::new(MySQLDataType(Timestamp {fsp: None})),
		            qualifiers: Some(vec![
		                    MySQLColumnQualifier(Default(Box::new(SQLIdentifier{id: String::from("CURRENT_TIMESTAMP"), parts: vec![String::from("CURRENT_TIMESTAMP")]}))),
		                    MySQLColumnQualifier(OnUpdate(Box::new(SQLIdentifier{id: String::from("CURRENT_TIMESTAMP"), parts: vec![String::from("CURRENT_TIMESTAMP")]})))
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

	let ansi_writer = AnsiSQLWriter{};
	let mysql_writer = MySQLWriter{};
	let writer = SQLWriter::new(vec![&mysql_writer, &ansi_writer]);
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {:?}", rewritten);
}

#[test]
fn create_tail_keys() {

	let ansi = AnsiSQLDialect::new();
	let dialect = MySQLDialect::new(&ansi);

	let sql = String::from("CREATE TABLE foo (
	      id BIGINT AUTO_INCREMENT,
	      a VARCHAR(50) NOT NULL,
	      b TIMESTAMP NOT NULL,
	      PRIMARY KEY (id),
	      UNIQUE KEY keyName1 (id, b),
	      KEY keyName2 (b),
	      FULLTEXT KEY keyName (a),
	      FOREIGN KEY fkeyName (a) REFERENCES bar(id)
  	)");

	let tokens = sql.tokenize(&dialect).unwrap();
	let parsed = tokens.parse().unwrap();

	assert_eq!(
		MySQLCreateTable {
		    table: Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]}),
		    column_list: vec![
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("id"), parts: vec![String::from("id")]}),
		            data_type: Box::new(MySQLDataType(BigInt {display: None})),
		            qualifiers: Some(vec![MySQLColumnQualifier(AutoIncrement)])
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
		            data_type: Box::new(MySQLDataType(Varchar {length: Some(50)})),
		            qualifiers: Some(vec![MySQLColumnQualifier(NotNull)])
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]}),
		            data_type: Box::new(MySQLDataType(Timestamp {fsp: None})),
		            qualifiers: Some(vec![MySQLColumnQualifier(NotNull)])
		        }
		    ],
		    keys: vec![
		        MySQLKeyDef(Primary {
					symbol: None,
		            name: None,
		            columns: vec![SQLIdentifier{id: String::from("id"), parts: vec![String::from("id")]}]
		        }),
		        MySQLKeyDef(Unique {
					symbol: None,
		            name: Some(Box::new(SQLIdentifier{id: String::from("keyName1"), parts: vec![String::from("keyName1")]})),
		            columns: vec![
		                SQLIdentifier{id: String::from("id"), parts: vec![String::from("id")]},
		                SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]}
		            ]
		        }),
		        MySQLKeyDef(Index {
		            name:  Some(Box::new(SQLIdentifier{id: String::from("keyName2"), parts: vec![String::from("keyName2")]})),
		            columns: vec![SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]}]
		        }),
		        MySQLKeyDef(FullText {
		            name: Some(Box::new(SQLIdentifier{id: String::from("keyName"), parts: vec![String::from("keyName")]})),
		            columns: vec![SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}]
		        }),
		        MySQLKeyDef(Foreign {
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

	let ansi_writer = AnsiSQLWriter{};
	let mysql_writer = MySQLWriter{};
	let writer = SQLWriter::new(vec![&mysql_writer, &ansi_writer]);
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {:?}", rewritten);
}

#[test]
fn create_tail_constraints() {

	let ansi = AnsiSQLDialect::new();
	let dialect = MySQLDialect::new(&ansi);

	let sql = String::from("CREATE TABLE foo (
	      id BIGINT AUTO_INCREMENT,
	      a VARCHAR(50) NOT NULL,
	      b TIMESTAMP NOT NULL,
	      CONSTRAINT symbol1 PRIMARY KEY (id),
	      CONSTRAINT symbol2 UNIQUE KEY keyName1 (a),
	      CONSTRAINT symbol3 FOREIGN KEY fkeyName (a) REFERENCES bar(id)
	)");

	let tokens = sql.tokenize(&dialect).unwrap();
	let parsed = tokens.parse().unwrap();

	println!("{:#?}", parsed);

	assert_eq!(
		MySQLCreateTable {
		    table: Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]}),
		    column_list: vec![
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("id"), parts: vec![String::from("id")]}),
		            data_type: Box::new(MySQLDataType(BigInt {display: None})),
		            qualifiers: Some(vec![MySQLColumnQualifier(AutoIncrement)])
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
		            data_type: Box::new(MySQLDataType(Varchar {length: Some(50)})),
		            qualifiers: Some(vec![MySQLColumnQualifier(NotNull)])
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]}),
		            data_type: Box::new(MySQLDataType(Timestamp {fsp: None})),
		            qualifiers: Some(vec![MySQLColumnQualifier(NotNull)])
		        }
		    ],
		    keys: vec![
				MySQLKeyDef(Primary {
					symbol: Some(Box::new(SQLIdentifier{id: String::from("symbol1"), parts: vec![String::from("symbol1")]})),
					name: None,
					columns: vec![SQLIdentifier{id: String::from("id"), parts: vec![String::from("id")]}]
				}),
				MySQLKeyDef(Unique {
					symbol: Some(Box::new(SQLIdentifier{id: String::from("symbol2"), parts: vec![String::from("symbol2")]})),
					name: Some(Box::new(SQLIdentifier{id: String::from("keyName1"), parts: vec![String::from("keyName1")]})),
					columns: vec![
						SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}
					]
				}),
				MySQLKeyDef(Foreign {
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

	let ansi_writer = AnsiSQLWriter{};
	let mysql_writer = MySQLWriter{};
	let writer = SQLWriter::new(vec![&mysql_writer, &ansi_writer]);
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {:?}", rewritten);
}

#[test]
fn create_table_options() {

	let ansi = AnsiSQLDialect::new();
	let dialect = MySQLDialect::new(&ansi);

	let sql = String::from("CREATE TABLE foo (
	      id BIGINT AUTO_INCREMENT,
	      a VARCHAR(50)
	) Engine InnoDB DEFAULT CHARSET utf8 COMMENT 'Table Comment' AUTO_INCREMENT 12345");

	let tokens = sql.tokenize(&dialect).unwrap();
	let parsed = tokens.parse().unwrap();

	println!("{:#?}", parsed);

	assert_eq!(
		MySQLCreateTable {
		    table: Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]}),
		    column_list: vec![
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("id"), parts: vec![String::from("id")]}),
		            data_type: Box::new(MySQLDataType(BigInt {display: None})),
		            qualifiers: Some(vec![MySQLColumnQualifier(AutoIncrement)])
		        },
		        MySQLColumnDef {
		            column: Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
		            data_type: Box::new(MySQLDataType(Varchar {length: Some(50)})),
		            qualifiers: None
		        }
		    ],
		    keys: vec![],
			table_options: vec![
		        ASTNode::MySQLTableOption(super::super::MySQLTableOption::Engine(Box::new(SQLIdentifier{id: String::from("InnoDB"), parts: vec![String::from("InnoDB")]}))),
		        ASTNode::MySQLTableOption(super::super::MySQLTableOption::Charset(Box::new(SQLIdentifier{id: String::from("utf8"), parts: vec![String::from("utf8")]}))),
		        ASTNode::MySQLTableOption(super::super::MySQLTableOption::Comment(Box::new(SQLLiteral(LiteralString(1,String::from("Table Comment")))))),
				ASTNode::MySQLTableOption(super::super::MySQLTableOption::AutoIncrement(Box::new(SQLLiteral(LiteralLong(2,12345_u64)))))
		    ]
		},
		parsed
	);

	let ansi_writer = AnsiSQLWriter{};
	let mysql_writer = MySQLWriter{};
	let writer = SQLWriter::new(vec![&mysql_writer, &ansi_writer]);
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {:?}", rewritten);
}

#[test]
fn test_integration() {
	let ansi = AnsiSQLDialect::new();
	let dialect = MySQLDialect::new(&ansi);
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
	let mysql_writer = MySQLWriter{};
	let writer = SQLWriter::new(vec![&mysql_writer, &ansi_writer]);
	let rewritten = writer.write(&parsed).unwrap();
	assert_eq!(format_sql(&rewritten), format_sql(&sql));

	println!("Rewritten: {:?}", rewritten);

}
