use query::planner::{Rel, Rex, RelVisitor, TupleType, HasTupleType};
use query::{Operator, LiteralExpr};
use std::collections::HashMap;
use std::error::Error;
use encrypt::*;

#[derive(Debug)]
pub struct EncryptVisitor {
	pub valuemap: HashMap<u32, Result<Vec<u8>, Box<Error>>>
}

impl RelVisitor for EncryptVisitor  {
	fn visit_rel(&mut self, rel: &Rel) -> Result<(), String> {
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
			&Rel::Dual{..} => {},
			&Rel::Insert{ref table, box ref columns, box ref values, ref tt} => {
				match (columns, values) {
					(&Rex::RexExprList(ref c_list), &Rex::RexExprList(ref v_list)) => {
						for (index, v) in v_list.iter().enumerate() {
							match v {
								&Rex::Literal(ref lit) => {
									if let Rex::Identifier{ref id, ref el} = c_list[index] {
										match lit {
											&LiteralExpr::LiteralLong(ref i, ref val) => {
												self.valuemap.insert(i.clone(), val.encrypt(&el.encryption));
											},
											&LiteralExpr::LiteralString(ref i, ref val) => {
												self.valuemap.insert(i.clone(), val.clone().encrypt(&el.encryption));
											}
											_ => return Err(format!("Unsupported value type {:?} for encryption", lit))
										}
									} else {
										return Err(format!("Expected identifier at column list index {}, received {:?}", index, c_list[index]))
									}
								},
								_ => {}
							}
						}
					},
					_ => return Err(String::from("Unsupported INSERT syntax"))
				}
			}
		}
		Ok(())
	}

	fn visit_rex(&mut self, rex: &Rex, tt: &TupleType) -> Result<(), String> {
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
												_ => return Err(format!("Unsupported value type {:?} for encryption", literal))
											}
										},
										_ => return Err(format!("Operator {:?} not supported for encrypted column {}", op, element.name))
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
	use query::{Tokenizer, Parser};
	use query::planner::{Planner, RelVisitor};
	use std::error::Error;

	#[test]
	fn test_rel_visitor() {
		let config = config::parse_config("zero-config.xml");

        let ansi = AnsiSQLDialect::new();
        let dialect = MySQLDialect::new(&ansi);

        let sql = String::from("SELECT id, first_name, last_name, ssn, age, sex FROM users WHERE first_name = 'Frodo'");
        let parsed = sql.tokenize(&dialect).unwrap().parse().unwrap();

        let s = String::from("zero");
        let default_schema = Some(&s);
        let planner = Planner{default_schema: default_schema, config: &config};

        let plan = planner.sql_to_rel(&parsed).unwrap().unwrap();

		let value_map: HashMap<u32, Result<Vec<u8>, Box<Error>>> = HashMap::new();
		let mut encrypt_vis = EncryptVisitor {
			valuemap: value_map
		};

		encrypt_vis.visit_rel(&plan);

		 println!("HERE {:#?}", encrypt_vis);
	}

	// #[test]
	// fn test_vis_insert() {
	//
	// 	let ansi = AnsiSQLDialect::new();
	// 	let dialect = MySQLDialect::new(&ansi);
	//
    //     let sql = String::from("INSERT INTO users (id, first_name, last_name, ssn, age, sex) VALUES(1, 'Janis', 'Joplin', '123456789', 27, 'F')");
	// 	let parsed = sql.tokenize(&dialect).unwrap().parse().unwrap();
	//
    //     let config = config::parse_config("zero-config.xml");
	// 	let value_map: HashMap<u32, Result<Vec<u8>, Box<Error>>> = HashMap::new();
	// 	let mut encrypt_vis = EncryptionVisitor {
	// 		config: &config,
	// 		valuemap: value_map
	// 	};
	// 	 walk(&mut encrypt_vis, &parsed);
	//
	// 	 println!("HERE {:#?}", encrypt_vis);
	// }

}
