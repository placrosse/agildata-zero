use super::sql_parser::{SQLExpr, LiteralExpr, SQLOperator, SQLUnionType, SQLJoinType, DataType, ColumnQualifier, SQLKeyDef};
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
			builder.push_str("SELECT");
			_write(builder, *expr_list, literals);
			if !relation.is_none() {
				builder.push_str(" FROM");
				_write(builder, *relation.unwrap(), literals)
			}
			if !selection.is_none() {
				builder.push_str(" WHERE");
				_write(builder, *selection.unwrap(), literals)
			}
			if !order.is_none() {
				builder.push_str(" ORDER BY");
				_write(builder, *order.unwrap(), literals)
			}

		},
		SQLExpr::SQLInsert{table, column_list, values_list} => {
			builder.push_str("INSERT INTO");
			_write(builder, *table, literals);
			builder.push_str(" (");
			_write(builder, *column_list, literals);
			builder.push_str(") VALUES(");
			_write(builder, *values_list, literals);
			builder.push_str(")");
		},
        SQLExpr::SQLUpdate{table, assignments, selection} => {
            builder.push_str("UPDATE");
            _write(builder, *table, literals);
            builder.push_str(" SET");
            _write(builder, *assignments, literals);
            if selection.is_some() {
                builder.push_str(" WHERE");
                _write(builder, *selection.unwrap(), literals);
            }
        },
        SQLExpr::SQLCreateTable{table, column_list, keys} => {
            builder.push_str("CREATE TABLE");
            _write(builder, *table, literals);

            builder.push_str(&" (");
            let mut sep = "";
            for c in column_list {
                builder.push_str(sep);
                _write(builder, c, literals);
                sep = ", ";
            }

            for k in keys {
                builder.push_str(sep);
                _write_key_definition(builder, k, literals);
                sep = ", ";
            }

            builder.push_str(&")");
        },
        SQLExpr::SQLColumnDef{column, data_type, qualifiers} => {
            _write(builder, *column, literals);
            _write_data_type(builder, data_type, literals);
            if qualifiers.is_some() {
                for q in qualifiers.unwrap() {
                    _write_column_qualifier(builder, q, literals);
                }
            }

        },
		SQLExpr::SQLExprList(vector) => {
			let mut sep = "";
			for e in vector {
				builder.push_str(sep);
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
							write!(builder, "X'{}'", to_hex_string(e)).unwrap(); // must_use
						},
						&None => write!(builder, " {}", l).unwrap(),
					},
					None => write!(builder, " {}", l).unwrap(),
				}
			},
			LiteralExpr::LiteralBool(_, b) => {
				write!(builder, "{}", b).unwrap();
			},
			LiteralExpr::LiteralDouble(_, d) => {
				write!(builder, "{}", d).unwrap();
			},
			LiteralExpr::LiteralString(i, s) => {
				match literals.get(&i) {
					Some(value) => match value {
						&Some(ref e) => {
							write!(builder, "X'{}'", to_hex_string(e)).unwrap(); // must_use
						},
						&None => write!(builder, " '{}'", s).unwrap(),
					},
					None => write!(builder, " '{}'", s).unwrap(),
				}
			}
			//_ => panic!("Unsupported literal for writing {:?}", lit)
		},
		SQLExpr::SQLAlias{expr, alias} => {
			_write(builder, *expr, literals);
			builder.push_str(" AS");
			_write(builder, *alias, literals);
		},
		SQLExpr::SQLIdentifier(id) => {
			write!(builder, " {}", id).unwrap();
		},
		SQLExpr::SQLNested(expr) => {
			builder.push_str("(");
			_write(builder, *expr, literals);
			builder.push_str(")");
		},
		SQLExpr::SQLUnary{operator, expr} => {
			_write_operator(builder, operator);
			_write(builder, *expr, literals);
		},
		SQLExpr::SQLOrderBy{expr, is_asc} => {
			_write(builder, *expr, literals);
			if !is_asc {
				builder.push_str(" DESC");
			}
		},
		SQLExpr::SQLJoin{left, join_type, right, on_expr} => {
			_write(builder, *left, literals);
			_write_join_type(builder, join_type);
			_write(builder, *right, literals);
			if !on_expr.is_none() {
				builder.push_str(" ON");
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
                builder.push_str(" BIT");
                _write_optional_display(builder, display);
            },
            DataType::TinyInt{display} => {
                builder.push_str(" TINYINT");
                _write_optional_display(builder, display);
            },
            DataType::SmallInt{display} => {
                builder.push_str(" SMALLINT");
                _write_optional_display(builder, display);
            },
            DataType::MediumInt{display} => {
                builder.push_str(" MEDIUMINT");
                _write_optional_display(builder, display);
            },
            DataType::Int{display} => {
                builder.push_str(" INTEGER");
                _write_optional_display(builder, display);
            },
            DataType::BigInt{display} => {
                builder.push_str(" BIGINT");
                _write_optional_display(builder, display);
            },
            DataType::Decimal{precision, scale} => {
                builder.push_str(" DECIMAL");
                _write_optional_precision_and_scale(builder, precision, scale);
            },
            DataType::Float{precision, scale} => {
                builder.push_str(" FLOAT");
                _write_optional_precision_and_scale(builder, precision, scale);
            },
            DataType::Double{precision, scale} => {
                builder.push_str(" DOUBLE");
                _write_optional_precision_and_scale(builder, precision, scale);
            },
            DataType::Bool => {
                builder.push_str(" BOOLEAN");
            },
            DataType::Date => {
                builder.push_str(" DATE");
            },
            DataType::DateTime{fsp} => {
                builder.push_str(" DATETIME");
                _write_optional_display(builder, fsp);
            },
            DataType::Timestamp{fsp} => {
                builder.push_str(" TIMESTAMP");
                _write_optional_display(builder, fsp);
            },
            DataType::Time{fsp} => {
                builder.push_str(" TIME");
                _write_optional_display(builder, fsp);
            },
            DataType::Year{display} => {
                builder.push_str(" YEAR");
                _write_optional_display(builder, display);
            },
            DataType::Char{length} => {
                builder.push_str(" CHAR");
                _write_optional_display(builder, length);
            },
            DataType::NChar{length} => {
                builder.push_str(" NCHAR");
                _write_optional_display(builder, length);
            },
            DataType::CharByte{length} => {
                builder.push_str(" CHAR");
                _write_optional_display(builder, length);
                builder.push_str(" BYTE");
            },
            DataType::Varchar{length} => {
                builder.push_str(" VARCHAR");
                _write_optional_display(builder, length);
            },
            DataType::NVarchar{length} => {
                builder.push_str(" NVARCHAR");
                _write_optional_display(builder, length);
            },
            DataType::Binary{length} => {
                builder.push_str(" BINARY");
                _write_optional_display(builder, length);
            },
            DataType::VarBinary{length} => {
                builder.push_str(" VARBINARY");
                _write_optional_display(builder, length);
            },
            DataType::Blob{length} => {
                builder.push_str(" BLOB");
                _write_optional_display(builder, length);
            },
            DataType::Text{length} => {
                builder.push_str(" TEXT");
                _write_optional_display(builder, length);
            },
            DataType::TinyBlob => {
                builder.push_str(" TINYBLOB");
            },
            DataType::TinyText => {
                builder.push_str(" TINYTEXT");
            },
            DataType::MediumBlob => {
                builder.push_str(" MEDIUMBLOB");
            },
            DataType::MediumText => {
                builder.push_str(" MEDIUMTEXT");
            },
            DataType::LongBlob => {
                builder.push_str(" LONGBLOB");
            },
            DataType::LongText => {
                builder.push_str(" LONGTEXT");
            },
            DataType::Enum{values} => {
                builder.push_str(" ENUM(");
                _write(builder, *values, literals);
                builder.push_str(")");
            },
            DataType::Set{values} => {
                builder.push_str(" SET(");
                _write(builder, *values, literals);
                builder.push_str(")");
            },
            // _ => panic!("Unsupported data type {:?}", data_type)

        }
    }

    fn _write_key_definition(builder:  &mut String, key: SQLKeyDef, literals: &HashMap<u32, Option<Vec<u8>>>) {
        match key {
            SQLKeyDef::Primary{name, columns} => {
                builder.push_str(&" PRIMARY KEY");
                if name.is_some() {
                    _write(builder, *name.unwrap(), literals);
                }
                _write_key_column_list(builder, columns, literals);
            },
            SQLKeyDef::Unique{name, columns} => {
                builder.push_str(&" UNIQUE KEY");
                if name.is_some() {
                    _write(builder, *name.unwrap(), literals);
                }
                _write_key_column_list(builder, columns, literals);
            },
            SQLKeyDef::FullText{name, columns} => {
                builder.push_str(&" FULLTEXT KEY");
                if name.is_some() {
                    _write(builder, *name.unwrap(), literals);
                }
                _write_key_column_list(builder, columns, literals);
            },
            SQLKeyDef::Index{name, columns} => {
                builder.push_str(&" KEY");
                if name.is_some() {
                    _write(builder, *name.unwrap(), literals);
                }
                _write_key_column_list(builder, columns, literals);
            },
            SQLKeyDef::Foreign{name, columns, reference_table, reference_columns} => {
                builder.push_str(&" FOREIGN KEY");
                if name.is_some() {
                    _write(builder, *name.unwrap(), literals);
                }
                _write_key_column_list(builder, columns, literals);

                builder.push_str(&" REFERENCES");
                _write(builder, *reference_table, literals);
                _write_key_column_list(builder, reference_columns, literals);
            }
        }
    }

    fn _write_key_column_list(builder: &mut String, list: Vec<SQLExpr>, literals: &HashMap<u32, Option<Vec<u8>>>) {
        builder.push_str(&" (");
        let mut sep = "";
        for c in list {
            builder.push_str(sep);
            _write(builder, c, literals);
            sep = ", ";
        }
        builder.push_str(&")");
    }

    fn _write_column_qualifier(builder:  &mut String, q: ColumnQualifier, literals: &HashMap<u32, Option<Vec<u8>>>) {
        match q {
            ColumnQualifier::CharacterSet(box e) => {
                builder.push_str(&" CHARACTER SET");
                _write(builder, e, literals);
            },
            ColumnQualifier::Collate(box e) => {
                builder.push_str(&" COLLATE");
                _write(builder, e, literals);
            },
            ColumnQualifier::Default(box e) => {
                builder.push_str(&" DEFAULT");
                _write(builder, e, literals);
            },
            ColumnQualifier::Signed => builder.push_str(&" SIGNED"),
            ColumnQualifier::Unsigned => builder.push_str(&" UNSIGNED"),
            ColumnQualifier::Null => builder.push_str(&" NULL"),
            ColumnQualifier::NotNull => builder.push_str(&" NOT NULL"),
            ColumnQualifier::AutoIncrement => builder.push_str(&" AUTO_INCREMENT"),
            ColumnQualifier::PrimaryKey => builder.push_str(&" PRIMARY KEY"),
            ColumnQualifier::UniqueKey => builder.push_str(&" UNIQUE"),
            ColumnQualifier::OnUpdate(box e) => {
                builder.push_str(&" ON UPDATE");
                _write(builder, e, literals);
            },
            ColumnQualifier::Comment(box e) => {
                builder.push_str(&" COMMENT");
                _write(builder, e, literals);
            }
        }
    }

    fn _write_optional_display(builder: &mut String, display: Option<u32>) {
        match display {
            Some(d) => {write!(builder, "({})", d).unwrap();},
            None => {}
        }
        ()
    }

    fn _write_optional_precision_and_scale(builder: &mut String, precision: Option<u32>, scale: Option<u32>) {
        match precision {
            Some(p) => {
                write!(builder, "({}", p).unwrap();
                if scale.is_some() {
                    write!(builder, ",{}", scale.unwrap()).unwrap();
                }
                builder.push_str(")");
            },
            None => {}
        }
        ()
    }
}
