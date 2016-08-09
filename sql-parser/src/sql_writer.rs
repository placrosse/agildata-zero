use super::sql_parser::{SQLExpr, LiteralExpr, SQLOperator, SQLUnionType, SQLJoinType};
use std::fmt::Write;


pub fn write(node:SQLExpr) -> String {
	let mut builder = String::new();
	_write(&mut builder, node);
	builder
}

fn _write(builder: &mut String, node: SQLExpr) {
	match node {
		SQLExpr::SQLSelect{expr_list, relation, selection, order} => {
			write!(builder, "{}", "SELECT").unwrap();
			_write(builder, *expr_list);
			if !relation.is_none() {
				write!(builder, " {}", "FROM").unwrap();
				_write(builder, *relation.unwrap())
			}
			if !selection.is_none() {
				write!(builder, " {}", "WHERE").unwrap();
				_write(builder, *selection.unwrap())
			}
			if !order.is_none() {
				write!(builder, " {}", "ORDER BY").unwrap();
				_write(builder, *order.unwrap())
			}

		},
		SQLExpr::SQLInsert{table, column_list, values_list} => {
			write!(builder, "{}", "INSERT INTO").unwrap();
			_write(builder, *table);
			write!(builder, " {}", "(");
			_write(builder, *column_list);
			write!(builder, "{}", ") VALUES(");
			_write(builder, *values_list);
			write!(builder, " {}", ")");
		},
		SQLExpr::SQLExprList(vector) => {
			let mut sep = "";
			for e in vector {
				write!(builder, "{}", sep).unwrap();
				_write(builder, e);
				sep = ",";
			}
		},
		SQLExpr::SQLBinary{left, op, right} => {
			_write(builder, *left);
			_write_operator(builder, op);
			_write(builder, *right);

		},
		SQLExpr::SQLLiteral(lit) => match lit {
			LiteralExpr::LiteralLong(i, l) => {
				write!(builder, " {}", l).unwrap();
			},
			LiteralExpr::LiteralBool(i, b) => {
				write!(builder, " {}", b).unwrap();
			},
			LiteralExpr::LiteralDouble(i, d) => {
				write!(builder, " {}", d).unwrap();
			},
			LiteralExpr::LiteralString(i, s) => {
				write!(builder, " '{}'", s).unwrap();
			}
			//_ => panic!("Unsupported literal for writing {:?}", lit)
		},
		SQLExpr::SQLAlias{expr, alias} => {
			_write(builder, *expr);
			write!(builder, " {}", "AS").unwrap();
			_write(builder, *alias);
		},
		SQLExpr::SQLIdentifier(id) => {
			write!(builder, " {}", id).unwrap();
		},
		SQLExpr::SQLNested(expr) => {
			write!(builder, " {}", "(").unwrap();
			_write(builder, *expr);
			write!(builder, "{}", ")").unwrap();
		},
		SQLExpr::SQLUnary{operator, expr} => {
			_write_operator(builder, operator);
			_write(builder, *expr);
		},
		SQLExpr::SQLOrderBy{expr, is_asc} => {
			_write(builder, *expr);
			if !is_asc {
				write!(builder, " {}", "DESC").unwrap();
			}
		},
		SQLExpr::SQLJoin{left, join_type, right, on_expr} => {
			_write(builder, *left);
			_write_join_type(builder, join_type);
			_write(builder, *right);
			if !on_expr.is_none() {
				write!(builder, " {}", "ON").unwrap();
				_write(builder, *on_expr.unwrap());
			}
		},
		SQLExpr::SQLUnion{left, union_type, right} => {
			_write(builder, *left);
			_write_union_type(builder, union_type);
			_write(builder, *right);
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
