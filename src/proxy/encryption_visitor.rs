use config::{Config, TConfig};
use encrypt::Encrypt;

use parser::visitor::*;
use parser::sql_parser::{SQLExpr, LiteralExpr, SQLOperator};

use std::collections::HashMap;

#[derive(Debug)]
pub struct EncryptionVisitor<'a> {
	pub config: &'a Config,
	pub valuemap: HashMap<u32, Option<Vec<u8>>>
}

impl<'a> EncryptionVisitor<'a> {
	fn is_identifier(&self, expr: &SQLExpr) -> bool {
		match expr {
			&SQLExpr::SQLIdentifier(_) => true,
			_ => false
		}
	}

	fn is_literal(&self, expr: &SQLExpr) -> bool {
		match expr {
			&SQLExpr::SQLLiteral(_) => true,
			_ => false
		}
	}

	pub fn get_value_map(&self) -> &HashMap<u32, Option<Vec<u8>>> {
		&self.valuemap
	}
}

impl<'a> SQLExprVisitor for EncryptionVisitor<'a> {
	fn visit_sql_expr(&mut self, expr: &SQLExpr) {
		match expr {
			&SQLExpr::SQLSelect{box ref expr_list, ref relation, ref selection, ref order} => {
				self.visit_sql_expr(expr_list);
				match relation {
					&Some(box ref expr) => self.visit_sql_expr(expr),
					&None => {}
				}
				match selection {
					&Some(box ref expr) => self.visit_sql_expr(expr),
					&None => {}
				}
				match order {
					&Some(box ref expr) => self.visit_sql_expr(expr),
					&None => {}
				}
			},
			&SQLExpr::SQLUpdate{box ref table, box ref assignments, ref selection} => {
				self.visit_sql_expr(table);
				self.visit_sql_expr(assignments);

				match selection {
					&Some(box ref expr) => self.visit_sql_expr(expr),
					&None => {}
				}
			},
			&SQLExpr::SQLInsert{box ref table, box ref column_list, box ref values_list} => {
				let table = match table {
					&SQLExpr::SQLIdentifier(ref v) => v,
					_ => panic!("Illegal")
				};
				match column_list {
					&SQLExpr::SQLExprList(ref columns) => {
						match values_list {
							&SQLExpr::SQLExprList(ref values) => {
								for (i, e) in columns.iter().enumerate() {
									match e {
										&SQLExpr::SQLIdentifier(ref name) => {
											let col = self.config.get_column_config(&String::from("babel"), table, name);
											if col.is_some() {
												match values[i] {
													SQLExpr::SQLLiteral(ref l) => match l {
														&LiteralExpr::LiteralLong(ref i, ref val) => {
															self.valuemap.insert(i.clone(), val.encrypt(&col.unwrap().encryption));
														},
														&LiteralExpr::LiteralString(ref i, ref val) => {
															self.valuemap.insert(i.clone(), val.clone().encrypt(&col.unwrap().encryption));
														}
														_ => panic!("Unsupported value type {:?}", l)
													},
													_ => {}
												}
											}
										},
										_ => panic!("Illegal")
									}
								}
							},
							_ => panic!("Illegal")
						}

					},
					_ => panic!("Illegal")
				}
			},
			&SQLExpr::SQLExprList(ref vector) => {
				for e in vector {
					self.visit_sql_expr(&e);
				}
			},
			&SQLExpr::SQLBinary{box ref left, ref op, box ref right} => {
				//println!("HERE");
				// ident = lit
				match op {
					// TODO Messy...clean up
					// TODO should check left and right
					&SQLOperator::EQ => {
						if self.is_identifier(left) && self.is_literal(right) {
							let ident = match left {
								&SQLExpr::SQLIdentifier(ref v) => v,
								_ => panic!("Unreachable")
							};
							let col = self.config.get_column_config(&String::from("babel"), &String::from("users"), ident);
							if col.is_some() {
								match right {
									&SQLExpr::SQLLiteral(ref l) => match l {
										&LiteralExpr::LiteralLong(ref i, ref val) => {
											self.valuemap.insert(i.clone(), val.encrypt(&col.unwrap().encryption));
										},
										&LiteralExpr::LiteralString(ref i, ref val) => {
											self.valuemap.insert(i.clone(), val.clone().encrypt(&col.unwrap().encryption));
										}
										_ => panic!("Unsupported value type {:?}", l)
									},
									_ => panic!("Unreachable")
								}
							}
						} else if self.is_identifier(&right) && self.is_literal(&left) {
							panic!("Syntax literal = identifier not currently supported")
						}
					},
					_ => {}
				}

				self.visit_sql_expr(left);
				self.visit_sql_expr(right);
			},
			&SQLExpr::SQLLiteral(ref lit) => {
				self.visit_sql_lit_expr(lit);
			},
			&SQLExpr::SQLAlias{box ref expr, box ref alias} => {
				self.visit_sql_expr(&expr);
				self.visit_sql_expr(&alias);
			},
			&SQLExpr::SQLIdentifier(_) => {
				// TODO end of visit arm
			},
			&SQLExpr::SQLNested(ref expr) => {
				self.visit_sql_expr(expr);
			},
			&SQLExpr::SQLUnary{ref operator, box ref expr} => {
				self.visit_sql_operator(operator);
				self.visit_sql_expr(expr);
			},
			&SQLExpr::SQLOrderBy{box ref expr, ..} => {
				self.visit_sql_expr(expr);
				// TODO bool
			},
			&SQLExpr::SQLJoin{box ref left, box ref right, ref on_expr, ..} => {
				self.visit_sql_expr(left);
				// TODO visit join type
				self.visit_sql_expr(right);
				match on_expr {
					&Some(box ref expr) => self.visit_sql_expr(expr),
					&None => {}
				}
			},
			&SQLExpr::SQLUnion{box ref left, box ref right, ..} => {
				self.visit_sql_expr(left);
				// TODO union type
				self.visit_sql_expr(right);
			},
			_ => panic!("Unsupported expr {:?}", expr)
		}
	}

