use super::sql_parser::{SQLExpr, LiteralExpr, SQLOperator, SQLUnionType, SQLJoinType};
use std::fmt::Write;
use std::collections::HashMap;

pub fn to_hex_string(bytes: &Vec<u8>) -> String {
  let strs: Vec<String> = bytes.iter()
                               .map(|b| format!("{:02X}", b))
                               .collect();
  strs.connect("")
}

pub fn write(node:SQLExpr, literals: &HashMap<u32, Option<Vec<u8>>>) -> String {
	let mut builder = String::new();
	_write(&mut builder, node, literals);
	builder
}

fn _write(builder: &mut String, node: SQLExpr, literals: &HashMap<u32, Option<Vec<u8>>>) {
	match node {
		SQLExpr::SQLSelect{expr_list, relation, selection, order} => {
			write!(builder, "{}", "SELECT").unwrap();
			_write(builder, *expr_list, literals);
			if !relation.is_none() {
				write!(builder, " {}", "FROM").unwrap();
				_write(builder, *relation.unwrap(), literals)
			}
			if !selection.is_none() {
				write!(builder, " {}", "WHERE").unwrap();
				_write(builder, *selection.unwrap(), literals)
			}
			if !order.is_none() {
				write!(builder, " {}", "ORDER BY").unwrap();
				_write(builder, *order.unwrap(), literals)
			}

		},
		SQLExpr::SQLInsert{table, column_list, values_list} => {
			write!(builder, "{}", "INSERT INTO").unwrap();
			_write(builder, *table, literals);
			write!(builder, " {}", "(");
			_write(builder, *column_list, literals);
			write!(builder, "{}", ") VALUES(");
			_write(builder, *values_list, literals);
			write!(builder, " {}", ")");
		},
		SQLExpr::SQLExprList(vector) => {
			let mut sep = "";
			for e in vector {
				write!(builder, "{}", sep).unwrap();
				_write(builder, e, literals);
				sep = ",";
			}
		},
		SQLExpr::SQLBinary{left, op, right} => {
			_write(builder, *left, literals);
			_write_operator(builder, op);
			_write(builder, *right, literals);

		},
		SQLExpr::SQLLiteral(lit) => match lit {
			LiteralExpr::LiteralLong(i, l) => {
				match literals.get(&i) {
					Some(value) => match value {
						// TODO write! escapes the single quotes...
						// X'...'
						&Some(ref e) => {
							write!(builder, "X'{}'", to_hex_string(e)).unwrap();
						},
						&None => write!(builder, " {}", l).unwrap()
					},
					None => write!(builder, " {}", l).unwrap()
				}
			},
			LiteralExpr::LiteralBool(i, b) => {
				write!(builder, " {}", b).unwrap();
			},
			LiteralExpr::LiteralDouble(i, d) => {
				write!(builder, " {}", d).unwrap();
			},
			LiteralExpr::LiteralString(i, s) => {
				match literals.get(&i) {
					Some(value) => match value {
						// TODO write! escapes the single quotes...
						// X'...'
						&Some(ref e) => {
							write!(builder, "X'{}'", to_hex_string(e)).unwrap();
						},
						&None => write!(builder, " {}", s).unwrap()
					},
					None => write!(builder, " {}", s).unwrap()
				}
			}
			//_ => panic!("Unsupported literal for writing {:?}", lit)
		},
		SQLExpr::SQLAlias{expr, alias} => {
			_write(builder, *expr, literals);
			write!(builder, " {}", "AS").unwrap();
			_write(builder, *alias, literals);
		},
		SQLExpr::SQLIdentifier(id) => {
			write!(builder, " {}", id).unwrap();
		},
		SQLExpr::SQLNested(expr) => {
			write!(builder, " {}", "(").unwrap();
			_write(builder, *expr, literals);
			write!(builder, "{}", ")").unwrap();
		},
		SQLExpr::SQLUnary{operator, expr} => {
			_write_operator(builder, operator);
			_write(builder, *expr, literals);
		},
		SQLExpr::SQLOrderBy{expr, is_asc} => {
			_write(builder, *expr, literals);
			if !is_asc {
				write!(builder, " {}", "DESC").unwrap();
			}
		},
		SQLExpr::SQLJoin{left, join_type, right, on_expr} => {
			_write(builder, *left, literals);
			_write_join_type(builder, join_type);
			_write(builder, *right, literals);
			if !on_expr.is_none() {
				write!(builder, " {}", "ON").unwrap();
				_write(builder, *on_expr.unwrap(), literals);
			}
		},
		SQLExpr::SQLUnion{left, union_type, right} => {
			_write(builder, *left, literals);
			_write_union_type(builder, union_type);
			_write(builder, *right, literals);
		}
		//_ => panic!("Unsupported node for writing {:?}", node)
	}

	fn _write_operator(builder: &mut String, op: SQLOperator) {
		let op_text = match op {
			SQLOperator::ADD => "+",
			SQLOperator::SUB => "-",
			SQLOperator::MULT => "*",
			SQLOperator::DIV => "/",
			SQLOperator::MOD => "%",
			SQLOperator::GT => ">",
			SQLOperator::LT => "<",
			SQLOperator::GTEQ => ">=",
			SQLOperator::LTEQ => "<=",
			SQLOperator::EQ => "=",
			SQLOperator::NEQ => "!=",
			SQLOperator::OR => "OR",
			SQLOperator::AND  => "AND"
		};
		write!(builder, " {}", op_text).unwrap();
	}

	fn _write_join_type(builder: &mut String, join_type: SQLJoinType) {
		let text = match join_type {
			SQLJoinType::INNER => "INNER JOIN",
			SQLJoinType::LEFT => "LEFT JOIN",
			SQLJoinType::RIGHT => "RIGHT JOIN",
			SQLJoinType::FULL => "FULL OUTER JOIN",
			SQLJoinType::CROSS => "CROSS JOIN"
		};
		write!(builder, " {}", text).unwrap();
	}

	fn _write_union_type(builder: &mut String, union_type: SQLUnionType) {
		let text = match union_type {
			SQLUnionType::UNION => "UNION",
			SQLUnionType::ALL => "UNION ALL",
			SQLUnionType::DISTINCT => "UNION DISTINCT"
		};
		write!(builder, " {}", text).unwrap();
	}
}
