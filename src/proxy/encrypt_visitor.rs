use query::planner::{Rel, Rex, RelVisitor, TupleType, HasTupleType, Element};
use query::{Operator, LiteralExpr};
use std::collections::HashMap;
use encrypt::*;
use error::ZeroError;
use decimal::*;
use std::str::FromStr;
use chrono::*;
// TODO error: use of unstable library feature 'try_from' (see issue #33417)
//use std::convert::TryFrom;

#[derive(Debug)]
pub struct EncryptVisitor {
    // TODO this should be Option<Vec<u8>> for null handling
	pub valuemap: HashMap<u32, Vec<u8>>
}

impl EncryptVisitor {
	pub fn get_value_map(&self) -> &HashMap<u32, Vec<u8>> {
		&self.valuemap
	}

    pub fn encrypt_literal(&mut self, lit: &LiteralExpr, el: &Element, sign: Option<&Operator>) -> Result<(), Box<ZeroError>> {
        match el.data_type {
            NativeType::U64 => {
                match lit {
                    &LiteralExpr::LiteralLong(ref i, ref val) => {
                        match sign {
                            Some(&Operator::SUB) => return Err(ZeroError::EncryptionError {
                                message: format!("Negative unary unsupported on unsigned numerics, column: {}.{}", el.name, el.relation).into(),
                                code: "1064".into()
                            }.into()),
                            _ => {self.valuemap.insert(i.clone(), val.encrypt(&el.encryption, &el.key)?);}
                        }

                    },
                    _ => return Err(ZeroError::EncryptionError {
                        message: format!("Invalid value {:?} for column {}.{}", lit, el.name, el.relation).into(),
                        code: "1064".into()
                    }.into())
                }
            },
            NativeType::I64 => {
                match lit {
                    &LiteralExpr::LiteralLong(ref i, ref val) => {
                        let v = match i64::from_str(&format!("{}", val)) {
                            Ok(v) => v,
                            Err(e) => return Err(ZeroError::EncryptionError {
                                message: format!("Failed to coerce {} to signed due to : {}", val, e).into(),
                                code: "1064".into()
                            }.into())
                        };

                        let encrypted = match sign {
                            Some(&Operator::SUB) => (-v).encrypt(&el.encryption, &el.key)?,
                            _ => v.encrypt(&el.encryption, &el.key)?
                        };
                        self.valuemap.insert(i.clone(), encrypted);
                    },
                    _ => return Err(ZeroError::EncryptionError {
                        message: format!("Invalid value {:?} for column {}.{}", lit, el.relation, el.name).into(),
                        code: "1064".into()
                    }.into())
                }
            },
            NativeType::F64 => {
                match lit {
                    &LiteralExpr::LiteralDouble(ref i, ref val) => {
                        let encrypted = match sign {
                            Some(&Operator::SUB) => (-val).encrypt(&el.encryption, &el.key)?,
                            _ => val.encrypt(&el.encryption, &el.key)?
                        };
                        self.valuemap.insert(i.clone(), encrypted);
                    },
                    _ => return Err(ZeroError::EncryptionError {
                        message: format!("Invalid value {:?} for column {}.{}", lit, el.relation, el.name).into(),
                        code: "1064".into()
                    }.into())
                }
            },
            NativeType::D128 => {
                match lit {
                    &LiteralExpr::LiteralDouble(ref i, ref val) => {
                        // TODO precision loss?
                        let v = match d128::from_str(&val.to_string()) {
                            Ok(d) => d,
                            // Note: d128::from_str e is a ()
                            Err(e) => return Err(ZeroError::EncryptionError {
                                message: format!("Failed to coerce {} to d128", val).into(),
                                code: "1064".into()
                            }.into())
                        };

                        let encrypted = match sign {
                            Some(&Operator::SUB) => (-v).encrypt(&el.encryption, &el.key)?,
                            _ => v.encrypt(&el.encryption, &el.key)?
                        };
                        self.valuemap.insert(i.clone(), encrypted);
                    },
                    _ => return Err(ZeroError::EncryptionError {
                        message: format!("Invalid value {:?} for column {}.{}", lit, el.relation, el.name).into(),
                        code: "1064".into()
                    }.into())
                }
            },
            NativeType::BOOL => {
                match lit {
                    &LiteralExpr::LiteralBool(ref i, ref val) => {
                        self.valuemap.insert(i.clone(), val.encrypt(&el.encryption, &el.key)?);
                    },
                    _ => return Err(ZeroError::EncryptionError {
                        message: format!("Invalid value {:?} for column {}.{}", lit, el.relation, el.name).into(),
                        code: "1064".into()
                    }.into())
                }
            },
            NativeType::Varchar(..) | NativeType::Char(..) => {
                match lit {
                    &LiteralExpr::LiteralString(ref i, ref val) => {
                        self.valuemap.insert(i.clone(), val.clone().encrypt(&el.encryption, &el.key)?);
                    },
                    _ => return Err(ZeroError::EncryptionError {
                        message: format!("Invalid value {:?} for column {}.{}", lit, el.relation, el.name).into(),
                        code: "1064".into()
                    }.into())
                }
            },
            NativeType::DATE => {
                match lit {
                    &LiteralExpr::LiteralString(ref i, ref val) => {
                        let v = match UTC.datetime_from_str(&format!("{} 00:00:00",val), "%Y-%m-%d %H:%M:%S") {
                            Ok(v) => v,
                            Err(e) => return Err(ZeroError::EncryptionError {
                                message: format!("Failed to coerce {} to date due to {}", val, e).into(),
                                code: "1064".into()
                            }.into())
                        };

                        self.valuemap.insert(i.clone(), v.encrypt(&el.encryption, &el.key)?);
                    },
                    _ => return Err(ZeroError::EncryptionError {
                        message: format!("Invalid value {:?} for column {}.{}", lit, el.relation, el.name).into(),
                        code: "1064".into()
                    }.into())
                }
            },
            NativeType::DATETIME(..) => {
                match lit {
                    &LiteralExpr::LiteralString(ref i, ref val) => {
                        let v = match UTC.datetime_from_str(val, "%Y-%m-%d %H:%M:%S%.f") {
                            Ok(v) => v,
                            Err(e) => return Err(ZeroError::EncryptionError {
                                message: format!("Failed to coerce {} to DATETIME due to {}", val, e).into(),
                                code: "1064".into()
                            }.into())
                        };

                        self.valuemap.insert(i.clone(), v.clone().encrypt(&el.encryption, &el.key)?);
                    },
                    _ => return Err(ZeroError::EncryptionError {
                        message: format!("Invalid value {:?} for column {}.{}", lit, el.relation, el.name).into(),
                        code: "1064".into()
                    }.into())
                }
            },
            _ => return Err(ZeroError::EncryptionError {
                message: format!("Unsupported data type {:?} for encryption {:?}. Column: {}.{}", el.data_type, el.encryption, el.relation, el.name).into(),
                code: "1064".into()
            }.into())

        }

        Ok(())
    }
}


