// use super::super::parser::sql_writer::*;
// use super::super::parser::sql_parser::{SQLExpr, LiteralExpr, DataType};
use query::{Writer, ExprWriter, ASTNode, MySQLColumnQualifier, LiteralToken, Token};
use query::MySQLDataType::*;
use std::collections::HashMap;
use std::fmt::Write;
use config::*;
use encrypt::*;
use error::ZeroError;
use super::physical_planner::{EncryptionPlan};
use decimal::*;
use std::fmt::Debug;
use std::str::FromStr;
use chrono::UTC;
use chrono::offset::TimeZone;


pub fn to_hex_string(bytes: &Vec<u8>) -> String {
  let strs: Vec<String> = bytes.iter()
                               .map(|b| format!("{:02X}", b))
                               .collect();
  strs.join("")
}

pub fn map_err_to_zero<E: Debug>(err: E) -> Box<ZeroError> {
    ZeroError::EncryptionError {
        message: format!("{:?}", err),
        code: "1064".into()
    }.into()
}


pub struct LiteralEncryptionWriter<'a> {
    pub literals: &'a Vec<LiteralToken>,
    pub literal_plans: &'a HashMap<usize, EncryptionPlan>
}

impl<'a> ExprWriter for LiteralEncryptionWriter<'a> {
    fn write(&self, _: &Writer, builder: &mut String, node: &ASTNode) -> Result<bool, Box<ZeroError>> {
        match node {
            &ASTNode::SQLLiteral(i) => {
                if self.literal_plans.contains_key(&i) {
                    let lit = self.literals.get(i).unwrap();
                    let plan = self.literal_plans.get(&i).unwrap();

                    match plan.encryption {
                        EncryptionType::NA => Ok(false),
                        _ => {
                            let key = plan.key.unwrap();
                            let encrypted = match plan.data_type {
                                NativeType::U64 => {
                                    match lit {
                                        &LiteralToken::LiteralLong(ref i, ref v) => {

                                            let val = u64::from_str(v).map_err(map_err_to_zero)?;
                                            val.encrypt(&plan.encryption, &key)?

                                        },
                                        _ => return Err(ZeroError::EncryptionError {
                                            message: format!("Invalid value {:?} expected type {:?}", lit, plan.data_type).into(),
                                            code: "1064".into()
                                        }.into())
                                    }
                                },
                                NativeType::I64 => {
                                    return Err(ZeroError::EncryptionError {
                                        message: format!("Signed numerics currently unsupported").into(),
                                        code: "1064".into()
                                    }.into())
                                    // TODO reimplement
                                    //                                match lit {
                                    //                                    &LiteralToken::LiteralLong(ref i, ref val) => {
                                    //                                        let v = match i64::from_str(val) {
                                    //                                            Ok(v) => v,
                                    //                                            Err(e) => return Err(ZeroError::EncryptionError {
                                    //                                                message: format!("Failed to coerce {} to signed due to : {}", val, e).into(),
                                    //                                                code: "1064".into()
                                    //                                            }.into())
                                    //                                        };
                                    //
                                    //                                        let encrypted = match sign {
                                    //                                            Some(&Operator::SUB) => (-v).encrypt(&plan.encryption, &plan.key)?,
                                    //                                            _ => v.encrypt(&plan.encryption, &plan.key)?
                                    //                                        };
                                    //                                         encrypted
                                    //                                    },
                                    //                                    _ => return Err(ZeroError::EncryptionError {
                                    //                                        message: format!("Invalid value {:?} for column {}.{}", lit, plan.relation, plan.name).into(),
                                    //                                        code: "1064".into()
                                    //                                    }.into())
                                    //                                }
                                },
                                NativeType::F64 => {
                                    match lit {
                                        &LiteralToken::LiteralDouble(ref i, ref v) => {
                                            let val = f64::from_str(v).map_err(map_err_to_zero)?;
                                            val.encrypt(&plan.encryption, &key)?
                                        },
                                        _ => return Err(ZeroError::EncryptionError {
                                            message: format!("Invalid value {:?} expected type {:?}", lit, plan.data_type).into(),
                                            code: "1064".into()
                                        }.into())
                                    }
                                },
                                NativeType::D128 => {
                                    match lit {
                                        &LiteralToken::LiteralDouble(ref i, ref val) => {
                                            let v = match d128::from_str(val) {
                                                Ok(d) => d,
                                                // Note: d128::from_str e is a ()
                                                Err(e) => return Err(ZeroError::EncryptionError {
                                                    message: format!("Failed to coerce {} to d128", val).into(),
                                                    code: "1064".into()
                                                }.into())
                                            };

                                            v.encrypt(&plan.encryption, &key)?
                                        },
                                        _ => return Err(ZeroError::EncryptionError {
                                            message: format!("Invalid value {:?} expected type {:?}", lit, plan.data_type).into(),
                                            code: "1064".into()
                                        }.into())
                                    }
                                },
                                NativeType::BOOL => {
                                    match lit {
                                        &LiteralToken::LiteralBool(ref i, ref v) => {
                                            let val = bool::from_str(v).map_err(map_err_to_zero)?;
                                            val.encrypt(&plan.encryption, &key)?
                                        },
                                        _ => return Err(ZeroError::EncryptionError {
                                            message: format!("Invalid value {:?} expected type {:?}", lit, plan.data_type).into(),
                                            code: "1064".into()
                                        }.into())
                                    }
                                },
                                NativeType::Varchar(..) | NativeType::Char(..) => {
                                    match lit {
                                        &LiteralToken::LiteralString(ref i, ref val) => {
                                            val.clone().encrypt(&plan.encryption, &key)?
                                        },
                                        _ => return Err(ZeroError::EncryptionError {
                                            message: format!("Invalid value {:?} expected type {:?}", lit, plan.data_type).into(),
                                            code: "1064".into()
                                        }.into())
                                    }
                                },
                                NativeType::DATE => {
                                    match lit {
                                        &LiteralToken::LiteralString(ref i, ref val) => {
                                            let v = match UTC.datetime_from_str(&format!("{} 00:00:00",val), "%Y-%m-%d %H:%M:%S") {
                                                Ok(v) => v,
                                                Err(e) => return Err(ZeroError::EncryptionError {
                                                    message: format!("Failed to coerce {} to date due to {}", val, e).into(),
                                                    code: "1064".into()
                                                }.into())
                                            };

                                            v.encrypt(&plan.encryption, &key)?
                                        },
                                        _ => return Err(ZeroError::EncryptionError {
                                            message: format!("Invalid value {:?} expected type {:?}", lit, plan.data_type).into(),
                                            code: "1064".into()
                                        }.into())
                                    }
                                },
                                NativeType::DATETIME(..) => {
                                    match lit {
                                        &LiteralToken::LiteralString(ref i, ref val) => {
                                            let v = match UTC.datetime_from_str(val, "%Y-%m-%d %H:%M:%S%.f") {
                                                Ok(v) => v,
                                                Err(e) => return Err(ZeroError::EncryptionError {
                                                    message: format!("Failed to coerce {} to DATETIME due to {}", val, e).into(),
                                                    code: "1064".into()
                                                }.into())
                                            };

                                            v.clone().encrypt(&plan.encryption, &key)?
                                        },
                                        _ => return Err(ZeroError::EncryptionError {
                                            message: format!("Invalid value {:?} expected type {:?}", lit, plan.data_type).into(),
                                            code: "1064".into()
                                        }.into())
                                    }
                                },
                                _ => return Err(ZeroError::EncryptionError {
                                    message: format!("Unsupported encryption {:?} for data type {:?}", plan.encryption, plan.data_type).into(),
                                    code: "1064".into()
                                }.into())
                            };

                            write!(builder, "X'{}'", to_hex_string(&encrypted)).unwrap();
                            Ok(true)
                        }
                    }
                } else {
                    Ok(false)
                }


            },
            _ => Ok(false)
        }
    }
}

