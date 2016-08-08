extern crate config;
use self::config::{EncryptionType, Config, TConfig};
use super::visitor::*;
use super::sql_parser::{SQLExpr, LiteralExpr, SQLOperator, SQLUnionType, SQLJoinType};
use std::collections::HashMap;

trait Encrypt {
	fn encrypt(self, scheme: &EncryptionType) -> String;
}

impl Encrypt for u64 {
	fn encrypt(self, scheme: &EncryptionType) -> String {
		String::from("fooness")
	}
}

#[derive(Debug)]
struct EncryptionVisitor {
	config: Config,
	valuemap: HashMap<u32, String>
}

impl SQLExprVisitor for EncryptionVisitor {
	fn visit_sql_expr(&mut self, expr: SQLExpr) {
		match expr {
			SQLExpr::SQLSelect{expr_list, relation, selection, order} => {
				self.visit_sql_expr(*expr_list);
				if !relation.is_none() {
					self.visit_sql_expr(*relation.unwrap());
				}
				if !selection.is_none() {
					self.visit_sql_expr(*selection.unwrap());
				}
				if !order.is_none() {
					self.visit_sql_expr(*order.unwrap());
				}
			},
			SQLExpr::SQLExprList(vector) => {
				for e in vector {
					self.visit_sql_expr(e);
				}
			},
			SQLExpr::SQLBinary{left, op, right} => {
				//println!("HERE");
				match op {
					// TODO Messy...clean up
					SQLOperator::EQ => {
						let mut ret= match left {
							box SQLExpr::SQLIdentifier(v) => {
								match right {
									box SQLExpr::SQLLiteral(l) => {
										let mut col = self.config.get_column_config(&String::from("babel"), &String::from("users"), &String::from("age"));
										if (col.is_some()) {
											match l {
												LiteralExpr::LiteralLong(i,value) => {
													// TODO Lifetimes of immutable self...
													self.valuemap.insert(i, value.encrypt(&col.unwrap().encryption));
												}
												_ => panic!("Unsupported")
											}
										}
										true
									},
									_ => false
								}
							},
							_ => false
						};
					}
					_ => {}
				}

				// self.visit_sql_expr(*left);
				// self.visit_sql_expr(*right);
			},
			SQLExpr::SQLLiteral(lit) => {
				self.visit_sql_lit_expr(lit);
			},
			SQLExpr::SQLAlias{expr, alias} => {
				self.visit_sql_expr(*expr);
				self.visit_sql_expr(*alias);
			},
			SQLExpr::SQLIdentifier(id) => {
				// TODO end of visit arm
			},
			SQLExpr::SQLNested(expr) => {
				self.visit_sql_expr(*expr);
			},
			SQLExpr::SQLUnary{operator, expr} => {
				self.visit_sql_operator(operator);
				self.visit_sql_expr(*expr);
			},
			SQLExpr::SQLOrderBy{expr, is_asc} => {
				self.visit_sql_expr(*expr);
				// TODO bool
			},
			SQLExpr::SQLJoin{left, join_type, right, on_expr} => {
				self.visit_sql_expr(*left);
				// TODO visit join type
				self.visit_sql_expr(*right);
				if !on_expr.is_none() {
					self.visit_sql_expr(*on_expr.unwrap());
				}
			},
			SQLExpr::SQLUnion{left, union_type, right} => {
				self.visit_sql_expr(*left);
				// TODO union type
				self.visit_sql_expr(*right);
			},
			//_ => panic!("Unsupported expr {:?}", expr)
		}
	}

	fn visit_sql_lit_expr(&mut self, lit: LiteralExpr) {
		panic!("visit_sql_lit_expr() not implemented");
	}

	fn visit_sql_operator(&mut self, op: SQLOperator) {
		panic!("visit_sql_operator() not implemented");
	}
}

pub fn walk(visitor: &mut SQLExprVisitor, e: SQLExpr) {
	visitor.visit_sql_expr(e);
}

#[cfg(test)]
mod tests {
	use super::super::sql_parser::AnsiSQLParser;
	use super::*;
	use super::{EncryptionVisitor};
	use std::collections::HashMap;

	#[test]
	fn test_visitor() {
		let parser = AnsiSQLParser {};
		let sql = "SELECT age, first_name, last_name FROM users WHERE age = 1";
		let parsed = parser.parse(sql);

		let config = super::config::parse_config("../config/src/demo-client-config.xml");
		let mut valueMap: HashMap<u32, String> = HashMap::new();
		let mut encrypt_vis = EncryptionVisitor {
			config: config,
			valuemap: valueMap
		};
		 walk(&mut encrypt_vis, parsed);

		 println!("HERE {:#?}", encrypt_vis);
	}
}
