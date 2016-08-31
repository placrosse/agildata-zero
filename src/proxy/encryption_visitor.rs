use config::{Config, TConfig};
use encrypt::Encrypt;

use query::{ASTVisitor, ASTNode, LiteralExpr, Operator};

use std::collections::HashMap;

#[derive(Debug)]
pub struct EncryptionVisitor<'a> {
	pub config: &'a Config,
	pub valuemap: HashMap<u32, Option<Vec<u8>>>
}

impl<'a> EncryptionVisitor<'a> {
	fn is_identifier(&self, expr: &ASTNode) -> bool {
		match expr {
			&ASTNode::SQLIdentifier{..} => true,
			_ => false
		}
	}

	fn is_literal(&self, expr: &ASTNode) -> bool {
		match expr {
			&ASTNode::SQLLiteral(_) => true,
			_ => false
		}
	}

	pub fn get_value_map(&self) -> &HashMap<u32, Option<Vec<u8>>> {
		&self.valuemap
	}
}

impl<'a> ASTVisitor for EncryptionVisitor<'a> {
	fn visit_ast(&mut self, expr: &ASTNode) {
		match expr {
			&ASTNode::SQLSelect{box ref expr_list, ref relation, ref selection, ref order} => {
				self.visit_ast(expr_list);
				match relation {
					&Some(box ref expr) => self.visit_ast(expr),
					&None => {}
				}
				match selection {
					&Some(box ref expr) => self.visit_ast(expr),
					&None => {}
				}
				match order {
					&Some(box ref expr) => self.visit_ast(expr),
					&None => {}
				}
			},
			&ASTNode::SQLUpdate{box ref table, box ref assignments, ref selection} => {
				self.visit_ast(table);
				self.visit_ast(assignments);

				match selection {
					&Some(box ref expr) => self.visit_ast(expr),
					&None => {}
				}
			},
			&ASTNode::SQLInsert{box ref table, box ref column_list, box ref values_list} => {
				let table = match table {
					&ASTNode::SQLIdentifier{id: ref v, ..} => v,
					_ => panic!("Illegal")
				};
				match column_list {
					&ASTNode::SQLExprList(ref columns) => {
						match values_list {
							&ASTNode::SQLExprList(ref values) => {
								for (i, e) in columns.iter().enumerate() {
									match e {
										&ASTNode::SQLIdentifier{id: ref name, ..} => {
											let col = self.config.get_column_config(&String::from("zero"), table, name);
											if col.is_some() {
												match values[i] {
													ASTNode::SQLLiteral(ref l) => match l {
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
			&ASTNode::SQLExprList(ref vector) => {
				for e in vector {
					self.visit_ast(&e);
				}
			},
			&ASTNode::SQLBinary{box ref left, ref op, box ref right} => {
				//println!("HERE");
				// ident = lit
				match op {
					// TODO Messy...clean up
					// TODO should check left and right
					&Operator::EQ => {
						if self.is_identifier(left) && self.is_literal(right) {
							let ident = match left {
								&ASTNode::SQLIdentifier{id: ref v, ..} => v,
								_ => panic!("Unreachable")
							};
							let col = self.config.get_column_config(&String::from("zero"), &String::from("users"), ident);
							if col.is_some() {
								match right {
									&ASTNode::SQLLiteral(ref l) => match l {
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

				self.visit_ast(left);
				self.visit_ast(right);
			},
			&ASTNode::SQLLiteral(ref lit) => {
				self.visit_ast_lit(lit);
			},
			&ASTNode::SQLAlias{box ref expr, box ref alias} => {
				self.visit_ast(&expr);
				self.visit_ast(&alias);
			},
			&ASTNode::SQLIdentifier{..} => {
				// TODO end of visit arm
			},
			&ASTNode::SQLNested(ref expr) => {
				self.visit_ast(expr);
			},
			&ASTNode::SQLUnary{ref operator, box ref expr} => {
				self.visit_ast_operator(operator);
				self.visit_ast(expr);
			},
			&ASTNode::SQLOrderBy{box ref expr, ..} => {
				self.visit_ast(expr);
				// TODO bool
			},
			&ASTNode::SQLJoin{box ref left, box ref right, ref on_expr, ..} => {
				self.visit_ast(left);
				// TODO visit join type
				self.visit_ast(right);
				match on_expr {
					&Some(box ref expr) => self.visit_ast(expr),
					&None => {}
				}
			},
			&ASTNode::SQLUnion{box ref left, box ref right, ..} => {
				self.visit_ast(left);
				// TODO union type
				self.visit_ast(right);
			},
			&ASTNode::MySQLCreateTable{..} => {
				println!("WARN: create table visitation not implemented")
			},
			_ => panic!("Unsupported expr {:?}", expr)
		}
	}

	fn visit_ast_lit(&mut self, _lit: &LiteralExpr) {
		//do nothing
	}

	fn visit_ast_operator(&mut self, _op: &Operator) {
		// do nothing
	}
}

pub fn walk(visitor: &mut ASTVisitor, e: &ASTNode) {
	visitor.visit_ast(e);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
	use query::{ASTVisitor, ASTNode, LiteralExpr, Operator, SQLWriter, Tokenizer, Parser, Writer};
	use query::dialects::mysqlsql::*;
	use query::dialects::ansisql::*;
	use super::super::writers::*;
    use config;

	#[test]
	fn test_visitor() {
		let ansi = AnsiSQLDialect::new();
		let dialect = MySQLDialect::new(&ansi);

		let sql = String::from("SELECT age, first_name, last_name FROM users WHERE age = 1");
		let parsed = sql.tokenize(&dialect).unwrap().parse().unwrap();

		let config = config::parse_config("example-zero-config.xml");
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

		let ansi = AnsiSQLDialect::new();
		let dialect = MySQLDialect::new(&ansi);

		let sql = String::from("INSERT INTO users (id, first_name, last_name, ssn, age, sex) VALUES(1, 'Janis', 'Joplin', '123456789', 27, 'F')");
		let parsed = sql.tokenize(&dialect).unwrap().parse().unwrap();

		let config = config::parse_config("example-zero-config.xml");
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
		let ansi = AnsiSQLDialect::new();
		let dialect = MySQLDialect::new(&ansi);

		let sql = String::from("UPDATE users SET age = 31, ssn = '987654321' WHERE first_name = 'Janis' AND last_name = 'Joplin'");
		let parsed = sql.tokenize(&dialect).unwrap().parse().unwrap();

		let config = config::parse_config("example-zero-config.xml");
		let value_map: HashMap<u32, Option<Vec<u8>>> = HashMap::new();
		let mut encrypt_vis = EncryptionVisitor {
			config: &config,
			valuemap: value_map
		};
		walk(&mut encrypt_vis, &parsed);

		println!("HERE {:#?}", encrypt_vis);

		let lit_writer = LiteralReplacingWriter{literals: &encrypt_vis.get_value_map()};
		let ansi_writer = AnsiSQLWriter{};

		let writer = SQLWriter::new(vec![&lit_writer, &ansi_writer]);

		let rewritten = writer.write(&parsed).unwrap();

		println!("Rewritten: {}", rewritten);

	}
}
