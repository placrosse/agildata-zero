use parser::sql_parser::*;
use std::fmt::Debug;

mod types;
mod rex;
mod rel;
mod default_planner;

pub trait Rex: Debug {}
pub type RexNode = Box<Rex>;


pub trait Rel: Debug {
	fn as_producer(&self) -> Option<&RelProducer>;
}
pub type RelNode = Box<Rel>;

#[derive(Debug)]
pub struct TupleType {
	pub elements: Vec<Element>
}

impl TupleType {
	fn new(elements: Vec<Element>) -> Self {
		TupleType{elements: elements}
	}
}

// p_ denotes provinence
#[derive(Debug)]
pub struct Element {
	name: String,
	relation: String,
	data_type: RelType,
	p_name: Option<String>,
	p_relation: Option<String>
}

pub trait Type: Debug{}
pub type RelType = Box<Type>;

// TODO perhaps just combine these into 
pub trait RelConsumer<'a> {
	fn get_child_nodes(&'a self) -> Vec<&'a RelNode>;
}

pub trait RelProducer<'a> {
	fn get_tuple_type(&'a self) -> &'a TupleType;
}

pub trait Planner {
	fn plan(&self, node: &SQLExpr) -> Result<RelNode, String>;
}
