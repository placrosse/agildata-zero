use super::{Planner, RelNode, Rel};
use parser::sql_parser::*;
use super::rel::*;
use super::rex;

struct DefaultQueryPlanner{}

impl Planner for DefaultQueryPlanner {
	fn plan(&self, node: &SQLExpr) -> Result<RelNode, String> {
		match node {
			&SQLExpr::SQLSelect{box ref expr_list, ref relation, ref selection, ref order} => {
				let mut input = match relation {
					&Some(box ref expr) => self.plan(expr)?,
					_ => Box::new(Dual{})
				};

				// TODO support
				if selection.is_some() {
					return Err(String::from("WHERE clause unsupported"))
				}

				if order.is_some() {
					return Err(String::from("ORDER BY clause unsupported"))
				}

				let project_list = rex::to_rex(expr_list)?;

				Ok(Box::new(
					Projection{
						project_list: project_list,
						input: input
					}
				))
			},
			&SQLExpr::SQLIdentifier(ref name) => {
				// TODO actually validate this.
				Ok(Box::new(TableScan{name: name.clone()}))
			}
			_ => Err(String::from(format!("No planner support for SQLExpr: {:?}", node)))
		}
	}
}

#[cfg(test)]
mod tests {
	use super::DefaultQueryPlanner;
	use super::super::{Planner, RelNode, Rel};
	use parser::sql_parser::*;
	use super::super::rel::*;

	#[test]
	fn test_simple_crud() {
		let parser = AnsiSQLParser {};
		let sql = "SELECT id, first_name, last_name, ssn, age, sex FROM users";
		let parsed = parser.parse(sql).unwrap();

		let planner = DefaultQueryPlanner{};
		let plan = planner.plan(&parsed).unwrap();

		println!("PLAN {:#?}", plan);
	}
}