impl RelVisitor for EncryptVisitor  {
	fn visit_rel(&mut self, rel: &Rel) -> Result<(),  Box<ZeroError>> {
		match rel {
			&Rel::Projection{box ref project, box ref input, ref tt} => {
				self.visit_rex(project, tt)?;
				self.visit_rel(input)?;
			},
			&Rel::Selection{box ref expr, box ref input} => {
				self.visit_rex(expr, input.tt())?;
				self.visit_rel(input)?;
			},
			&Rel::TableScan{..} => {},
			&Rel::Join{box ref left, box ref right, ref on_expr, ref tt, ..} => {
				self.visit_rel(left)?;
				self.visit_rel(right)?;
				match on_expr {
					&Some(box ref o) => self.visit_rex(o, tt)?,
					&None => {}
				}
			},
			&Rel::AliasedRel{box ref input, ..} => self.visit_rel(input)?,
			&Rel::Dual{..} => {},
			&Rel::Update{ref table, box ref set_stmts, ref selection, ref tt} => {
				match set_stmts {
					&Rex::RexExprList(ref list) => {
						for e in list.iter() {
							self.visit_rex(e, tt);
						}
					},
					_ => {}
				}
				match selection {
					&Some(box ref s) => self.visit_rex(s, tt)?,
					&None => {}
				}
			},
			&Rel::Insert{ref table, box ref columns, box ref values, ..} => {
				match (columns, values) {
					(&Rex::RexExprList(ref c_list), &Rex::RexExprList(ref v_list)) => {
						for (index, v) in v_list.iter().enumerate() {
							match v {
								&Rex::Literal(ref lit) => {
									if let Rex::Identifier{ref id, ref el} = c_list[index] {
										if el.encryption != EncryptionType::NA {
                                            self.encrypt_literal(lit, el, None)?;
										}

									} else {
										return Err(ZeroError::EncryptionError{
                                            message: format!("Expected identifier at column list index {}, received {:?}", index, c_list[index]).into(),
                                            code: "1064".into()
                                        }.into())
                                    }
								},
                                // TODO swap this logic out with some evaluate()
                                &Rex::RexUnary{ref operator, rex: box Rex::Literal(ref lit)} => {
                                    if let Rex::Identifier{ref id, ref el} = c_list[index] {
                                        if el.encryption != EncryptionType::NA {
                                            self.encrypt_literal(lit, el, Some(operator))?;
                                        }

                                    } else {
                                        return Err(ZeroError::EncryptionError{
                                            message: format!("Expected identifier at column list index {}, received {:?}", index, c_list[index]).into(),
                                            code: "1064".into()
                                        }.into())
                                    }
                                },
								_ => {}
							}
						}
					},
					_ => return Err(ZeroError::EncryptionError{
                        message: format!("Unsupported INSERT syntax").into(),
                        code: "1064".into()
                    }.into())
				}
			}
			//_ => return Err(format!("Unsupported rel {:?}", rel))
		}
		Ok(())
	}

