use std::iter::Peekable;
use std::str::Chars;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU32, Ordering};

pub mod dialects;

#[cfg(test)]
mod tests;

// Dialect api
pub trait Dialect {

	fn get_keywords(&self) -> &'static [&'static str];

	fn get_token(&self, chars: &mut Peekable<Chars>, keywords: &Vec<&'static str>) -> Result<Option<Token>, String>;

	fn parse_prefix<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<Option<ASTNode>, String>;

	fn get_precedence<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<u8, String>;

	fn parse_infix<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>, left: ASTNode, precedence: u8) -> Result<Option<ASTNode>, String>;

	// fn plan();
}

// Tokenizer apis
pub trait Tokenizer<D: Dialect> {
	fn tokenize<'a>(&self, dialects: &'a Vec<D>) -> Result<Tokens<'a, D>, String>;
}

impl<D: Dialect> Tokenizer<D> for String {
	fn tokenize<'a>(&self, dialects: &'a Vec<D>) -> Result<Tokens<'a, D>, String> {

		let mut keywords: Vec<&'static str> = Vec::new();
		for d in dialects.iter() {
			keywords.extend_from_slice(d.get_keywords())
		}
		let mut chars = self.chars().peekable();
		let mut tokens: Vec<Token> = Vec::new();
		while let Some(&ch) = chars.peek() {
			match get_dialect_token(&dialects, &mut chars, &keywords)? {
				None => return Err(String::from(format!("No token dialect support for character {:?}", ch))),
				Some(token) => tokens.push(token)
			}
		}

		let stream = tokens
			.into_iter()
			.filter(|t| match t { &Token::Whitespace => false, _ => true })
			.collect::<Vec<_>>();

		Ok(Tokens::new(dialects, stream))
	}
}

fn get_dialect_token<D: Dialect> (dialects: &Vec<D>, chars: &mut Peekable<Chars>, keywords: &Vec<&'static str>) -> Result<Option<Token>, String> {
	for d in dialects.iter() {
		let token = d.get_token(chars, keywords)?;
		match token {
			Some(t) => {
				return Ok(Some(t));
			},
			None => {}
		}
	}
	Ok(None)
}

#[derive(Debug)]
pub struct Tokens<'a, D: 'a + Dialect> {
	pub dialects: &'a Vec<D>,
	pub tokens: Vec<Token>,
	pub index: AtomicU32
}

impl<'a, D: 'a + Dialect> Tokens<'a, D> {
	pub fn new(dialects: &'a Vec<D>, tokens: Vec<Token>) -> Self {
		Tokens {
			dialects: dialects,
			tokens: tokens,
			index: AtomicU32::new(0)
		}
	}

	pub fn peek(&self) -> Option<&Token> {
		let i = self.index.load(Ordering::SeqCst) as usize;
		if (i < (self.tokens.len())) {
			Some(&self.tokens[i as usize])
		} else {
			None
		}
	}

	pub fn next(&self) -> Option<&Token> {
		let i = self.index.load(Ordering::SeqCst) as usize;
		//println!("next() i={}", i);
		if (i < (self.tokens.len())) {
			self.index.fetch_add(1, Ordering::SeqCst);
			Some(&self.tokens[i as usize])
		} else {
			panic!("Index out of bounds")
		}
	}
}

#[derive(Debug,PartialEq,Clone)]
pub enum Token  {
	Whitespace,
	Keyword(String),
	Identifier(String),
	Literal(LiteralToken),
	Operator(String),
	Punctuator(String)
}

#[derive(Debug,PartialEq,Clone)]
pub enum LiteralToken {
    LiteralString(u32, String),
    LiteralLong(u32, String),
    LiteralDouble(u32, String),
    LiteralBool(u32, String),
}
// Parser APIs
pub trait Parser<D: Dialect> {
	fn parse(&self) -> Result<ASTNode, String>;
	fn parse_expr(&self, precedence: u8) -> Result<ASTNode, String>;
}


