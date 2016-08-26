use super::{Planner, RelNode, TupleType, RelType, Element};
use parser::sql_parser::*;
use super::rel::*;
use super::rex;
use super::types::*;
use config::*;
use encrypt::{NativeType};

// TODO config isn't really right. Need a real meta system
struct DefaultQueryPlanner<'a> {
	config: &'a Config
}

impl<'a> Planner for DefaultQueryPlanner<'a> {
	fn plan(&self, node: &SQLExpr) -> Result<RelNode, String> {
		match node {
			&SQLExpr::SQLSelect{box ref expr_list, ref relation, ref selection, ref order} => {
				let mut input = match relation {
					&Some(box ref expr) => self.plan(expr)?,
					_ => Box::new(Dual::new())
				};

				// TODO support
				if selection.is_some() {
					return Err(String::from("WHERE clause unsupported"))
				}

				if order.is_some() {
					return Err(String::from("ORDER BY clause unsupported"))
				}

				let project_list = rex::to_rex(expr_list, input.as_producer().unwrap().get_tuple_type())?;

				Ok(Box::new(
					Projection{
						project_list: project_list,
						input: input
					}
				))
			},
			&SQLExpr::SQLIdentifier{ref id, ref parts} => {
				// TODO planner needs default schema
				let default_schema = String::from("babel");
				let schema = if parts.len() > 1 {
					&parts[0]
				} else {
					&default_schema
				};

				// TODO actually validate this.
				Ok(Box::new(TableScan{
					name: id.clone(),
					tt: self.get_table_tuple_type(&schema, &id)?
				}))
			}
			_ => Err(String::from(format!("No planner support for SQLExpr: {:?}", node)))
		}
	}
}

impl<'a> DefaultQueryPlanner<'a> {

	fn get_table_tuple_type(&self, schema: &String, table: &String) -> Result<TupleType, String> {
		let table_config = self.config.get_table_config(schema, table);

		// TODO hashmap not ordered by index, also may not include some columns
		match table_config {
			Some(config) => {
				let mut elements: Vec<Element> = Vec::new();
				for (c_name, c_config) in config.column_map.iter() {
					elements.push(
						Element {
							name: c_name.clone(),
							relation: table.clone(),
							data_type: self.reconcile_data_type(&c_config.native_type)?,
							p_name: None,
							p_relation: None
						}
					);
				}

				Ok(TupleType{elements: elements})
			},
			_ => Err(String::from(format!("Invalid table {}.{}", schema, table)))
		}
	}

	// TODO convert to planner types or just use encrypt types?
	fn reconcile_data_type(&self, native_type: &NativeType) -> Result<RelType, String> {
		match native_type {
			&NativeType::U64 => Ok(Box::new(U64{})),
			&NativeType::Varchar(_) => Ok(Box::new(VarChar{})),
			_ => Err(String::from(format!("Unsupported type conversion {:?}", native_type)))
		}
	}
}

#[cfg(test)]
mod tests {
	use super::DefaultQueryPlanner;
	use super::super::{Planner};
	use parser::sql_parser::*;
	use super::super::rel::*;
	use config;

	#[test]
	fn test_simple_crud() {
		let config = config::parse_config("example-babel-config.xml");

		let parser = AnsiSQLParser {};
		let sql = "SELECT id, first_name, last_name, ssn, age, sex FROM users";
		let parsed = parser.parse(sql).unwrap();

		let planner = DefaultQueryPlanner{
			config: &config
		};

		let plan = planner.plan(&parsed).unwrap();

		match plan {
			box Projection {..} => println!("A Projection"),
			_ => panic!(format!("HERER {:?}", plan))
		}

		println!("PLAN {:#?}", plan);
	}
}