	fn visit_sql_lit_expr(&mut self, _lit: &LiteralExpr) {
		//do nothing
	}

	fn visit_sql_operator(&mut self, _op: &SQLOperator) {
		// do nothing
	}
}

pub fn walk(visitor: &mut SQLExprVisitor, e: &SQLExpr) {
	visitor.visit_sql_expr(e);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
	use parser::sql_parser::AnsiSQLParser;
	use parser::sql_writer::*;
	use super::super::writers::*;
    use config;

	#[test]
	fn test_visitor() {
		let parser = AnsiSQLParser {};
		let sql = "SELECT age, first_name, last_name FROM users WHERE age = 1";
		let parsed = parser.parse(sql).unwrap();

		let config = config::parse_config("example-babel-config.xml");
		let value_map: HashMap<u32, Option<Vec<u8>>> = HashMap::new();
		let mut encrypt_vis = EncryptionVisitor {
			config: &config,
			valuemap: value_map
		};
		 walk(&mut encrypt_vis, &parsed);

		 println!("HERE {:#?}", encrypt_vis);
	}

	#[test]
	fn test_vis_insert() {
		let parser = AnsiSQLParser {};
		let sql = "INSERT INTO users (id, first_name, last_name, ssn, age, sex) VALUES(1, 'Janis', 'Joplin', '123456789', 27, 'F')";
		let parsed = parser.parse(sql).unwrap();

		let config = config::parse_config("example-babel-config.xml");
		let value_map: HashMap<u32, Option<Vec<u8>>> = HashMap::new();
		let mut encrypt_vis = EncryptionVisitor {
			config: &config,
			valuemap: value_map
		};
		 walk(&mut encrypt_vis, &parsed);

		 println!("HERE {:#?}", encrypt_vis);
	}

	#[test]
	fn test_vis_update() {
		let parser = AnsiSQLParser {};
		let sql = "UPDATE users SET age = 31, ssn = '987654321' WHERE first_name = 'Janis' AND last_name = 'Joplin'";
		let parsed = parser.parse(sql).unwrap();

		let config = config::parse_config("example-babel-config.xml");
		let value_map: HashMap<u32, Option<Vec<u8>>> = HashMap::new();
		let mut encrypt_vis = EncryptionVisitor {
			config: &config,
			valuemap: value_map
		};
		walk(&mut encrypt_vis, &parsed);

		println!("HERE {:#?}", encrypt_vis);

		let lit_writer = LiteralReplacingWriter{literals: &encrypt_vis.get_value_map()};

		let writer = SQLWriter::new(vec![&lit_writer]);

		let rewritten = writer.write(&parsed).unwrap();

		println!("Rewritten: {}", rewritten);

	}
}
