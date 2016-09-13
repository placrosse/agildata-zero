use query::planner::{Rel, Rex, RelVisitor, TupleType, HasTupleType};
use query::{Operator, LiteralExpr};
use std::collections::HashMap;
use std::error::Error;
use encrypt::*;

#[derive(Debug)]
pub struct EncryptVisitor {
	pub valuemap: HashMap<u32, Result<Vec<u8>, Box<Error>>>
}

impl EncryptVisitor {
	pub fn get_value_map(&self) -> &HashMap<u32, Result<Vec<u8>, Box<Error>>> {
		&self.valuemap
	}
}


impl RelVisitor for EncryptVisitor  {
	fn visit_rel(&mut self, rel: &Rel) -> Result<(),  Box<Error>> {
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
			&Rel::Insert{ref table, box ref columns, box ref values, ..} => {
				match (columns, values) {
					(&Rex::RexExprList(ref c_list), &Rex::RexExprList(ref v_list)) => {
						for (index, v) in v_list.iter().enumerate() {
							match v {
								&Rex::Literal(ref lit) => {
									if let Rex::Identifier{ref id, ref el} = c_list[index] {
										if el.encryption != EncryptionType::NA {
											match lit {
												&LiteralExpr::LiteralLong(ref i, ref val) => {
													self.valuemap.insert(i.clone(), val.encrypt(&el.encryption));
												},
												&LiteralExpr::LiteralString(ref i, ref val) => {
													self.valuemap.insert(i.clone(), val.clone().encrypt(&el.encryption));
												}
												_ => return Err(format!("Unsupported value type {:?} for encryption", lit).into())
											}
										}

									} else {
										return Err(format!("Expected identifier at column list index {}, received {:?}", index, c_list[index]).into())
									}
								},
								_ => {}
							}
						}
					},
					_ => return Err(String::from("Unsupported INSERT syntax").into())
				}
			}
			//_ => return Err(format!("Unsupported rel {:?}", rel))
		}
		Ok(())
	}

	fn visit_rex(&mut self, rex: &Rex, tt: &TupleType) -> Result<(),  Box<Error>> {
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
											match literal {
												&LiteralExpr::LiteralLong(ref i, ref val) => {
													self.valuemap.insert(i.clone(), val.encrypt(&element.encryption));
												},
												&LiteralExpr::LiteralString(ref i, ref val) => {
													self.valuemap.insert(i.clone(), val.clone().encrypt(&element.encryption));
												}
												_ => return Err(format!("Unsupported value type {:?} for encryption", literal).into())
											}
										},
										_ => return Err(format!("Operator {:?} not supported for encrypted column {}", op, element.name).into())
									}
								}
							}

						} else if let Some((left_element, right_element)) = match (left, right) {
							(&Rex::Identifier{el: ref l, ..}, &Rex::Identifier{el: ref r, .. }) => Some((l, r)),
							_ => None
						} {
							// If there is a mismatch on an operation between two identifiers, return an error
							if !(left_element.encryption == right_element.encryption && left_element.data_type == right_element.data_type) {
								return Err(format!(
									"Unsupported operation:  {}.{} [{:?}, {:?}] {:?} {}.{} [{:?}, {:?}]",
									left_element.relation, left_element.name, left_element.encryption, left_element.data_type,
									op,
									right_element.relation, right_element.name, right_element.encryption, right_element.data_type
								).into())
							} else {
								// If they do match, validate
								if left_element.encryption != EncryptionType::NA {
									match op {
										&Operator::EQ => {}, // OK,
										_ => return Err(format!(
											"Unsupported operation:  {}.{} [{:?}, {:?}] {:?} {}.{} [{:?}, {:?}]",
											left_element.relation, left_element.name, left_element.encryption, left_element.data_type,
											op,
											right_element.relation, right_element.name, right_element.encryption, right_element.data_type
										).into())
									}
								}
							}
						}
					}
				}
			},
			_ => {} // TODO
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use config;
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

		let value_map: HashMap<u32, Result<Vec<u8>, Box<Error>>> = HashMap::new();
		let mut encrypt_vis = EncryptVisitor {
			valuemap: value_map
		};

		encrypt_vis.visit_rel(&plan).unwrap();

		let lit_writer = LiteralReplacingWriter{literals: &encrypt_vis.get_value_map()};
		let ansi_writer = AnsiSQLWriter{};

		let writer = SQLWriter::new(vec![&lit_writer, &ansi_writer]);

		let rewritten = writer.write(&parsed).unwrap();

		println!("Rewritten: {}", rewritten);

		assert_eq!(rewritten, String::from("SELECT id, first_name, last_name, ssn, age, sex FROM users WHERE first_name =X'0000000000000000000000001634B73C83460779A2179A0134D8F22A82F7EF5FD4'"));
	}

	#[test]
	fn test_relvis_insert() {

		let sql = String::from("INSERT INTO users (id, first_name, last_name, ssn, age, sex) VALUES(1, 'Janis', 'Joplin', '123456789', 27, 'F')");
		let res = parse_and_plan(sql).unwrap();
		let parsed = res.0;
		let plan = res.1;

		let value_map: HashMap<u32, Result<Vec<u8>, Box<Error>>> = HashMap::new();
		let mut encrypt_vis = EncryptVisitor {
			valuemap: value_map
		};

		encrypt_vis.visit_rel(&plan).unwrap();

		let lit_writer = LiteralReplacingWriter{literals: &encrypt_vis.get_value_map()};
		let ansi_writer = AnsiSQLWriter{};

		let writer = SQLWriter::new(vec![&lit_writer, &ansi_writer]);

		let rewritten = writer.write(&parsed).unwrap();

		println!("Rewritten: {}", rewritten);

		assert_eq!(rewritten, String::from("INSERT INTO users ( id, first_name, last_name, ssn, age, sex) VALUES( 1,X'0000000000000000000000001A27B6319FFE5092BC87E3F8A8D59DEFC1CEFC35AA',X'0000000000000000000000001A29A8348545EF85B58E92DFE127DD4942CB5622F01F',X'0000000000000000000000006174EB6CD91D111BD46235D350E485AA0BA1DCCBF442BE0C8A',X'0000000000000000000000005046D858EC2B26384619838AA28512F4082E25592BD37DBC',X'00000000000000000000000016A2122F7E9A44212C8CD2FC4185084959')"));

	}

	#[test]
	fn test_relvis_join() {
		let sql = String::from("SELECT l.id, r.id, l.first_name, r.user_id
         FROM users AS l
         JOIN user_purchases AS r ON l.id = r.user_id");
		let res = parse_and_plan(sql).unwrap();
 		let parsed = res.0;
 		let plan = res.1;

		let value_map: HashMap<u32, Result<Vec<u8>, Box<Error>>> = HashMap::new();
		let mut encrypt_vis = EncryptVisitor {
			valuemap: value_map
		};

		encrypt_vis.visit_rel(&plan).unwrap();

		let lit_writer = LiteralReplacingWriter{literals: &encrypt_vis.get_value_map()};
		let ansi_writer = AnsiSQLWriter{};

		let writer = SQLWriter::new(vec![&lit_writer, &ansi_writer]);

		let rewritten = writer.write(&parsed).unwrap();

		println!("Rewritten: {}", rewritten);

		assert_eq!(rewritten, String::from("SELECT l.id, r.id, l.first_name, r.user_id FROM users AS l INNER JOIN user_purchases AS r ON l.id = r.user_id"));
	}

	#[test]
	fn test_relvis_join_unsupported() {

		// mismatched encryption types
		let mut sql = String::from("SELECT l.id, r.id, l.first_name, r.user_id
		 FROM users AS l
		 JOIN user_purchases AS r ON l.id = r.item_code");
		let mut plan = parse_and_plan(sql).unwrap().1;

		let value_map: HashMap<u32, Result<Vec<u8>, Box<Error>>> = HashMap::new();
		let mut encrypt_vis = EncryptVisitor {
			valuemap: value_map
		};

        assert_eq!(encrypt_vis.visit_rel(&plan).err().unwrap().description(), String::from("Unsupported operation:  l.id [NA, U64] EQ r.item_code [AES, U64]"));

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
		assert_eq!(encrypt_vis.visit_rel(&plan).err().unwrap().description(), String::from("Unsupported operation:  l.age [AES, U64] GT r.item_code [AES, U64]"));


	}

	fn parse_and_plan(sql: String) -> Result<(ASTNode, Rel), Box<Error>> {
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
        fn get_table_meta(&self, schema: &String, table: &String) -> Result<Option<Rc<TableMeta>>, Box<Error>> {

            let rc = match (schema as &str, table as &str) {
                ("zero", "users") => {
                    Some(Rc::new(TableMeta {
                        columns: vec![
                            ColumnMeta {name: String::from("id"), native_type: NativeType::U64, encryption: EncryptionType::NA},
                            ColumnMeta {name: String::from("first_name"), native_type: NativeType::Varchar(50), encryption: EncryptionType::AES},
                            ColumnMeta {name: String::from("last_name"), native_type: NativeType::Varchar(50), encryption: EncryptionType::AES},
                            ColumnMeta {name: String::from("ssn"), native_type: NativeType::Varchar(50), encryption: EncryptionType::AES},
                            ColumnMeta {name: String::from("age"), native_type: NativeType::U64, encryption: EncryptionType::AES},
                            ColumnMeta {name: String::from("sex"), native_type: NativeType::Varchar(50), encryption: EncryptionType::AES},

                        ]
                    }))
                },
                ("zero", "user_purchases") => {
                    Some(Rc::new(TableMeta {
                        columns: vec![
                            ColumnMeta {name: String::from("id"), native_type: NativeType::U64, encryption: EncryptionType::NA},
                            ColumnMeta {name: String::from("user_id"), native_type: NativeType::U64, encryption: EncryptionType::NA},
                            ColumnMeta {name: String::from("item_code"), native_type: NativeType::U64, encryption: EncryptionType::AES},
                            ColumnMeta {name: String::from("amount"), native_type: NativeType::F64, encryption: EncryptionType::AES}
                        ]
                    }))
                },
                _ => None
            };
            Ok(rc)
        }

    }

}
