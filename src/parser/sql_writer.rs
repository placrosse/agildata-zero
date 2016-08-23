use super::sql_parser::{SQLExpr, LiteralExpr, SQLOperator,
    SQLUnionType, SQLJoinType, DataType, ColumnQualifier,
    SQLKeyDef, TableOption};
use std::fmt::Write;

pub trait Writer {
    fn write(&self, node: &SQLExpr) -> Result<String, String>;
	fn _write(&self, builder: &mut String, node: &SQLExpr) -> Result<(), String>;
}

pub trait ExprWriter {
    fn write(&self, writer: &Writer, builder: &mut String, node: &SQLExpr) -> Result<bool, String>;
}

pub struct SQLWriter<'a> {
    variants: Vec<&'a ExprWriter>
}

impl<'a> Default for SQLWriter<'a> {
	fn default() -> Self {SQLWriter::new(vec![])}
}

impl<'a> SQLWriter<'a> {
	pub fn new(variants: Vec<&'a ExprWriter>) -> Self {
		SQLWriter{variants: variants}
	}
}

impl<'a> Writer for SQLWriter<'a> {
    fn write(&self, node: &SQLExpr) -> Result<String, String> {
        let mut builder = String::new();
        self._write(&mut builder, node)?;
        Ok(builder)
    }

    fn _write(&self, builder: &mut String, node: &SQLExpr) -> Result<(), String> {
		for v in self.variants.iter() {
			if v.write(self, builder, node)? {
				return Ok(())
			}
		}

		if !(DefaultSQLWriter{}.write(self, builder, node)?) {
			Err(String::from(format!("No provided ExprWriter writes expr {:?}", node)))
		} else {
			Ok(())
		}
    }
}

struct DefaultSQLWriter{}

impl ExprWriter for DefaultSQLWriter {
	fn write(&self, writer: &Writer, builder: &mut String, node: &SQLExpr) -> Result<bool, String> {
		match node {
			&SQLExpr::SQLSelect{box ref expr_list, ref relation, ref selection, ref order} => {
				builder.push_str("SELECT");
				writer._write(builder, expr_list)?;
				match relation {
					&Some(box ref e) => {
						builder.push_str(" FROM");
						writer._write(builder, e)?
					},
					&None => {}
				}
				match selection {
					&Some(box ref e) => {
						builder.push_str(" WHERE");
						writer._write(builder, e)?
					},
					&None => {}
				}
				match order {
					&Some(box ref e) => {
						builder.push_str(" ORDER BY");
						writer._write(builder, e)?
					},
					&None => {}
				}

			},
			&SQLExpr::SQLInsert{box ref table, box ref column_list, box ref values_list} => {
				builder.push_str("INSERT INTO");
				writer._write(builder, table)?;
				builder.push_str(" (");
				writer._write(builder, column_list)?;
				builder.push_str(") VALUES(");
				writer._write(builder, values_list)?;
				builder.push_str(")");
			},
			&SQLExpr::SQLUpdate{box ref table, box ref assignments, ref selection} => {
				builder.push_str("UPDATE");
				writer._write(builder, table)?;
				builder.push_str(" SET");
				writer._write(builder, assignments)?;
				match selection {
					&Some(box ref e) => {
						builder.push_str(" WHERE");
						writer._write(builder, e)?
					},
					&None => {}
				}
			},
			&SQLExpr::SQLCreateTable{box ref table, ref column_list, ref keys, ref table_options} => {
				builder.push_str("CREATE TABLE");
				writer._write(builder, table)?;

				builder.push_str(&" (");
				let mut sep = "";
				for c in column_list {
					builder.push_str(sep);
					writer._write(builder, c)?;
					sep = ", ";
				}

				for k in keys {
					builder.push_str(sep);
					self._write_key_definition(writer, builder, k)?;
					sep = ", ";
				}

				builder.push_str(&")");

				sep = " ";
				for o in table_options {
					builder.push_str(sep);
					self._write_table_option(writer, builder, o)?;
				}
			},
			&SQLExpr::SQLColumnDef{box ref column, ref data_type, ref qualifiers} => {
				writer._write(builder, column)?;
				self._write_data_type(writer, builder, data_type)?;
				match qualifiers {
					&Some(ref e) => {
						for q in e.iter() {
							self._write_column_qualifier(writer, builder, q)?;
						}
					},
					&None => {}
				}

			},
			&SQLExpr::SQLExprList(ref vector) => {
				let mut sep = "";
				for e in vector.iter() {
					builder.push_str(sep);
					writer._write(builder, e)?;
					sep = ",";
				}
			},
			&SQLExpr::SQLBinary{box ref left, ref op, box ref right} => {
				writer._write(builder, left)?;
				self._write_operator(builder, op);
				writer._write(builder, right)?;

			},
			&SQLExpr::SQLLiteral(ref lit) => match lit {
				&LiteralExpr::LiteralLong(_, ref l) => {
					write!(builder, " {}", l).unwrap()
				},
				&LiteralExpr::LiteralBool(_, ref b) => {
					write!(builder, "{}", b).unwrap();
				},
				&LiteralExpr::LiteralDouble(_, ref d) => {
					write!(builder, "{}", d).unwrap();
				},
				&LiteralExpr::LiteralString(_, ref s) => {
					write!(builder, " '{}'", s).unwrap()
				}
				//_ => panic!("Unsupported literal for writing {:?}", lit)
			},
			&SQLExpr::SQLAlias{box ref expr, box ref alias} => {
				writer._write(builder, expr)?;
				builder.push_str(" AS");
				writer._write(builder, alias)?;
			},
			&SQLExpr::SQLIdentifier(ref id) => {
				write!(builder, " {}", id).unwrap();
			},
			&SQLExpr::SQLNested(box ref expr) => {
				builder.push_str("(");
				writer._write(builder, expr)?;
				builder.push_str(")");
			},
			&SQLExpr::SQLUnary{ref operator, box ref expr} => {
				self._write_operator(builder, operator);
				writer._write(builder, expr)?;
			},
			&SQLExpr::SQLOrderBy{box ref expr, ref is_asc} => {
				writer._write(builder, expr)?;
				if !is_asc {
					builder.push_str(" DESC");
				}
			},
			&SQLExpr::SQLJoin{box ref left, ref join_type, box ref right, ref on_expr} => {
				writer._write(builder, left)?;
				self._write_join_type(builder, join_type);
				writer._write(builder, right)?;
				match on_expr {
					&Some(box ref e) => {
						builder.push_str(" ON");
						writer._write(builder, e)?;
					},
					&None => {}
				}
			},
			&SQLExpr::SQLUnion{box ref left, ref union_type, box ref right} => {
				writer._write(builder, left)?;
				self._write_union_type(builder, union_type);
				writer._write(builder, right)?;
			}
			//_ => panic!("Unsupported node for writing {:?}", node)
		}

		Ok(true)
	}
}

