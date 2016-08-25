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
		&SQLExpr::SQLIdentifier(ref t) => Ok(Box::new(RexIdentifier{name: t.clone()})),
		_ => Err(String::from(format!("Unsuppported expr to rex {:?}", node)))
	}
}
