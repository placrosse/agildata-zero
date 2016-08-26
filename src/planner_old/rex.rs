use super::{Rex, RexNode, TupleType};
use parser::sql_parser::*;

#[derive(Debug)]
struct RexIdentifier {
	name: String
}
impl Rex for RexIdentifier {}

#[derive(Debug)]
struct RexExprList {
	rex_list: Vec<RexNode>
}
impl Rex for RexExprList {}

// TODO how needed is rex, really?
pub fn to_rex(node: &SQLExpr, tt: &TupleType) -> Result<RexNode, String> {
	match node {
		&SQLExpr::SQLExprList(ref list) => {
			let mut rexs: Vec<RexNode> = Vec::new();
			for e in list.iter() {
				rexs.push(to_rex(e, tt)?)
			}
			Ok(Box::new(RexExprList{
				rex_list: rexs
			}))
		}
		&SQLExpr::SQLIdentifier{ref id, ref parts} => {
			let relation = if parts.len() > 1 {
				Some(&parts[parts.len() - 2])
			} else {
				None
			};

			let name = &parts[parts.len() - 1];

			// TODO implement better solution than iteration
			for e in tt.elements.iter() {
				match relation {
					Some(rel) => {
						match e.p_relation {
							Some(ref a) => {return Err(String::from("Aliasing not implemented"))},
							None => {return Err(String::from("Qualification not implemented"))}
						}
					},
					None => {
						match e.p_name {
							Some(ref v) => {
								if v.to_uppercase() == name.to_uppercase() {
									return Ok(Box::new(RexIdentifier{name: id.clone()}))
								}
							},
							None => {
								if e.name.to_uppercase() == name.to_uppercase() {
									return Ok(Box::new(RexIdentifier{name: id.clone()}))
								}
							}
						}
					}
				}
			}
			Err(String::from(format!("Invalid identifier {} for tuple type {:?}", id, tt)))

		},
		_ => Err(String::from(format!("Unsuppported expr to rex {:?}", node)))
	}
}
