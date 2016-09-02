use query::planner::{Rel, Rex, RelVisitor, TupleType, HasTupleType};
use query::Operator;
use std::collections::HashMap;
use std::error::Error;

pub struct EncryptVisitor {
	pub valuemap: HashMap<u32, Result<Vec<u8>, Box<Error>>>
}

impl RelVisitor for EncryptVisitor  {
	fn visit_rel(&mut self, rel: &Rel) -> Result<(), String> {
		match rel {
			&Rel::Projection{box ref project, box ref input, ref tt} => {
				self.visit_rex(project, tt);
				self.visit_rel(input);
			},
			&Rel::Selection{box ref expr, box ref input} => {
				self.visit_rex(expr, input.tt());
				self.visit_rel(input);
			},
			&Rel::TableScan{ref table, ref tt} => {},
			&Rel::Dual{ref tt} => {}
		}
		Ok(())
	}

	fn visit_rex(&mut self, rex: &Rex, tt: &TupleType) -> Result<(), String> {
		match rex {
			&Rex::BinaryExpr{box ref left, ref op, box ref right} => {
				match op {
					&Operator::AND | &Operator::OR => {
						self.visit_rex(left, tt);
						self.visit_rex(right, tt);
					}
					_ => {
						match (left, right) {
							(&Rex::Identifier{ref id, ref el}, &Rex::Literal(_)) => {
								match op {
									&Operator::EQ => {},
									_ => return Err(format!("Operator {:?} not supported for encrypted column {}", op, el.name))
								}
							},
							(&Rex::Literal(_), &Rex::Identifier{..}) => {},
							_ => {} // Dont care
						}
					} // TBD
				}
			},
			_ => {} // TODO
		}
		Ok(())
	}
}
