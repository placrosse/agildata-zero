use super::super::parser::sql_writer::*;
use super::super::parser::sql_parser::{SQLExpr, LiteralExpr};
use std::collections::HashMap;
use std::fmt::Write;
use config::*;

pub fn to_hex_string(bytes: &Vec<u8>) -> String {
  let strs: Vec<String> = bytes.iter()
                               .map(|b| format!("{:02X}", b))
                               .collect();
  strs.join("")
}

pub struct LiteralReplacingWriter<'a> {
    pub literals: &'a HashMap<u32, Option<Vec<u8>>>
}

impl<'a> ExprWriter for LiteralReplacingWriter<'a> {
	fn write(&self, writer: &Writer, builder: &mut String, node: &SQLExpr) -> Result<bool, String> {
		match node {
			&SQLExpr::SQLLiteral(ref e) => match e {
				&LiteralExpr::LiteralLong(ref i, _) => self.optionally_write_literal(i, builder),
    			&LiteralExpr::LiteralBool(ref i, _) => self.optionally_write_literal(i, builder),
    			&LiteralExpr::LiteralDouble(ref i, _) => self.optionally_write_literal(i, builder),
    			&LiteralExpr::LiteralString(ref i, _) => self.optionally_write_literal(i, builder)
			},
			_ => Ok(false)
		}
	}
}

impl<'a> LiteralReplacingWriter<'a> {
	fn optionally_write_literal(&self, index: &u32, builder: &mut String) -> Result<bool, String> {
		match self.literals.get(index) {
			Some(value) => match value {
				&Some(ref e) => {
					write!(builder, "X'{}'", to_hex_string(e)).unwrap();
					Ok(true)
				},
				&None => Ok(false),
			},
			None => Ok(false),
		}
	}
}

struct CreateTranslatingWriter<'a> {
	config: &'a Config
}

// QLCreateTable{
// 	table: Box<SQLExpr>,
// 	column_list: Vec<SQLExpr>,
// 	keys: Vec<SQLKeyDef>,
// 	table_options: Vec<TableOption>
// }
impl<'a> ExprWriter for CreateTranslatingWriter<'a> {
	fn write(&self, writer: &Writer, builder: &mut String, node: &SQLExpr) -> Result<bool, String> {
		match node {
			&SQLExpr::SQLCreateTable{box ref table, ref column_list, ref keys, ref table_options} => {
				Ok(false)
			},
			_ => Ok(false)
		}
	}
}
