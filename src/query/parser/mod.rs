use super::dialects::Dialect;
use super::tokenizer::IToken;
use super::planner::IRel;

pub trait Parser<D: Dialect<T, A, R>, T: IToken, A: IAST, R: IRel> {
	fn parse(&self, dialects: D, tokens: Vec<T>) -> Result<Option<ASTNode<A>>, String>;
}

pub enum ASTNode<A: IAST> {
	AST(A)
}
pub trait IAST{}