	fn visit_rex(&mut self, rex: &Rex, tt: &TupleType) -> Result<(),  Box<ZeroError>> {
		match rex {
			&Rex::BinaryExpr{box ref left, ref op, box ref right} => {
				match op {
					&Operator::AND | &Operator::OR => {
						self.visit_rex(left, tt)?;
						self.visit_rex(right, tt)?;
					}
					_ => {

						// If binary between an encrypted column and literal
						if let Some((element, literal)) = match (left, right) {
							(&Rex::Identifier{ref el, ..}, &Rex::Literal(ref l))
							| (&Rex::Literal(ref l), &Rex::Identifier{ref el, ..}) => {
								match el.encryption {
									EncryptionType::NA => None,
									_ => Some((el, l))
								}
							},
							_ => None
						} {
                            match element.encryption {
                                EncryptionType::NA => {},
                                _ => {
                                    match op {
                                        &Operator::EQ => {
                                            self.encrypt_literal(literal, element, None)?;
                                        },
                                        _ => return Err(ZeroError::EncryptionError {
                                            message: format!("Operator {:?} not supported for encrypted column {}", op, element.name).into(),
                                            code: "1064".into()
                                        }.into())
                                    }
                                }
                            }

                        // a binary between and encrypted column and unary operated literal
                        } else if let Some((element, literal, sign)) = match (left, right) {
                            (&Rex::Identifier{ref el, ..}, &Rex::RexUnary{ref operator, rex: box Rex::Literal(ref l)})
                            | (&Rex::RexUnary{ref operator, rex: box Rex::Literal(ref l)}, &Rex::Identifier{ref el, ..}, )=> {
                                match el.encryption {
                                    EncryptionType::NA => None,
                                    _ => Some((el, l, Some(operator)))
                                }
                            },
                            _ => None
                        } {
                            match element.encryption {
                                EncryptionType::NA => {},
                                _ => {
                                    match op {
                                        &Operator::EQ => {
                                            self.encrypt_literal(literal, element, sign)?;
                                        },
                                        _ => return Err(ZeroError::EncryptionError {
                                            message: format!("Operator {:?} not supported for encrypted column {}", op, element.name).into(),
                                            code: "1064".into()
                                        }.into())
                                    }
                                }
                            }

                        // a binary between two encrypted columns
						} else if let Some((left_element, right_element)) = match (left, right) {
							(&Rex::Identifier{el: ref l, ..}, &Rex::Identifier{el: ref r, .. }) => Some((l, r)),
							_ => None
						} {
							// If there is a mismatch on an operation between two identifiers, return an error
							if !(left_element.encryption == right_element.encryption && left_element.data_type == right_element.data_type) {
								return Err(ZeroError::EncryptionError{
                                    message: format!("Unsupported operation:  {}.{} [{:?}, {:?}] {:?} {}.{} [{:?}, {:?}]",
                                                     left_element.relation, left_element.name, left_element.encryption, left_element.data_type,
                                                     op,
                                                     right_element.relation, right_element.name, right_element.encryption, right_element.data_type
                                    ).into(),
                                    code: "1064".into()
                                }.into())
							} else {
								// If they do match, validate
								if left_element.encryption != EncryptionType::NA {
									match op {
										&Operator::EQ => {}, // OK,
										_ =>return Err(ZeroError::EncryptionError{
                                            message: format!("Unsupported operation:  {}.{} [{:?}, {:?}] {:?} {}.{} [{:?}, {:?}]",
                                                             left_element.relation, left_element.name, left_element.encryption, left_element.data_type,
                                                             op,
                                                             right_element.relation, right_element.name, right_element.encryption, right_element.data_type
                                            ).into(),
                                            code: "1064".into()
                                        }.into())
									}
								}
							}
						}
					}
				}
			},
            &Rex::RexFunctionCall{ref name, ref args} => {
                for a in args {
                    self.visit_rex(a, tt)?
                }
            }
//			_ => return Err(ZeroError::EncryptionError{
//                message: format!("Unsupported Expr for encryption and validation {:?}", rex).into(),
//                code: "1064".into()
//            }.into())
            _ => {}
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use config;
    use error::ZeroError;
	use std::collections::HashMap;
	use query::dialects::ansisql::*;
	use query::dialects::mysqlsql::*;
	use query::{Tokenizer, Parser, SQLWriter, Writer, ASTNode};
	use query::planner::{Planner, RelVisitor, Rel, SchemaProvider, TableMeta, ColumnMeta};
	use encrypt::{EncryptionType, NativeType};
	use std::rc::Rc;
	use std::error::Error;
	use super::super::writers::*;

	#[test]
	fn test_rel_visitor() {
        let sql = String::from("SELECT id, first_name, last_name, ssn, age, sex FROM users WHERE first_name = 'Frodo'");
		let res = parse_and_plan(sql).unwrap();
		let parsed = res.0;
		let plan = res.1;

		let value_map: HashMap<u32, Vec<u8>> = HashMap::new();
		let mut encrypt_vis = EncryptVisitor {
			valuemap: value_map
		};

		encrypt_vis.visit_rel(&plan).unwrap();

		let lit_writer = LiteralReplacingWriter{literals: &encrypt_vis.get_value_map()};
		let ansi_writer = AnsiSQLWriter{};

		let writer = SQLWriter::new(vec![&lit_writer, &ansi_writer]);

		let rewritten = writer.write(&parsed).unwrap();

		debug!("Rewritten: {}", rewritten);

		assert_eq!(rewritten, String::from("SELECT id, first_name, last_name, ssn, age, sex FROM users WHERE first_name =X'00000000000000000000000088D52F592281137DB2A0D5F0B3BD40CF004D3AA9F7'"));
	}

	#[test]
	fn test_relvis_update() {

		let sql = String::from("UPDATE users SET id = id + 100, first_name = 'NewName' WHERE id = 1 AND first_name = 'Janis'");
		let res = parse_and_plan(sql).unwrap();
		let parsed = res.0;
		let plan = res.1;

        let value_map: HashMap<u32, Vec<u8>> = HashMap::new();
		let mut encrypt_vis = EncryptVisitor {
			valuemap: value_map
		};

		encrypt_vis.visit_rel(&plan).unwrap();

		let lit_writer = LiteralReplacingWriter{literals: &encrypt_vis.get_value_map()};
		let ansi_writer = AnsiSQLWriter{};

		let writer = SQLWriter::new(vec![&lit_writer, &ansi_writer]);

		let rewritten = writer.write(&parsed).unwrap();

		println!("Rewritten: {}", rewritten);

		assert_eq!(rewritten, String::from("UPDATE users SET id = id + 100, first_name =X'00000000000000000000000080C237732C0D0E242F25422B940C09AA57634E05CD5C1D' WHERE id = 1 AND first_name =X'00000000000000000000000084C62E543E13A88D09B8993F104387DEBDCC1C41DB'"));

	}
	
	#[test]
	fn test_relvis_insert() {

		let sql = String::from("INSERT INTO users (id, first_name, last_name, ssn, age, sex) VALUES(1, 'Janis', 'Joplin', '123456789', 27, 'F')");
		let res = parse_and_plan(sql).unwrap();
		let parsed = res.0;
		let plan = res.1;

        let value_map: HashMap<u32, Vec<u8>> = HashMap::new();
		let mut encrypt_vis = EncryptVisitor {
			valuemap: value_map
		};

		encrypt_vis.visit_rel(&plan).unwrap();

		let lit_writer = LiteralReplacingWriter{literals: &encrypt_vis.get_value_map()};
		let ansi_writer = AnsiSQLWriter{};

		let writer = SQLWriter::new(vec![&lit_writer, &ansi_writer]);

		let rewritten = writer.write(&parsed).unwrap();

		debug!("Rewritten: {}", rewritten);

		assert_eq!(rewritten, String::from("INSERT INTO users ( id, first_name, last_name, ssn, age, sex) VALUES( 1,X'00000000000000000000000084C62E543E13A88D09B8993F104387DEBDCC1C41DB',X'00000000000000000000000084C83051240E026A526D31E1F98F42C07A2FDCBB497E',X'000000000000000000000000FF95730978565C563ED2E4E8EBA993D536B7E238A72F84F163',X'000000000000000000000000CEA7403D4D606B756345DB01F9B7BEC3E4F987B62AF1F0AC',X'00000000000000000000000088FBA66E3217EC0FD67F0DE527E2933E6E')"));

	}

	#[test]
	fn test_relvis_join() {
		let sql = String::from("SELECT l.id, r.id, l.first_name, r.user_id
         FROM users AS l
         JOIN user_purchases AS r ON l.id = r.user_id");
		let res = parse_and_plan(sql).unwrap();
 		let parsed = res.0;
 		let plan = res.1;

        let value_map: HashMap<u32, Vec<u8>> = HashMap::new();
		let mut encrypt_vis = EncryptVisitor {
			valuemap: value_map
		};

		encrypt_vis.visit_rel(&plan).unwrap();

		let lit_writer = LiteralReplacingWriter{literals: &encrypt_vis.get_value_map()};
		let ansi_writer = AnsiSQLWriter{};

		let writer = SQLWriter::new(vec![&lit_writer, &ansi_writer]);

		let rewritten = writer.write(&parsed).unwrap();

		debug!("Rewritten: {}", rewritten);

		assert_eq!(rewritten, String::from("SELECT l.id, r.id, l.first_name, r.user_id FROM users AS l INNER JOIN user_purchases AS r ON l.id = r.user_id"));
	}

    #[test]
    fn test_relvis_func_calls() {
        let sql = String::from("SELECT COUNT(id) FROM users");
        let res = parse_and_plan(sql).unwrap();
        let parsed = res.0;
        let plan = res.1;

        let value_map: HashMap<u32, Vec<u8>> = HashMap::new();
        let mut encrypt_vis = EncryptVisitor {
            valuemap: value_map
        };

        encrypt_vis.visit_rel(&plan).unwrap();

        let lit_writer = LiteralReplacingWriter{literals: &encrypt_vis.get_value_map()};
        let ansi_writer = AnsiSQLWriter{};

        let writer = SQLWriter::new(vec![&lit_writer, &ansi_writer]);

        let rewritten = writer.write(&parsed).unwrap();

        debug!("Rewritten: {}", rewritten);

        assert_eq!(rewritten, String::from("SELECT COUNT( id) FROM users"));
    }

	#[test]
	fn test_relvis_join_unsupported() {

		// mismatched encryption types
		let mut sql = String::from("SELECT l.id, r.id, l.first_name, r.user_id
		 FROM users AS l
		 JOIN user_purchases AS r ON l.id = r.item_code");
		let mut plan = parse_and_plan(sql).unwrap().1;

        let value_map: HashMap<u32, Vec<u8>> = HashMap::new();
		let mut encrypt_vis = EncryptVisitor {
			valuemap: value_map
		};

        assert_eq!(encrypt_vis.visit_rel(&plan).err().unwrap().to_string(), String::from("Unsupported operation:  l.id [NA, U64] EQ r.item_code [AES, U64]"));

        // two unencryped columns
		sql = String::from("SELECT l.id, r.id, l.first_name, r.user_id
		 FROM users AS l
		 JOIN user_purchases AS r ON l.id > r.user_id");
		plan = parse_and_plan(sql).unwrap().1;

		assert_eq!(encrypt_vis.visit_rel(&plan).is_ok(), true);

		// unsupported operator on two encrypted columns
		sql = String::from("SELECT l.id, r.id, l.first_name, r.user_id
		 FROM users AS l
		 JOIN user_purchases AS r ON l.age > r.item_code");
		plan = parse_and_plan(sql).unwrap().1;
		assert_eq!(encrypt_vis.visit_rel(&plan).err().unwrap().to_string(), String::from("Unsupported operation:  l.age [AES, U64] GT r.item_code [AES, U64]"));

    }

    #[test]
    fn test_relvis_rel_as_rex() {

        let sql = String::from("SELECT id FROM users WHERE id = (SELECT id FROM users)");
        let res = parse_and_plan(sql).unwrap();
        let plan = res.1;

        let value_map: HashMap<u32, Vec<u8>> = HashMap::new();
        let mut encrypt_vis = EncryptVisitor {
            valuemap: value_map
        };

        encrypt_vis.visit_rel(&plan).unwrap();

    }

	fn parse_and_plan(sql: String) -> Result<(ASTNode, Rel), Box<ZeroError>> {
		let provider = DummyProvider{};

		let ansi = AnsiSQLDialect::new();
		let dialect = MySQLDialect::new(&ansi);

		let parsed = sql.tokenize(&dialect)?.parse()?;

		let s = String::from("zero");
		let default_schema = Some(&s);
		let planner = Planner::new(default_schema, Rc::new(provider));
		let plan = planner.sql_to_rel(&parsed)?.unwrap();
		Ok((parsed, plan))

	}

	struct DummyProvider {}
    impl SchemaProvider for DummyProvider {
        fn get_table_meta(&self, schema: &String, table: &String) -> Result<Option<Rc<TableMeta>>, Box<ZeroError>> {

            let rc = match (schema as &str, table as &str) {
                ("zero", "users") => {
                    Some(Rc::new(TableMeta {
                        columns: vec![
                            ColumnMeta {name: String::from("id"), native_type: NativeType::U64,
                                        encryption: EncryptionType::NA,
                                        key: [0u8; 32]},
                            ColumnMeta {name: String::from("first_name"), native_type: NativeType::Varchar(50),
                                        encryption: EncryptionType::AES,
                                        key: [0u8; 32]},
                            ColumnMeta {name: String::from("last_name"), native_type: NativeType::Varchar(50),
                                        encryption: EncryptionType::AES,
                                        key: [0u8; 32]},
                            ColumnMeta {name: String::from("ssn"), native_type: NativeType::Varchar(50),
                                        encryption: EncryptionType::AES,
                                        key: [0u8; 32]},
                            ColumnMeta {name: String::from("age"), native_type: NativeType::U64,
                                        encryption: EncryptionType::AES,
                                        key: [0u8; 32]},
                            ColumnMeta {name: String::from("sex"), native_type: NativeType::Varchar(50),
                                        encryption: EncryptionType::AES,
                                        key: [0u8; 32]},
                        ]
                    }))
                },
                ("zero", "user_purchases") => {
                    Some(Rc::new(TableMeta {
                        columns: vec![
                            ColumnMeta {name: String::from("id"), native_type: NativeType::U64,
                                        encryption: EncryptionType::NA,
                                        key: [0u8; 32]},
                            ColumnMeta {name: String::from("user_id"), native_type: NativeType::U64,
                                        encryption: EncryptionType::NA,
                                        key: [0u8; 32]},
                            ColumnMeta {name: String::from("item_code"), native_type: NativeType::U64,
                                        encryption: EncryptionType::AES,
                                        key: [0u8; 32]},
                            ColumnMeta {name: String::from("amount"), native_type: NativeType::F64,
                                        encryption: EncryptionType::AES,
                                        key: [0u8; 32]},
                        ]
                    }))
                },
                _ => None
            };
            Ok(rc)
        }

    }

}