pub struct LiteralReplacingWriter<'a> {
    pub encrypted_literals: &'a HashMap<u32, Vec<u8>>
}

impl<'a> ExprWriter for LiteralReplacingWriter<'a> {
	fn write(&self, _: &Writer, builder: &mut String, node: &ASTNode) -> Result<bool, Box<ZeroError>> {
		match node {
			&ASTNode::SQLLiteral(i) => {
                let index = i as u32;
                self.optionally_write_literal(&index, builder)
            },
            &ASTNode::SQLUnary{ref operator, expr: box ASTNode::SQLLiteral(i)} => {
                // This value was encrypted as a signed value, so do not write the unary...
                let index = i as u32;
                if self.encrypted_literals.contains_key(&index as &u32) {
                  self.optionally_write_literal(&index as &u32, builder)
                } else {
                  Ok(false)
                }
            },
			_ => Ok(false)
		}
	}
}

impl<'a> LiteralReplacingWriter<'a> {
	fn optionally_write_literal(&self, index: &u32, builder: &mut String) -> Result<bool, Box<ZeroError>> {
		match self.encrypted_literals.get(index) {
			Some(value) => {
				write!(builder, "X'{}'", to_hex_string(value)).unwrap();
				Ok(true)
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
	fn write(&self, writer: &Writer, builder: &mut String, node: &ASTNode) -> Result<bool, Box<ZeroError>> {
		match node {
			&ASTNode::MySQLCreateTable{box ref table, ref column_list, ref keys, ref table_options} => {
				let table_name = match table {
					&ASTNode::SQLIdentifier{id: ref t, ..} => t,
					_ => return  Err(ZeroError::ParseError{
                            message: format!("Expected identifier, received {:?}", table).into(),
                            code: "1064".into()
                        }.into())
				};

				builder.push_str("CREATE TABLE");
				writer._write(builder, table)?;
				builder.push_str("(");

				let mut sep = "";
				for c in column_list.iter() {
					builder.push_str(sep);

					let column_name = match c {
						&ASTNode::MySQLColumnDef{box ref column, ..} => match column {
							&ASTNode::SQLIdentifier{id: ref t, ..} => t,
							_ => return  Err(ZeroError::ParseError{
                        message: format!("Expected identifier, received {:?}", table).into(),
                        code: "1064".into()
                    }.into())
						},
						_ => return  Err(ZeroError::ParseError{
                                message: format!("Expected column definition, received {:?}", table).into(),
                                code: "1064".into()
                            }.into())
					};

					let col = self.config.get_column_config(&self.schema, &table_name, &column_name);
					match col {
						Some(config) => {
							match c {
								&ASTNode::MySQLColumnDef{box ref column, box ref data_type, ref qualifiers} => {
									writer._write(builder, column)?;

									let encryption_type = &config.encryption;
									match encryption_type {
										&EncryptionType::NA => writer._write(builder, data_type)?,
										_ => writer._write(builder, &self.translate_type(data_type, &config.encryption)?)?
									}


									match qualifiers {
										&Some(ref list) => {
											for q in list.iter() {
                                                if let &ASTNode::MySQLColumnQualifier(ref qual) = q {
                                                    match qual {
                                                        &MySQLColumnQualifier::Signed | &MySQLColumnQualifier::Unsigned => {
                                                            if encryption_type == &EncryptionType::NA {
                                                                writer._write(builder, q)?
                                                            }
                                                        },
                                                        _ => writer._write(builder, q)?
                                                    }
                                                }
											}
										},
										_=> {}
									}

								},
								_ => return Err(ZeroError::ParseError{
                                        message: format!("Expected column definition, received {:?}", c).into(),
                                        code: "1064".into()
                                    }.into())
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
	fn translate_type(&self, data_type: &ASTNode, encryption: &EncryptionType) -> Result<ASTNode, Box<ZeroError>> {
		match (data_type, encryption) {
			(&ASTNode::MySQLDataType(ref dt), &EncryptionType::AES) => match dt {
                &Bit{..} | &TinyInt{..} |
                &SmallInt{..} | &MediumInt{..} |
                &Int{..} | &BigInt{..}  => {
					// TODO currently all are stored as 8 bytes
					Ok(ASTNode::MySQLDataType(Binary{length: Some(8 + 28)}))
				},
                &Bool => Ok(ASTNode::MySQLDataType(Binary{length: Some(1 + 28)})),
                &Decimal{..} => Ok(ASTNode::MySQLDataType(Binary{length: Some(16 + 28)})),
                &Float{..} | &Double{..} => Ok(ASTNode::MySQLDataType(Binary{length: Some(8 + 28)})),
                &Char{ref length} | &NChar{ref length} => {
                    let l = length.unwrap_or(1) + 28;
                    Ok(ASTNode::MySQLDataType(VarBinary{length: Some(l)}))
                },
				&Varchar{ref length} | &NVarchar{ref length} => {
					Ok(ASTNode::MySQLDataType(VarBinary{length: Some(self.get_encrypted_string_length(length))}))
				},
                &Date | &DateTime{..} => Ok(ASTNode::MySQLDataType(Binary{length: Some(12 + 28)})),
				_ => Err(ZeroError::EncryptionError{
                        message: format!("Unsupported data type for AES translation {:?}", dt).into(),
                        code: "1064".into()
                    }.into())
			},
			_ => Err(ZeroError::EncryptionError{
                    message: format!("Expected data type and encryption, received data_type: {:?}, encryption: {:?}", data_type, encryption).into(),
                    code: "1064".into()
                }.into())
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
	use query::{Writer, SQLWriter, Tokenizer, Parser};
    use query::dialects::mysqlsql::*;
    use query::dialects::ansisql::*;
    use config;

	#[test]
	fn simple_users() {
        let ansi = AnsiSQLDialect::new();
        let dialect = MySQLDialect::new(&ansi);

		let config = config::parse_config("zero-config.xml");
		let schema = String::from("zero");

		let sql = String::from("CREATE TABLE users (
			id INTEGER PRIMARY KEY,
			first_name VARCHAR(50),
			last_name VARCHAR(50),
			ssn VARCHAR(50),
			age INTEGER,
			sex VARCHAR(50)
		)");

        let tokens = sql.tokenize(&dialect).unwrap();
		let parsed = tokens.parse().unwrap();

		let translator = CreateTranslatingWriter {
			config: &config,
			schema: &schema
		};
        let mysql = MySQLWriter{};
        let ansi = AnsiSQLWriter{literal_tokens: &tokens.literals};

		let writer = SQLWriter::new(vec![&translator, &mysql, &ansi]);

		let expected = "CREATE TABLE users (
			id INTEGER PRIMARY KEY,
			first_name VARBINARY(78),
			last_name VARBINARY(78),
			ssn VARBINARY(78),
			age BINARY(36),
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
