use super::{Rel, RelNode, RelConsumer, RelProducer, RexNode, TupleType};

#[derive(Debug)]
pub struct Dual {
	tt: TupleType
}
impl Rel for Dual {
	fn as_producer(&self) -> Option<&RelProducer> {Some(self)}
	fn as_consumer(&self) -> Option<&RelConsumer> {None}
}

impl<'a> RelConsumer<'a> for Dual {
	fn get_child_nodes(&'a self) -> Vec<&'a RelNode> {vec![]}
}
impl<'a> RelProducer<'a> for Dual {
	fn get_tuple_type(&'a self) -> &TupleType {
		&self.tt
	}
}

impl Dual {
	pub fn new() -> Self {Dual{tt: TupleType::new(vec![])}}
}

#[derive(Debug)]
pub struct TableScan {
	pub name: String,
	pub tt: TupleType
}

impl Rel for TableScan{
	fn as_producer(&self) -> Option<&RelProducer> {Some(self)}
	fn as_consumer(&self) -> Option<&RelConsumer> {Some(self)}
}
impl<'a> RelConsumer<'a> for TableScan {
	fn get_child_nodes(&'a self) -> Vec<&'a RelNode> {
		vec![]
	}
}

impl<'a> RelProducer<'a> for TableScan {
	fn get_tuple_type(&'a self) -> &TupleType {
		&self.tt
	}
}

#[derive(Debug)]
pub struct Projection {
	pub project_list: RexNode,
	pub input: RelNode
}

impl<'a> Rel for Projection {
	fn as_producer(&self) -> Option<&RelProducer> {Some(self)}
	fn as_consumer(&self) -> Option<&RelConsumer> {Some(self)}
}

impl<'a> RelConsumer<'a> for Projection {
	fn get_child_nodes(&'a self) -> Vec<&'a RelNode> {
		vec![&self.input]
	}
}

impl<'a> RelProducer<'a> for Projection {
	fn get_tuple_type(&'a self) -> &TupleType {
		&self.input.as_producer().unwrap().get_tuple_type()
	}
}
