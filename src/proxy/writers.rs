use super::super::parser::sql_writer::*;
use super::super::parser::sql_parser::{SQLExpr, LiteralExpr, DataType};
use std::collections::HashMap;
use std::fmt::Write;
use config::*;
use encrypt::*;

pub fn to_hex_string(bytes: &Vec<u8>) -> String {
  let strs: Vec<String> = bytes.iter()
                               .map(|b| format!("{:02X}", b))
                               .collect();
  strs.join("")
}

pub struct LiteralReplacingWriter<'a> {
    pub literals: &'a HashMap<u32, Option<Vec<u8>>>
}

impl<'a> ExprWriter for LiteralReplacingWriter<'a> {
	fn write(&self, _: &Writer, builder: &mut String, node: &SQLExpr) -> Result<bool, String> {
		match node {
			&SQLExpr::SQLLiteral(ref e) => match e {
				&LiteralExpr::LiteralLong(ref i, _) => self.optionally_write_literal(i, builder),
    			&LiteralExpr::LiteralBool(ref i, _) => self.optionally_write_literal(i, builder),
    			&LiteralExpr::LiteralDouble(ref i, _) => self.optionally_write_literal(i, builder),
    			&LiteralExpr::LiteralString(ref i, _) => self.optionally_write_literal(i, builder)
			},
			_ => Ok(false)
		}
	}
}

impl<'a> LiteralReplacingWriter<'a> {
	fn optionally_write_literal(&self, index: &u32, builder: &mut String) -> Result<bool, String> {
		match self.literals.get(index) {
			Some(value) => match value {
				&Some(ref e) => {
					write!(builder, "X'{}'", to_hex_string(e)).unwrap();
					Ok(true)
				},
				&None => Ok(false),
			},
			None => Ok(false),
		}
	}
}

pub struct CreateTranslatingWriter<'a> {
	pub config: &'a Config,
	pub schema: &'a String
}

impl<'a> ExprWriter for CreateTranslatingWriter<'a> {
	fn write(&self, writer: &Writer, builder: &mut String, node: &SQLExpr) -> Result<bool, String> {
		match node {
			&SQLExpr::SQLCreateTable{box ref table, ref column_list, ref keys, ref table_options} => {
				let table_name = match table {
					&SQLExpr::SQLIdentifier(ref t) => t,
					_ => return Err(String::from(format!("Expected identifier, received {:?}", table)))
				};

				builder.push_str("CREATE TABLE");
				writer._write(builder, table)?;
				builder.push_str("(");

				let mut sep = "";
				for c in column_list.iter() {
					builder.push_str(sep);

					let column_name = match c {
						&SQLExpr::SQLColumnDef{box ref column, ..} => match column {
							&SQLExpr::SQLIdentifier(ref t) => t,
							_ => return Err(String::from(format!("Expected identifier, received {:?}", table)))
						},
						_ => return Err(String::from(format!("Expected column definition, received {:?}", table)))
					};

					let col = self.config.get_column_config(&self.schema, &table_name, &column_name);
					match col {
						Some(config) => {
							match c {
								&SQLExpr::SQLColumnDef{box ref column, box ref data_type, ref qualifiers} => {
									writer._write(builder, column)?;

									let encryption_type = &config.encryption;
									match encryption_type {
										&EncryptionType::NA => writer._write(builder, data_type)?,
										_ => writer._write(builder, &self.translate_type(data_type, &config.encryption)?)?
									}

									match qualifiers {
										&Some(ref list) => {
											for q in list.iter() {
												writer._write(builder, q)?;
											}
										},
										_=> {}
									}

								},
								_ => return Err(String::from(format!("Expected column definition, received {:?}", c)))
							}
						},
						_ => {
							writer._write(builder, c)?;
						}
					}

					sep = ", "
				}

				for k in keys.iter() {
					builder.push_str(sep);
					writer._write(builder, k)?;
				}

				builder.push_str(")");

				for o in table_options.iter() {
					writer._write(builder, o)?;
				}

				Ok(true)
			},
			_ => Ok(false)
		}
	}
}

// TODO needs to do some real length/display math for different encryption types
impl<'a> CreateTranslatingWriter<'a> {
	fn translate_type(&self, data_type: &SQLExpr, encryption: &EncryptionType) -> Result<SQLExpr, String> {
		match (data_type, encryption) {
			(&SQLExpr::SQLDataType(ref dt), &EncryptionType::AES) => match dt {
				&DataType::Int{..} => {
					// TODO currently all are stored as 8 bytes, delegate to encrypt
					Ok(SQLExpr::SQLDataType(DataType::VarBinary{length: Some(8 + 28)}))
				},
				&DataType::Varchar{ref length} | &DataType::Char{ref length} |
				&DataType::Blob{ref length} | &DataType::Text{ref length} |
				&DataType::Binary{ref length} | &DataType::VarBinary{ref length} => {
					Ok(SQLExpr::SQLDataType(DataType::VarBinary{length: Some(self.get_encrypted_string_length(length))}))
				},
				_ => Err(String::from(format!("Unsupported data type for AES translation {:?}", dt)))
			},
			_ => Err(String::from(format!("Expected data type and encryption, received data_type: {:?}, encryption: {:?}", data_type, encryption)))
		}
	}

	// TODO delegate to crypt module
	fn get_encrypted_string_length(&self, len: &Option<u32>) -> u32 {
		if len.is_some() {
			len.unwrap() + 28
		} else {
			1024 + 28
		}
	}
}

#[cfg(test)]
mod tests {

	use super::{CreateTranslatingWriter};
	use parser::sql_parser::AnsiSQLParser;
	use parser::sql_writer::*;
    use config;
	use byteorder::{WriteBytesExt,ReadBytesExt,BigEndian};

	#[test]
	fn simple_users() {
		let parser = AnsiSQLParser {};
		let config = config::parse_config("example-babel-config.xml");
		let schema = String::from("babel");

		let sql = "CREATE TABLE users (
			id INTEGER PRIMARY KEY,
			first_name VARCHAR(50),
			last_name VARCHAR(50),
			ssn VARCHAR(50),
			age INTEGER,
			sex VARCHAR(50)
		)";

		let parsed = parser.parse(sql).unwrap();

		let translator = CreateTranslatingWriter {
			config: &config,
			schema: &schema
		};

		let writer = SQLWriter::new(vec![&translator]);

		let expected = "CREATE TABLE users (
			id INTEGER PRIMARY KEY,
			first_name VARBINARY(78),
			last_name VARBINARY(78),
			ssn VARBINARY(78),
			age VARBINARY(36),
			sex VARBINARY(78)
		)";

		let rewritten = writer.write(&parsed).unwrap();

		println!("REWRITTEN {}", rewritten);

		assert_eq!(format_sql(&rewritten), format_sql(&expected));

	}

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

			// strip whitespace
			.replace(" ", "").replace("\n", "").replace("\r", "").replace("\t", "")


	}
}