impl DefaultSQLWriter {
	fn _write_operator(&self, builder: &mut String, op: &SQLOperator) {
        let op_text = match op {
            &SQLOperator::ADD => "+",
            &SQLOperator::SUB => "-",
            &SQLOperator::MULT => "*",
            &SQLOperator::DIV => "/",
            &SQLOperator::MOD => "%",
            &SQLOperator::GT => ">",
            &SQLOperator::LT => "<",
            // SQLOperator::GTEQ => ">=",
            // SQLOperator::LTEQ => "<=",
            &SQLOperator::EQ => "=",
            // SQLOperator::NEQ => "!=",
            &SQLOperator::OR => "OR",
            &SQLOperator::AND  => "AND"
        };
        write!(builder, " {}", op_text).unwrap();
    }

    fn _write_join_type(&self, builder: &mut String, join_type: &SQLJoinType) {
        let text = match join_type {
            &SQLJoinType::INNER => "INNER JOIN",
            &SQLJoinType::LEFT => "LEFT JOIN",
            &SQLJoinType::RIGHT => "RIGHT JOIN",
            &SQLJoinType::FULL => "FULL OUTER JOIN",
            &SQLJoinType::CROSS => "CROSS JOIN"
        };
        write!(builder, " {}", text).unwrap();
    }

    fn _write_union_type(&self, builder: &mut String, union_type: &SQLUnionType) {
        let text = match union_type {
            &SQLUnionType::UNION => "UNION",
            &SQLUnionType::ALL => "UNION ALL",
            &SQLUnionType::DISTINCT => "UNION DISTINCT"
        };
        write!(builder, " {} ", text).unwrap();
    }

