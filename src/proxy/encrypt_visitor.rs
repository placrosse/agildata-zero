use query::planner::{Rel, Rex, RelVisitor, TupleType, HasTupleType};
use query::{Operator, LiteralExpr};
use std::collections::HashMap;
use encrypt::*;
use error::ZeroError;
#[derive(Debug)]
pub struct EncryptVisitor {
	pub valuemap: HashMap<u32, Result<Vec<u8>, Box<ZeroError>>>
}

impl EncryptVisitor {
	pub fn get_value_map(&self) -> &HashMap<u32, Result<Vec<u8>, Box<ZeroError>>> {
		&self.valuemap
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
													self.valuemap.insert(i.clone(), val.encrypt(&el.encryption, &el.key));
												},
												&LiteralExpr::LiteralString(ref i, ref val) => {
													self.valuemap.insert(i.clone(), val.clone().encrypt(&el.encryption, &el.key));
												}
												_ => return Err(ZeroError::EncryptionError{
                                                    message: format!("Unsupported value type {:?} for encryption", lit).into(),
                                                    code: "1064".into()
                                                }.into())
											}
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
											match literal {
												&LiteralExpr::LiteralLong(ref i, ref val) => {
													self.valuemap.insert(i.clone(), val.encrypt(&element.encryption, &element.key));
												},
												&LiteralExpr::LiteralString(ref i, ref val) => {
													self.valuemap.insert(i.clone(), val.clone().encrypt(&element.encryption, &element.key));
												}
												_ => return  Err(ZeroError::EncryptionError{
                                                    message: format!("Unsupported value type {:?} for encryption", literal).into(),
                                                    code: "1064".into()
                                                }.into())
											}
										},
										_ => return  Err(ZeroError::EncryptionError{
                                            message: format!("Operator {:?} not supported for encrypted column {}", op, element.name).into(),
                                            code: "1064".into()
                                        }.into())
									}
								}
							}

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
			_ => {} // TODO
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

		let value_map: HashMap<u32, Result<Vec<u8>, Box<ZeroError>>> = HashMap::new();
		let mut encrypt_vis = EncryptVisitor {
			valuemap: value_map
		};

		encrypt_vis.visit_rel(&plan).unwrap();

		let lit_writer = LiteralReplacingWriter{literals: &encrypt_vis.get_value_map()};
		let ansi_writer = AnsiSQLWriter{};

		let writer = SQLWriter::new(vec![&lit_writer, &ansi_writer]);

		let rewritten = writer.write(&parsed).unwrap();

		println!("Rewritten: {}", rewritten);

		assert_eq!(rewritten, String::from("SELECT id, first_name, last_name, ssn, age, sex FROM users WHERE first_name =X'00000000000000000000000088D52F592281137DB2A0D5F0B3BD40CF004D3AA9F7'"));
	}

	#[test]
	fn test_relvis_insert() {

		let sql = String::from("INSERT INTO users (id, first_name, last_name, ssn, age, sex) VALUES(1, 'Janis', 'Joplin', '123456789', 27, 'F')");
		let res = parse_and_plan(sql).unwrap();
		let parsed = res.0;
		let plan = res.1;

		let value_map: HashMap<u32, Result<Vec<u8>, Box<ZeroError>>> = HashMap::new();
		let mut encrypt_vis = EncryptVisitor {
			valuemap: value_map
		};

		encrypt_vis.visit_rel(&plan).unwrap();

		let lit_writer = LiteralReplacingWriter{literals: &encrypt_vis.get_value_map()};
		let ansi_writer = AnsiSQLWriter{};

		let writer = SQLWriter::new(vec![&lit_writer, &ansi_writer]);

		let rewritten = writer.write(&parsed).unwrap();

		println!("Rewritten: {}", rewritten);

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

		let value_map: HashMap<u32, Result<Vec<u8>, Box<ZeroError>>> = HashMap::new();
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

		let value_map: HashMap<u32, Result<Vec<u8>, Box<ZeroError>>> = HashMap::new();
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

	fn parse_and_plan(sql: String) -> Result<(ASTNode, Rel), Box<ZeroError>> {
		let provider = DummyProvider{};

		let ansi = AnsiSQLDialect::new();
		let dialect = MySQLDialect::new(&ansi);

		let parsed = sql.tokenize(&dialect)?.parse()?;

		let s = String::from("zero");
		let default_schema = Some(&s);
		let planner = Planner::new(default_schema, &provider);
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
