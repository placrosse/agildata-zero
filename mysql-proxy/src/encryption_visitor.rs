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
}

impl<'a> SQLExprVisitor for EncryptionVisitor<'a> {
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
			SQLExpr::SQLInsert{table, column_list, values_list} => {
				panic!("Not implemented");
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
					// TODO should check left and right
					SQLOperator::EQ => {
						if (self.is_identifier(&*left) && self.is_literal(&*right)) {
							let ident = match left {
								box SQLExpr::SQLIdentifier(v) => v,
								_ => panic!("Unreachable")
							};
							let mut col = self.config.get_column_config(&String::from("babel"), &String::from("users"), &String::from(ident));
							if (col.is_some()) {
								match right {
									box SQLExpr::SQLLiteral(l) => match l {
										LiteralExpr::LiteralLong(i, val) => {
											self.valuemap.insert(i, val.encrypt(&col.unwrap().encryption));
										},
										_ => panic!("Unsupported value type {:?}", l)
									},
									_ => panic!("Unreachable")
								}
							}
						} else if (self.is_identifier(&*right) && self.is_literal(&*left)) {
							panic!("Syntax literal = identifier not currently supported")
						}

						// self.visit_sql_expr(*left);
						// self.visit_sql_expr(*right);
					},
					_ => {}
				}
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
			config: config,
			valuemap: valueMap
		};
		 walk(&mut encrypt_vis, &parsed);

		 println!("HERE {:#?}", encrypt_vis);
	}
}