impl<'a, D: Dialect> Parser<D> for Tokens<'a, D> {
	fn parse(&self) -> Result<ASTNode, String> { self.parse_expr(0) }

	fn parse_expr(&self, precedence: u8) -> Result<ASTNode, String> {
		match get_dialect_ast(&self.dialects, self, precedence)? {
			Some(node) => Ok(node),
			None => Err(String::from("No dialect support for token prefix TBD")) // TODO
		}
	}

}

fn get_dialect_ast<'a, D: Dialect>(dialects: &Vec<D>, tokens: &Tokens<'a, D>, precedence: u8) ->
	Result<Option<ASTNode>, String> {

	let mut expr = get_dialect_prefix(dialects, tokens)?;
	if expr.is_some() {
		return get_dialect_infix(dialects, tokens, expr.unwrap(), precedence)
	} else {
		Ok(expr)
	}
}

fn get_dialect_prefix<'a, D: Dialect>
	(dialects: &Vec<D>, tokens: &Tokens<'a, D>) ->
	Result<Option<ASTNode>, String> {

	for d in dialects.iter() {
		let expr = d.parse_prefix(tokens)?;
		if expr.is_some() {
			return Ok(expr)
		}
	}

	Ok(None)
}

fn get_dialect_infix<'a, D: Dialect>(dialects: &Vec<D>, tokens: &Tokens<'a, D>, left: ASTNode, precedence: u8) ->
	Result<Option<ASTNode>, String> {

	for d in dialects.iter() {
		let next_precedence = d.get_precedence(tokens)?;

		if precedence >= next_precedence {
			continue;
		}
		match d.parse_infix(tokens, left, next_precedence)? {
			Some(e) => return Ok(Some(e)),
			None => return Err(String::from("Illegal state!"))
		}
	}

	Ok(Some(left))
}


#[derive(Debug, PartialEq)]
pub enum ASTNode {
	// ANSISQL nodes
	SQLIdentifier{id: String, parts: Vec<String>},
	SQLBinary{left: Box<ASTNode>, op: Box<ASTNode>, right: Box<ASTNode>},
	SQLNested(Box<ASTNode>),
	SQLUnary{operator: Box<ASTNode>, expr: Box<ASTNode>},
	SQLLiteral(LiteralExpr),
	SQLAlias{expr: Box<ASTNode>, alias: Box<ASTNode>},
	SQLOperator(Operator),
	SQLExprList(Vec<ASTNode>),
    SQLOrderBy{expr: Box<ASTNode>, is_asc: bool},
    SQLSelect{
        expr_list: Box<ASTNode>,
        relation: Option<Box<ASTNode>>,
        selection: Option<Box<ASTNode>>,
        order: Option<Box<ASTNode>>
    },
    SQLInsert {
        table: Box<ASTNode>,
        column_list: Box<ASTNode>,
        values_list: Box<ASTNode>
    },
    SQLUpdate {
        table: Box<ASTNode>,
        assignments: Box<ASTNode>,
        selection: Option<Box<ASTNode>>
    },
    SQLUnion{left: Box<ASTNode>, union_type: Box<ASTNode>, right: Box<ASTNode>},
    SQLJoin{left: Box<ASTNode>, join_type: Box<ASTNode>, right: Box<ASTNode>, on_expr: Option<Box<ASTNode>>},
	SQLJoinType(JoinType),
	SQLUnionType(UnionType),

	// MySQL
    MySQLCreateTable{
        table: Box<ASTNode>,
        column_list: Vec<ASTNode>,
        keys: Vec<ASTNode>,
        table_options: Vec<ASTNode>
    },
    MySQLColumnDef{column: Box<ASTNode>, data_type: Box<ASTNode>, qualifiers: Option<Vec<ASTNode>>},
    MySQLKeyDef(MySQLKeyDef),
    MySQLColumnQualifier(MySQLColumnQualifier),
    MySQLDataType(MySQLDataType),
    MySQLTableOption(MySQLTableOption)
}