    fn _write_data_type(&self, writer: &Writer, builder: &mut String, data_type: &DataType) -> Result<(), String> {
        match data_type {
            &DataType::Bit{ref display} => {
                builder.push_str(" BIT");
                self._write_optional_display(builder, display);
            },
            &DataType::TinyInt{ref display} => {
                builder.push_str(" TINYINT");
                self._write_optional_display(builder, display);
            },
            &DataType::SmallInt{ref display} => {
                builder.push_str(" SMALLINT");
                self._write_optional_display(builder, display);
            },
            &DataType::MediumInt{ref display} => {
                builder.push_str(" MEDIUMINT");
                self._write_optional_display(builder, display);
            },
            &DataType::Int{ref display} => {
                builder.push_str(" INTEGER");
                self._write_optional_display(builder, display);
            },
            &DataType::BigInt{ref display} => {
                builder.push_str(" BIGINT");
                self._write_optional_display(builder, display);
            },
            &DataType::Decimal{ref precision, ref scale} => {
                builder.push_str(" DECIMAL");
                self._write_optional_precision_and_scale(builder, precision, scale);
            },
            &DataType::Float{ref precision, ref scale} => {
                builder.push_str(" FLOAT");
                self._write_optional_precision_and_scale(builder, precision, scale);
            },
            &DataType::Double{ref precision, ref scale} => {
                builder.push_str(" DOUBLE");
                self._write_optional_precision_and_scale(builder, precision, scale);
            },
            &DataType::Bool => {
                builder.push_str(" BOOLEAN");
            },
            &DataType::Date => {
                builder.push_str(" DATE");
            },
            &DataType::DateTime{ref fsp} => {
                builder.push_str(" DATETIME");
                self._write_optional_display(builder, fsp);
            },
            &DataType::Timestamp{ref fsp} => {
                builder.push_str(" TIMESTAMP");
                self._write_optional_display(builder, fsp);
            },
            &DataType::Time{ref fsp} => {
                builder.push_str(" TIME");
                self._write_optional_display(builder, fsp);
            },
            &DataType::Year{ref display} => {
                builder.push_str(" YEAR");
                self._write_optional_display(builder, display);
            },
            &DataType::Char{ref length} => {
                builder.push_str(" CHAR");
                self._write_optional_display(builder, length);
            },
            &DataType::NChar{ref length} => {
                builder.push_str(" NCHAR");
                self._write_optional_display(builder, length);
            },
            &DataType::CharByte{ref length} => {
                builder.push_str(" CHAR");
                self._write_optional_display(builder, length);
                builder.push_str(" BYTE");
            },
            &DataType::Varchar{ref length} => {
                builder.push_str(" VARCHAR");
                self._write_optional_display(builder, length);
            },
            &DataType::NVarchar{ref length} => {
                builder.push_str(" NVARCHAR");
                self._write_optional_display(builder, length);
            },
            &DataType::Binary{ref length} => {
                builder.push_str(" BINARY");
                self._write_optional_display(builder, length);
            },
            &DataType::VarBinary{ref length} => {
                builder.push_str(" VARBINARY");
                self._write_optional_display(builder, length);
            },
            &DataType::Blob{ref length} => {
                builder.push_str(" BLOB");
                self._write_optional_display(builder, length);
            },
            &DataType::Text{ref length} => {
                builder.push_str(" TEXT");
                self._write_optional_display(builder, length);
            },
            &DataType::TinyBlob => {
                builder.push_str(" TINYBLOB");
            },
            &DataType::TinyText => {
                builder.push_str(" TINYTEXT");
            },
            &DataType::MediumBlob => {
                builder.push_str(" MEDIUMBLOB");
            },
            &DataType::MediumText => {
                builder.push_str(" MEDIUMTEXT");
            },
            &DataType::LongBlob => {
                builder.push_str(" LONGBLOB");
            },
            &DataType::LongText => {
                builder.push_str(" LONGTEXT");
            },
            &DataType::Enum{box ref values} => {
                builder.push_str(" ENUM(");
                writer._write(builder, values)?;
                builder.push_str(")");
            },
            &DataType::Set{box ref values} => {
                builder.push_str(" SET(");
                writer._write(builder, values)?;
                builder.push_str(")");
            },
            // _ => panic!("Unsupported data type {:?}", data_type)

        }

		Ok(())
    }

