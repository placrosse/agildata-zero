use super::sql_parser::{SQLExpr, LiteralExpr, SQLOperator,
    SQLUnionType, SQLJoinType, DataType, ColumnQualifier,
    SQLKeyDef, TableOption};
use std::fmt::Write;
use std::collections::HashMap;

pub fn to_hex_string(bytes: &Vec<u8>) -> String {
  let strs: Vec<String> = bytes.iter()
                               .map(|b| format!("{:02X}", b))
                               .collect();
  strs.join("")
}

pub trait SqlWriter {
    fn write(&self, node:SQLExpr) -> String;
}

struct LiteralReplacingWriter<'a> {
    literals: &'a HashMap<u32, Option<Vec<u8>>>
}

impl<'a> SqlWriter for LiteralReplacingWriter<'a> {
    fn write(&self, node:SQLExpr) -> String {
        let mut builder = String::new();
        self._write(&mut builder, node);
        builder
    }
}

impl<'a> LiteralReplacingWriter<'a> {
    fn _write(&self, builder: &mut String, node: SQLExpr) {
    	match node {
    		SQLExpr::SQLSelect{expr_list, relation, selection, order} => {
    			builder.push_str("SELECT");
    			self._write(builder, *expr_list);
    			if !relation.is_none() {
    				builder.push_str(" FROM");
    				self._write(builder, *relation.unwrap())
    			}
    			if !selection.is_none() {
    				builder.push_str(" WHERE");
    				self._write(builder, *selection.unwrap())
    			}
    			if !order.is_none() {
    				builder.push_str(" ORDER BY");
    				self._write(builder, *order.unwrap())
    			}

    		},
    		SQLExpr::SQLInsert{table, column_list, values_list} => {
    			builder.push_str("INSERT INTO");
    			self._write(builder, *table);
    			builder.push_str(" (");
    			self._write(builder, *column_list);
    			builder.push_str(") VALUES(");
    			self._write(builder, *values_list);
    			builder.push_str(")");
    		},
            SQLExpr::SQLUpdate{table, assignments, selection} => {
                builder.push_str("UPDATE");
                self._write(builder, *table);
                builder.push_str(" SET");
                self._write(builder, *assignments);
                if selection.is_some() {
                    builder.push_str(" WHERE");
                    self._write(builder, *selection.unwrap());
                }
            },
            SQLExpr::SQLCreateTable{table, column_list, keys, table_options} => {
                builder.push_str("CREATE TABLE");
                self._write(builder, *table);

                builder.push_str(&" (");
                let mut sep = "";
                for c in column_list {
                    builder.push_str(sep);
                    self._write(builder, c);
                    sep = ", ";
                }

                for k in keys {
                    builder.push_str(sep);
                    self._write_key_definition(builder, k);
                    sep = ", ";
                }

                builder.push_str(&")");

                sep = " ";
                for o in table_options {
                    builder.push_str(sep);
                    self._write_table_option(builder, o);
                }
            },
            SQLExpr::SQLColumnDef{column, data_type, qualifiers} => {
                self._write(builder, *column);
                self._write_data_type(builder, data_type);
                if qualifiers.is_some() {
                    for q in qualifiers.unwrap() {
                        self._write_column_qualifier(builder, q);
                    }
                }

            },
    		SQLExpr::SQLExprList(vector) => {
    			let mut sep = "";
    			for e in vector {
    				builder.push_str(sep);
    				self._write(builder, e);
    				sep = ",";
    			}
    		},
    		SQLExpr::SQLBinary{left, op, right} => {
    			self._write(builder, *left);
    			self._write_operator(builder, op);
    			self._write(builder, *right);

    		},
    		SQLExpr::SQLLiteral(lit) => match lit {
    			LiteralExpr::LiteralLong(i, l) => {
    				match self.literals.get(&i) {
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
    				match self.literals.get(&i) {
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
    			self._write(builder, *expr);
    			builder.push_str(" AS");
    			self._write(builder, *alias);
    		},
    		SQLExpr::SQLIdentifier(id) => {
    			write!(builder, " {}", id).unwrap();
    		},
    		SQLExpr::SQLNested(expr) => {
    			builder.push_str("(");
    			self._write(builder, *expr);
    			builder.push_str(")");
    		},
    		SQLExpr::SQLUnary{operator, expr} => {
    			self._write_operator(builder, operator);
    			self._write(builder, *expr);
    		},
    		SQLExpr::SQLOrderBy{expr, is_asc} => {
    			self._write(builder, *expr);
    			if !is_asc {
    				builder.push_str(" DESC");
    			}
    		},
    		SQLExpr::SQLJoin{left, join_type, right, on_expr} => {
    			self._write(builder, *left);
    			self._write_join_type(builder, join_type);
    			self._write(builder, *right);
    			if !on_expr.is_none() {
    				builder.push_str(" ON");
    				self._write(builder, *on_expr.unwrap());
    			}
    		},
    		SQLExpr::SQLUnion{left, union_type, right} => {
    			self._write(builder, *left);
    			self._write_union_type(builder, union_type);
    			self._write(builder, *right);
    		}
    		//_ => panic!("Unsupported node for writing {:?}", node)
    	}
    }

    fn _write_operator(&self, builder: &mut String, op: SQLOperator) {
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

    fn _write_join_type(&self, builder: &mut String, join_type: SQLJoinType) {
        let text = match join_type {
            SQLJoinType::INNER => "INNER JOIN",
            SQLJoinType::LEFT => "LEFT JOIN",
            SQLJoinType::RIGHT => "RIGHT JOIN",
            SQLJoinType::FULL => "FULL OUTER JOIN",
            SQLJoinType::CROSS => "CROSS JOIN"
        };
        write!(builder, " {}", text).unwrap();
    }

    fn _write_union_type(&self, builder: &mut String, union_type: SQLUnionType) {
        let text = match union_type {
            SQLUnionType::UNION => "UNION",
            SQLUnionType::ALL => "UNION ALL",
            SQLUnionType::DISTINCT => "UNION DISTINCT"
        };
        write!(builder, " {} ", text).unwrap();
    }

    fn _write_data_type(&self, builder: &mut String, data_type: DataType) {
        match data_type {
            DataType::Bit{display} => {
                builder.push_str(" BIT");
                self._write_optional_display(builder, display);
            },
            DataType::TinyInt{display} => {
                builder.push_str(" TINYINT");
                self._write_optional_display(builder, display);
            },
            DataType::SmallInt{display} => {
                builder.push_str(" SMALLINT");
                self._write_optional_display(builder, display);
            },
            DataType::MediumInt{display} => {
                builder.push_str(" MEDIUMINT");
                self._write_optional_display(builder, display);
            },
            DataType::Int{display} => {
                builder.push_str(" INTEGER");
                self._write_optional_display(builder, display);
            },
            DataType::BigInt{display} => {
                builder.push_str(" BIGINT");
                self._write_optional_display(builder, display);
            },
            DataType::Decimal{precision, scale} => {
                builder.push_str(" DECIMAL");
                self._write_optional_precision_and_scale(builder, precision, scale);
            },
            DataType::Float{precision, scale} => {
                builder.push_str(" FLOAT");
                self._write_optional_precision_and_scale(builder, precision, scale);
            },
            DataType::Double{precision, scale} => {
                builder.push_str(" DOUBLE");
                self._write_optional_precision_and_scale(builder, precision, scale);
            },
            DataType::Bool => {
                builder.push_str(" BOOLEAN");
            },
            DataType::Date => {
                builder.push_str(" DATE");
            },
            DataType::DateTime{fsp} => {
                builder.push_str(" DATETIME");
                self._write_optional_display(builder, fsp);
            },
            DataType::Timestamp{fsp} => {
                builder.push_str(" TIMESTAMP");
                self._write_optional_display(builder, fsp);
            },
            DataType::Time{fsp} => {
                builder.push_str(" TIME");
                self._write_optional_display(builder, fsp);
            },
            DataType::Year{display} => {
                builder.push_str(" YEAR");
                self._write_optional_display(builder, display);
            },
            DataType::Char{length} => {
                builder.push_str(" CHAR");
                self._write_optional_display(builder, length);
            },
            DataType::NChar{length} => {
                builder.push_str(" NCHAR");
                self._write_optional_display(builder, length);
            },
            DataType::CharByte{length} => {
                builder.push_str(" CHAR");
                self._write_optional_display(builder, length);
                builder.push_str(" BYTE");
            },
            DataType::Varchar{length} => {
                builder.push_str(" VARCHAR");
                self._write_optional_display(builder, length);
            },
            DataType::NVarchar{length} => {
                builder.push_str(" NVARCHAR");
                self._write_optional_display(builder, length);
            },
            DataType::Binary{length} => {
                builder.push_str(" BINARY");
                self._write_optional_display(builder, length);
            },
            DataType::VarBinary{length} => {
                builder.push_str(" VARBINARY");
                self._write_optional_display(builder, length);
            },
            DataType::Blob{length} => {
                builder.push_str(" BLOB");
                self._write_optional_display(builder, length);
            },
            DataType::Text{length} => {
                builder.push_str(" TEXT");
                self._write_optional_display(builder, length);
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
                self._write(builder, *values);
                builder.push_str(")");
            },
            DataType::Set{values} => {
                builder.push_str(" SET(");
                self._write(builder, *values);
                builder.push_str(")");
            },
            // _ => panic!("Unsupported data type {:?}", data_type)

        }
    }

    fn _write_key_definition(&self, builder:  &mut String, key: SQLKeyDef) {
        match key {
            SQLKeyDef::Primary{symbol, name, columns} => {

                if symbol.is_some() {
                    builder.push_str(&" CONSTRAINT");
                    self._write(builder, *symbol.unwrap());
                }

                builder.push_str(&" PRIMARY KEY");
                if name.is_some() {
                    self._write(builder, *name.unwrap());
                }
                self._write_key_column_list(builder, columns);
            },
            SQLKeyDef::Unique{symbol, name, columns} => {
                if symbol.is_some() {
                    builder.push_str(&" CONSTRAINT");
                    self._write(builder, *symbol.unwrap());
                }

                builder.push_str(&" UNIQUE KEY");
                if name.is_some() {
                    self._write(builder, *name.unwrap());
                }
                self._write_key_column_list(builder, columns);
            },
            SQLKeyDef::FullText{name, columns} => {
                builder.push_str(&" FULLTEXT KEY");
                if name.is_some() {
                    self._write(builder, *name.unwrap());
                }
                self._write_key_column_list(builder, columns);
            },
            SQLKeyDef::Index{name, columns} => {
                builder.push_str(&" KEY");
                if name.is_some() {
                    self._write(builder, *name.unwrap());
                }
                self._write_key_column_list(builder, columns);
            },
            SQLKeyDef::Foreign{symbol, name, columns, reference_table, reference_columns} => {
                if symbol.is_some() {
                    builder.push_str(&" CONSTRAINT");
                    self._write(builder, *symbol.unwrap());
                }

                builder.push_str(&" FOREIGN KEY");
                if name.is_some() {
                    self._write(builder, *name.unwrap());
                }
                self._write_key_column_list(builder, columns);

                builder.push_str(&" REFERENCES");
                self._write(builder, *reference_table);
                self._write_key_column_list(builder, reference_columns);
            }
        }
    }

    fn _write_table_option(&self, builder:  &mut String, option: TableOption) {
        match option {
            TableOption::Comment(e) => {
                builder.push_str(" COMMENT");
                self._write(builder, *e);
            },
            TableOption::Charset(e) => {
                builder.push_str(" DEFAULT CHARSET");
                self._write(builder, *e);
            },
            TableOption::Engine(e) => {
                builder.push_str(" ENGINE");
                self._write(builder, *e);
            },
            TableOption::AutoIncrement(e) => {
                builder.push_str(" AUTO_INCREMENT");
                self._write(builder, *e);
            }
        }
    }

    fn _write_key_column_list(&self, builder: &mut String, list: Vec<SQLExpr>) {
        builder.push_str(&" (");
        let mut sep = "";
        for c in list {
            builder.push_str(sep);
            self._write(builder, c);
            sep = ", ";
        }
        builder.push_str(&")");
    }

    fn _write_column_qualifier(&self, builder:  &mut String, q: ColumnQualifier) {
        match q {
            ColumnQualifier::CharacterSet(box e) => {
                builder.push_str(&" CHARACTER SET");
                self._write(builder, e);
            },
            ColumnQualifier::Collate(box e) => {
                builder.push_str(&" COLLATE");
                self._write(builder, e);
            },
            ColumnQualifier::Default(box e) => {
                builder.push_str(&" DEFAULT");
                self._write(builder, e);
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
                self._write(builder, e);
            },
            ColumnQualifier::Comment(box e) => {
                builder.push_str(&" COMMENT");
                self._write(builder, e);
            }
        }
    }

    fn _write_optional_display(&self, builder: &mut String, display: Option<u32>) {
        match display {
            Some(d) => {write!(builder, "({})", d).unwrap();},
            None => {}
        }
        ()
    }

    fn _write_optional_precision_and_scale(&self, builder: &mut String, precision: Option<u32>, scale: Option<u32>) {
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
