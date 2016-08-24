use super::{Rel, RelNode, RelConsumer, RexNode};

#[derive(Debug)]
pub struct Dual {}
impl Rel for Dual {}
impl<'a> RelConsumer<'a> for Dual {
	fn get_child_nodes(&'a self) -> Vec<&'a RelNode> {
		vec![]
	}
}

#[derive(Debug)]
pub struct TableScan {
	pub name: String
}

impl Rel for TableScan{}
impl<'a> RelConsumer<'a> for TableScan {
	fn get_child_nodes(&'a self) -> Vec<&'a RelNode> {
		vec![]
	}
}

#[derive(Debug)]
pub struct Projection {
	pub project_list: RexNode,
	pub input: RelNode
}

impl<'a> Rel for Projection {}
impl<'a> RelConsumer<'a> for Projection {
	fn get_child_nodes(&'a self) -> Vec<&'a RelNode> {
		vec![&self.input]
	}
}