    fn _write_key_definition(&self, writer: &Writer, builder:  &mut String, key: &SQLKeyDef) -> Result<(), String> {
        match key {
            &SQLKeyDef::Primary{ref symbol, ref name, ref columns} => {

				match symbol {
					&Some(box ref e) => {
						builder.push_str(&" CONSTRAINT");
						writer._write(builder, e)?;
					},
					&None => {}
				}

                builder.push_str(&" PRIMARY KEY");
				match name {
					&Some(box ref e) => {
						writer._write(builder, e)?;
					},
					&None => {}
				}
                self._write_key_column_list(writer, builder, columns)?;
            },
            &SQLKeyDef::Unique{ref symbol, ref name, ref columns} => {
				match symbol {
					&Some(box ref e) => {
						builder.push_str(&" CONSTRAINT");
						writer._write(builder, e)?;
					},
					&None => {}
				}

                builder.push_str(&" UNIQUE KEY");
				match name {
					&Some(box ref e) => {
						writer._write(builder, e)?;
					},
					&None => {}
				}
                self._write_key_column_list(writer, builder, columns)?;
            },
            &SQLKeyDef::FullText{ref name, ref columns} => {
                builder.push_str(&" FULLTEXT KEY");
				match name {
					&Some(box ref e) => {
						writer._write(builder, e)?;
					},
					&None => {}
				}
                self._write_key_column_list(writer, builder, columns)?;
            },
            &SQLKeyDef::Index{ref name, ref columns} => {
                builder.push_str(&" KEY");
				match name {
					&Some(box ref e) => {
						writer._write(builder, e)?;
					},
					&None => {}
				}
                self._write_key_column_list(writer, builder, columns)?;
            },
            &SQLKeyDef::Foreign{ref symbol, ref name, ref columns, box ref reference_table, ref reference_columns} => {
				match symbol {
					&Some(box ref e) => {
						builder.push_str(&" CONSTRAINT");
						writer._write(builder, e)?;
					},
					&None => {}
				}

                builder.push_str(&" FOREIGN KEY");
				match name {
					&Some(box ref e) => {
						writer._write(builder, e)?;
					},
					&None => {}
				}
                self._write_key_column_list(writer, builder, columns)?;

                builder.push_str(&" REFERENCES");
                writer._write(builder, &*reference_table)?;
                self._write_key_column_list(writer, builder, reference_columns)?;
            }
        }

		Ok(())
    }

    fn _write_table_option(&self, writer: &Writer, builder:  &mut String, option: &TableOption) -> Result<(), String> {
        match option {
            &TableOption::Comment(box ref e) => {
                builder.push_str(" COMMENT");
                writer._write(builder, e)?;
            },
            &TableOption::Charset(box ref e) => {
                builder.push_str(" DEFAULT CHARSET");
                writer._write(builder, e)?;
            },
            &TableOption::Engine(box ref e) => {
                builder.push_str(" ENGINE");
                writer._write(builder, e)?;
            },
            &TableOption::AutoIncrement(box ref e) => {
                builder.push_str(" AUTO_INCREMENT");
                writer._write(builder, e)?;
            }
        }

		Ok(())
    }

    fn _write_key_column_list(&self, writer: &Writer, builder: &mut String, list: &Vec<SQLExpr>) -> Result<(), String> {
        builder.push_str(&" (");
        let mut sep = "";
        for c in list {
            builder.push_str(sep);
            writer._write(builder, c)?;
            sep = ", ";
        }
        builder.push_str(&")");

		Ok(())
    }

    fn _write_column_qualifier(&self, writer: &Writer, builder:  &mut String, q: &ColumnQualifier) -> Result<(), String> {
        match q {
            &ColumnQualifier::CharacterSet(box ref e) => {
                builder.push_str(&" CHARACTER SET");
                writer._write(builder, e)?;
            },
            &ColumnQualifier::Collate(box ref e) => {
                builder.push_str(&" COLLATE");
                writer._write(builder, e)?;
            },
            &ColumnQualifier::Default(box ref e) => {
                builder.push_str(&" DEFAULT");
                writer._write(builder, e)?;
            },
            &ColumnQualifier::Signed => builder.push_str(&" SIGNED"),
            &ColumnQualifier::Unsigned => builder.push_str(&" UNSIGNED"),
            &ColumnQualifier::Null => builder.push_str(&" NULL"),
            &ColumnQualifier::NotNull => builder.push_str(&" NOT NULL"),
            &ColumnQualifier::AutoIncrement => builder.push_str(&" AUTO_INCREMENT"),
            &ColumnQualifier::PrimaryKey => builder.push_str(&" PRIMARY KEY"),
            &ColumnQualifier::UniqueKey => builder.push_str(&" UNIQUE"),
            &ColumnQualifier::OnUpdate(box ref e) => {
                builder.push_str(&" ON UPDATE");
                writer._write(builder, e)?;
            },
            &ColumnQualifier::Comment(box ref e) => {
                builder.push_str(&" COMMENT");
                writer._write(builder, e)?;
            }
        }

		Ok(())
    }

    fn _write_optional_display(&self, builder: &mut String, display: &Option<u32>) {
        match display {
            &Some(ref d) => {write!(builder, "({})", d).unwrap();},
            &None => {}
        }
    }

    fn _write_optional_precision_and_scale(&self, builder: &mut String, precision: &Option<u32>, scale: &Option<u32>) {
        match precision {
            &Some(ref p) => {
                write!(builder, "({}", p).unwrap();
                if scale.is_some() {
                    write!(builder, ",{}", scale.unwrap()).unwrap();
                }
                builder.push_str(")");
            },
            &None => {}
        }
        ()
    }
}
