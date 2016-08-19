use super::sql_parser::{SQLExpr, LiteralExpr, SQLOperator, SQLUnionType, SQLJoinType, DataType};
use std::fmt::Write;
use std::collections::HashMap;

pub fn to_hex_string(bytes: &Vec<u8>) -> String {
  let strs: Vec<String> = bytes.iter()
                               .map(|b| format!("{:02X}", b))
                               .collect();
  strs.join("")
}

pub fn write(node:SQLExpr, literals: &HashMap<u32, Option<Vec<u8>>>) -> String {
	let mut builder = String::new();
	_write(&mut builder, node, literals);
	builder
}

fn _write(builder: &mut String, node: SQLExpr, literals: &HashMap<u32, Option<Vec<u8>>>) {
	match node {
		SQLExpr::SQLSelect{expr_list, relation, selection, order} => {
			write!(builder, "{}", "SELECT");
			_write(builder, *expr_list, literals);
			if !relation.is_none() {
				write!(builder, " {}", "FROM");
				_write(builder, *relation.unwrap(), literals)
			}
			if !selection.is_none() {
				write!(builder, " {}", "WHERE");
				_write(builder, *selection.unwrap(), literals)
			}
			if !order.is_none() {
				write!(builder, " {}", "ORDER BY");
				_write(builder, *order.unwrap(), literals)
			}

		},
		SQLExpr::SQLInsert{table, column_list, values_list} => {
			write!(builder, "{}", "INSERT INTO");
			_write(builder, *table, literals);
			write!(builder, " (");
			_write(builder, *column_list, literals);
			write!(builder, ") VALUES(");
			_write(builder, *values_list, literals);
			write!(builder, ")");
		},
        SQLExpr::SQLUpdate{table, assignments, selection} => {
            write!(builder, "UPDATE");
            _write(builder, *table, literals);
            write!(builder, "SET");
            _write(builder, *assignments, literals);
            if selection.is_some() {
                write!(builder, "WHERE");
                _write(builder, *selection.unwrap(), literals);
            }
        },
        SQLExpr::SQLCreateTable{column_list, ..} => {
            write!(builder, "CREATE TABLE");
            let mut sep = "";
            for c in column_list {
                write!(builder, "{}", sep);
                _write(builder, c, literals);
                sep = ", ";
            }
        },
        SQLExpr::SQLColumnDef{column, data_type, ..} => {
            _write(builder, *column, literals);
            _write_data_type(builder, data_type, literals);
        },
		SQLExpr::SQLExprList(vector) => {
			let mut sep = "";
			for e in vector {
				write!(builder, "{}", sep);
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
							write!(builder, "X'{}'", to_hex_string(e));
						},
						&None => {write!(builder, " {}", l);},
					},
					None => write!(builder, " {}", l)
				}
			},
			LiteralExpr::LiteralBool(_, b) => {
				write!(builder, " {}", b);
			},
			LiteralExpr::LiteralDouble(_, d) => {
				write!(builder, " {}", d);
			},
			LiteralExpr::LiteralString(i, s) => {
				match literals.get(&i) {
					Some(value) => match value {
						// TODO write! escapes the single quotes...
						// X'...'
						&Some(ref e) => {
							write!(builder, "X'{}'", to_hex_string(e));
						},
						&None => write!(builder, " '{}'", s)
					},
					None => write!(builder, " '{}'", s)
				}
			}
			//_ => panic!("Unsupported literal for writing {:?}", lit)
		},
		SQLExpr::SQLAlias{expr, alias} => {
			_write(builder, *expr, literals);
			write!(builder, " {}", "AS");
			_write(builder, *alias, literals);
		},
		SQLExpr::SQLIdentifier(id) => {
			write!(builder, " {}", id);
		},
		SQLExpr::SQLNested(expr) => {
			write!(builder, " {}", "(");
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
			// SQLOperator::GTEQ => ">=",
			// SQLOperator::LTEQ => "<=",
			SQLOperator::EQ => "=",
			// SQLOperator::NEQ => "!=",
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
		write!(builder, " {} ", text).unwrap();
	}

    fn _write_data_type(builder: &mut String, data_type: DataType, literals: &HashMap<u32, Option<Vec<u8>>>) {
        match data_type {
            DataType::Bit{display} => {
                write!(builder, " {}", "BIT");
                _write_optional_display(builder, display);
            },
            DataType::TinyInt{display} => {
                write!(builder, " {}", "TINYINT");
                _write_optional_display(builder, display);
            },
            DataType::SmallInt{display} => {
                write!(builder, " {}", "SMALLINT");
                _write_optional_display(builder, display);
            },
            DataType::MediumInt{display} => {
                write!(builder, " {}", "MEDIUMINT");
                _write_optional_display(builder, display);
            },
            DataType::Int{display} => {
                write!(builder, " {}", "INTEGER");
                _write_optional_display(builder, display);
            },
            DataType::BigInt{display} => {
                write!(builder, " {}", "BIGINT");
                _write_optional_display(builder, display);
            },
            DataType::Decimal{precision, scale} => {
                write!(builder, " {}", "DECIMAL");
                _write_optional_precision_and_scale(builder, precision, scale);
            },
            DataType::Float{precision, scale} => {
                write!(builder, " {}", "FLOAT");
                _write_optional_precision_and_scale(builder, precision, scale);
            },
            DataType::Double{precision, scale} => {
                write!(builder, " {}", "DOUBLE");
                _write_optional_precision_and_scale(builder, precision, scale);
            },
            DataType::Bool => {
                write!(builder, " {}", "BOOLEAN");
            },
            DataType::Date => {
                write!(builder, " {}", "DATE");
            },
            DataType::DateTime{fsp} => {
                write!(builder, " {}", "DATETIME");
                _write_optional_display(builder, fsp);
            },
            DataType::Timestamp{fsp} => {
                write!(builder, " {}", "TIMESTAMP");
                _write_optional_display(builder, fsp);
            },
            DataType::Time{fsp} => {
                write!(builder, " {}", "TIME");
                _write_optional_display(builder, fsp);
            },
            DataType::Year{display} => {
                write!(builder, " {}", "DATETIME");
                _write_optional_display(builder, display);
            },
            DataType::Char{length} => {
                write!(builder, " {}", "CHAR");
                _write_optional_display(builder, length);
            },
            DataType::Varchar{length} => {
                write!(builder, " {}", "VARCHAR");
                _write_optional_display(builder, length);
            },
            DataType::Binary{length} => {
                write!(builder, " {}", "BINARY");
                _write_optional_display(builder, length);
            },
            DataType::VarBinary{length} => {
                write!(builder, " {}", "VARBINARY");
                _write_optional_display(builder, length);
            },
            DataType::Blob{length} => {
                write!(builder, " {}", "BLOB");
                _write_optional_display(builder, length);
            },
            DataType::Text{length} => {
                write!(builder, " {}", "TEXT");
                _write_optional_display(builder, length);
            },
            DataType::TinyBlob => {
                write!(builder, " {}", "TINYBLOB");
            },
            DataType::TinyText => {
                write!(builder, " {}", "TINYTEXT");
            },
            DataType::MediumBlob => {
                write!(builder, " {}", "MEDIUMBLOB");
            },
            DataType::MediumText => {
                write!(builder, " {}", "MEDIUMTEXT");
            },
            DataType::LongBlob => {
                write!(builder, " {}", "LONGBLOB");
            },
            DataType::LongText => {
                write!(builder, " {}", "LONGTEXT");
            },
            DataType::Enum{values} => {
                write!(builder, " {}(", "ENUM");
                _write(builder, *values, literals);
                write!(builder, " {}", ")");
            },
            DataType::Set{values} => {
                write!(builder, " {}(", "ENUM");
                _write(builder, *values, literals);
                write!(builder, " {}", ")");
            },
            // _ => panic!("Unsupported data type {:?}", data_type)

        }
    }

    fn _write_optional_display(builder: &mut String, display: Option<u32>) {
        match display {
            Some(d) => {write!(builder, "({})", d);},
            None => {}
        }
        ()
    }

    fn _write_optional_precision_and_scale(builder: &mut String, precision: Option<u32>, scale: Option<u32>) {
        match precision {
            Some(p) => {
                write!(builder, "({}", p);
                if scale.is_some() {
                    write!(builder, ",{}", scale.unwrap());
                }
                write!(builder, "{}", ")");
            },
            None => {}
        }
        ()
    }
}
