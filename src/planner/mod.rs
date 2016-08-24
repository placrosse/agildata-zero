use parser::sql_parser::*;
use std::fmt::Debug;

mod rex;
mod rel;
mod default_planner;

pub trait Rex: Debug {}
pub type RexNode = Box<Rex>;


pub trait Rel: Debug {}
pub type RelNode = Box<Rel>;

pub trait RelConsumer<'a> {
	fn get_child_nodes(&'a self) -> Vec<&'a RelNode>;
}

pub trait Planner {
	fn plan(&self, node: &SQLExpr) -> Result<RelNode, String>;
}
