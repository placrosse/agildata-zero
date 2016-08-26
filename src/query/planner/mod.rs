use super::dialects::Dialect;
use super::tokenizer::IToken;
use super::parser::IAST;

pub trait Planner<D: Dialect<T, A, R>, T: IToken, A: IAST, R: IRel> {
	fn plan(&self, dialects: D, ast: A) -> Result<Option<RelNode<R>>, String>;
}

pub trait IRel {}
pub enum RelNode<R: IRel> {
	Rel(R)
}