#[derive(Debug, PartialEq)]
pub enum LiteralExpr {
	LiteralLong(u32, u64),
	LiteralBool(u32, bool),
	LiteralDouble(u32, f64),
	LiteralString(u32, String)
}

#[derive(Debug, PartialEq)]
pub enum Operator {
	ADD,
	SUB,
	MULT,
	DIV,
	MOD,
	GT,
	LT,
	// GTEQ,
	// LTEQ,
	EQ,
	// NEQ,
	OR,
	AND
}

#[derive(Debug, PartialEq)]
pub enum UnionType {
	UNION,
	ALL,
	DISTINCT
}

#[derive(Debug, PartialEq)]
pub enum JoinType {
	INNER,
	LEFT,
	RIGHT,
	FULL,
	CROSS
}

#[derive(Debug, PartialEq)]
pub enum MySQLKeyDef {
	Primary{symbol: Option<Box<ASTNode>>, name: Option<Box<ASTNode>>, columns: Vec<ASTNode>},
	Unique{symbol: Option<Box<ASTNode>>, name: Option<Box<ASTNode>>, columns: Vec<ASTNode>},
	Foreign{symbol: Option<Box<ASTNode>>, name: Option<Box<ASTNode>>, columns: Vec<ASTNode>, reference_table: Box<ASTNode>, reference_columns: Vec<ASTNode>},
	FullText{name: Option<Box<ASTNode>>, columns: Vec<ASTNode>},
	Index{name: Option<Box<ASTNode>>, columns: Vec<ASTNode>}
}

#[derive(Debug, PartialEq)]
pub enum MySQLDataType {
	Bit{display: Option<u32>},
	TinyInt{display: Option<u32>},
	SmallInt{display: Option<u32>},
	MediumInt{display: Option<u32>},
	Int{display: Option<u32>},
	BigInt{display: Option<u32>},
	Decimal{precision: Option<u32>, scale: Option<u32>},
	Float{precision: Option<u32>, scale: Option<u32>},
	Double{precision: Option<u32>, scale: Option<u32>},
	Bool,
	Date,
	DateTime{fsp: Option<u32>},
	Timestamp{fsp: Option<u32>},
	Time{fsp: Option<u32>},
	Year{display: Option<u32>},
	Char{length: Option<u32>},
	NChar{length: Option<u32>},
	CharByte{length: Option<u32>},
	Varchar{length: Option<u32>},
	NVarchar{length: Option<u32>},
	Binary{length: Option<u32>},
	VarBinary{length: Option<u32>},
	TinyBlob,
	TinyText,
	Blob{length: Option<u32>},
	Text{length: Option<u32>},
	MediumBlob,
	MediumText,
	LongBlob,
	LongText,
	Enum{values: Box<ASTNode>},
	Set{values: Box<ASTNode>}
}

#[derive(Debug, PartialEq)]
pub enum MySQLColumnQualifier {
	CharacterSet(Box<ASTNode>),
	Collate(Box<ASTNode>),
	Default(Box<ASTNode>),
	Signed,
	Unsigned,
	Null,
	NotNull,
	AutoIncrement,
	PrimaryKey,
	UniqueKey,
	OnUpdate(Box<ASTNode>),
	Comment(Box<ASTNode>)
}

#[derive(Debug, PartialEq)]
pub enum MySQLTableOption {
	Engine(Box<ASTNode>),
	Charset(Box<ASTNode>),
	Comment(Box<ASTNode>),
	AutoIncrement(Box<ASTNode>)
}


// Planner APIs
pub trait Planner<D: Dialect> {
	fn plan(&self, dialects: D, ast: ASTNode) -> Result<Option<RelNode>, String>;
}

pub enum RelNode {
	Rel
}
