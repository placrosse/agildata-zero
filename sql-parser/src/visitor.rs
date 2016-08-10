use super::sql_parser::{SQLExpr, LiteralExpr, SQLOperator, SQLUnionType, SQLJoinType};

pub trait SQLExprVisitor {
	fn visit_sql_expr(&mut self, &SQLExpr);
	fn visit_sql_lit_expr(&mut self, &LiteralExpr);
	fn visit_sql_operator(&mut self, &SQLOperator);
}
