mod tokenizer;
mod parser;
mod planner;
mod dialects;

// use parser::sql_parser::*;
//
// pub enum RelNode<T: Rel> {
// 	RelExtension{r: T}
// }
//
// pub trait Rel {}
//
// trait PlannerDialect<T> where T: Rel {
// 	fn plan_expr(&self, planner: &QueryPlanner, node: &SQLExpr) -> T;
// }
//
// trait Expr {
//
// }
// enum TestSQLExpr<T> where T: Expr{
// 	SQLLiteral(String),
// 	SQLIdentifier(String),
// 	SQLExtension(T)
// }
//
// trait Dialect<E, R> {
// 	fn parse(expr: E) -> Result<Option<R>, String>>;
// }
//
// struct DefaultDialect {
// }
//
// struct MySQLDialect {
// }
//
// struct Planner { //}<D> where D: Dialect {
// 	default: Dialect,
// 	variant: Dialect
// }
//
// let dialects = vec![MySQL, ANSIISQL]
//"select 1".tokenize(&dialects).parse().plan().write()
// impl Tokenizer<T> for String where T: Dialect {
//
// 	fn tokenize(dialects: Vec<T>) -> TokenStream;
//
// }
//
// struct TokenStream {
// 	tokens: Vec<Token>,
// 	dialects: &'a Vec<T>
// }
//
//
// impl Parser for Vec<Token> {
// }
//
// impl Planner for ParsedExpr {
//
// }
//
// impl Planner {
// 	fn plan(sql: String) -> Result<Option<RelNode>, String> {
// 		match dialect.parse(&self, expr) {
// 			Some(e) => Ok(e),
// 			None => default.parse(&self,expr)
// 		}
// 	}
// }
//
// // struct QueryPlanner<T> where T: Rel{}
// //
// // impl<T> QueryPlanner<T> where T: Rel {
// // 	fn plan(&self, dialect: &PlannerDialect<T>, node: &SQLExpr) -> RelNode {
// // 		panic!("Here")
// // 	}
// // }
