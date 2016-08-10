extern crate config;
use self::config::{Config, TConfig};
extern crate encrypt;
use self::encrypt::{Encrypt, EncryptionType};

extern crate sql_parser;
use sql_parser::visitor::*;
use sql_parser::sql_parser::{SQLExpr, LiteralExpr, SQLOperator, SQLUnionType, SQLJoinType};

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
											let mut col = self.config.get_column_config(&String::from("babel"), table, name);
											if (col.is_some()) {
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
						if (self.is_identifier(left) && self.is_literal(right)) {
							let ident = match left {
								&SQLExpr::SQLIdentifier(ref v) => v,
								_ => panic!("Unreachable")
							};
							let mut col = self.config.get_column_config(&String::from("babel"), &String::from("users"), ident);
							if (col.is_some()) {
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
						} else if (self.is_identifier(&right) && self.is_literal(&left)) {
							panic!("Syntax literal = identifier not currently supported")
						}

						// self.visit_sql_expr(left);
						// self.visit_sql_expr(right);
					},
					_ => {}
				}
			},
			&SQLExpr::SQLLiteral(ref lit) => {
				self.visit_sql_lit_expr(lit);
			},
			&SQLExpr::SQLAlias{box ref expr, box ref alias} => {
				self.visit_sql_expr(&expr);
				self.visit_sql_expr(&alias);
			},
			&SQLExpr::SQLIdentifier(ref id) => {
				// TODO end of visit arm
			},
			&SQLExpr::SQLNested(ref expr) => {
				self.visit_sql_expr(expr);
			},
			&SQLExpr::SQLUnary{ref operator, box ref expr} => {
				self.visit_sql_operator(operator);
				self.visit_sql_expr(expr);
			},
			&SQLExpr::SQLOrderBy{box ref expr, ref is_asc} => {
				self.visit_sql_expr(expr);
				// TODO bool
			},
			&SQLExpr::SQLJoin{box ref left, ref join_type, box ref right, ref on_expr} => {
				self.visit_sql_expr(left);
				// TODO visit join type
				self.visit_sql_expr(right);
				match on_expr {
					&Some(box ref expr) => self.visit_sql_expr(expr),
					&None => {}
				}
			},
			&SQLExpr::SQLUnion{box ref left, ref union_type, box ref right} => {
				self.visit_sql_expr(left);
				// TODO union type
				self.visit_sql_expr(right);
			},
			//_ => panic!("Unsupported expr {:?}", expr)
		}
	}

	fn visit_sql_lit_expr(&mut self, lit: &LiteralExpr) {
		panic!("visit_sql_lit_expr() not implemented");
	}

	fn visit_sql_operator(&mut self, op: &SQLOperator) {
		panic!("visit_sql_operator() not implemented");
	}
}

pub fn walk(visitor: &mut SQLExprVisitor, e: &SQLExpr) {
	visitor.visit_sql_expr(e);
}

#[cfg(test)]
mod tests {
	use super::sql_parser::sql_parser::AnsiSQLParser;
	use super::*;
	use std::collections::HashMap;

	#[test]
	fn test_visitor() {
		let parser = AnsiSQLParser {};
		let sql = "SELECT age, first_name, last_name FROM users WHERE age = 1";
		let parsed = parser.parse(sql).unwrap();

		let config = super::config::parse_config("../config/src/demo-client-config.xml");
		let mut valueMap: HashMap<u32, Option<Vec<u8>>> = HashMap::new();
		let mut encrypt_vis = EncryptionVisitor {
			config: &config,
			valuemap: valueMap
		};
		 walk(&mut encrypt_vis, &parsed);

		 println!("HERE {:#?}", encrypt_vis);
	}

	#[test]
	fn test_vis_insert() {
		let parser = AnsiSQLParser {};
		let sql = "INSERT INTO users (id, first_name, last_name, ssn, age, sex) VALUES(1, 'Janis', 'Joplin', '123456789', 27, 'F')";
		let parsed = parser.parse(sql).unwrap();

		let config = super::config::parse_config("../src/example-babel-config.xml");
		let mut valueMap: HashMap<u32, Option<Vec<u8>>> = HashMap::new();
		let mut encrypt_vis = EncryptionVisitor {
			config: &config,
			valuemap: valueMap
		};
		 walk(&mut encrypt_vis, &parsed);

		 println!("HERE {:#?}", encrypt_vis);
	}
}
