use std::iter::Peekable;
use std::str::Chars;
use std::sync::atomic::{AtomicU32, Ordering};
use std::ascii::AsciiExt;
use error::ZeroError;
pub mod dialects;
pub mod planner;

#[cfg(test)]
mod tests;

// Dialect api
pub trait Dialect {

	fn get_keywords(&self) -> Vec<&'static str>;

	fn get_token(&self, chars: &mut Peekable<Chars>, keywords: &Vec<&'static str>, literals: &mut Vec<LiteralToken>) -> Result<Option<Token>, Box<ZeroError>>;

	fn parse_prefix<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<Option<ASTNode>, Box<ZeroError>>;

	fn get_precedence<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<u8, Box<ZeroError>>;

	fn parse_infix<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>, left: ASTNode, precedence: u8) -> Result<Option<ASTNode>, Box<ZeroError>>;

	// fn plan();
}

// Tokenizer apis
pub trait Tokenizer<D: Dialect> {
	fn tokenize<'a>(&self, dialect: &'a D) -> Result<Tokens<'a, D>, Box<ZeroError>>;
}

impl<D: Dialect> Tokenizer<D> for String {
	fn tokenize<'a>(&self, dialect: &'a D) -> Result<Tokens<'a, D>, Box<ZeroError>> {

		let keywords = dialect.get_keywords();

		let mut chars = self.chars().peekable();
		let mut tokens: Vec<Token> = Vec::new();
        let mut literals: Vec<LiteralToken> = Vec::new();
		while let Some(&ch) = chars.peek() {
			match dialect.get_token(&mut chars, &keywords, &mut literals)? {
				Some(t) => tokens.push(t),
				None => return Err(ZeroError::DecryptionError{
                    message: format!("No token dialect support for character {:?}", ch).into(),
                    code: "1064".into()
                }.into())
			}
		}

		let stream = tokens
			.into_iter()
			.filter(|t| match t { &Token::Whitespace => false, _ => true })
			.filter(|t| match t { &Token::Comment(_) => false, _ => true })
			.collect::<Vec<_>>();

		Ok(Tokens::new(dialect, stream, literals))
	}
}

#[derive(Debug)]
pub struct Tokens<'a, D: 'a + Dialect> {
	pub dialect: &'a D,
	pub tokens: Vec<Token>,
	pub index: AtomicU32,
    pub literals: Vec<LiteralToken>,
}


impl<'a, D: 'a + Dialect> Tokens<'a, D> {
	pub fn new(dialect: &'a D, tokens: Vec<Token>, literals: Vec<LiteralToken>) -> Self {
		Tokens {
			dialect: dialect,
			tokens: tokens,
			index: AtomicU32::new(0),
            literals: literals
		}
	}

    pub fn peek(&self) -> Option<&Token> {
		let i = self.index.load(Ordering::SeqCst) as usize;
		if i < self.tokens.len() {
			Some(&self.tokens[i as usize])
		} else {
			None
		}
	}

	pub fn next(&self) -> Option<&Token> {
		let i = self.index.load(Ordering::SeqCst) as usize;
		if i < self.tokens.len() {
			self.index.fetch_add(1, Ordering::SeqCst);
			Some(&self.tokens[i as usize])
		} else {
			panic!("Index out of bounds")
		}
	}

	fn consume_keyword(&self, text: &str) -> bool
		 {

		match self.peek() {
			Some(&Token::Keyword(ref v)) | Some(&Token::Identifier(ref v)) => {
				if text.eq_ignore_ascii_case(&v) {
					self.next();
					true
				} else {
					false
				}
			},
			_ => false
		}
	}

	fn consume_punctuator(&self, text: &str) -> bool {

		match self.peek() {
			Some(&Token::Punctuator(ref v)) => {
				if text.eq_ignore_ascii_case(&v) {
					self.next();
					true
				} else {
					false
				}
			},
			_ => false
		}
	}

	fn consume_operator(&self, text: &str) -> bool {

		match self.peek() {
			Some(&Token::Operator(ref v)) => {
				if text.eq_ignore_ascii_case(&v) {
					self.next();
					true
				} else {
					false
				}
			},
			_ => false
		}
	}

    fn consume_literal_null(&self) -> bool {
        let ret = match self.peek() {
            Some(&Token::Literal(i)) => {
                match self.literals.get(i) {
                    Some(t) => match t {
                        &LiteralToken::LiteralNull(_) => true,
                        _ => false
                    },
                    _ => false
                }
            },
            _ => false
        };

        if ret {
            self.next();
        }

        ret
    }

    fn get_literal(&self, i: usize) -> Option<&LiteralToken> {
        self.literals.get(i)
    }
}

#[derive(Debug,PartialEq, Eq, Clone, Hash)]
pub enum Token  {
	Whitespace,
	Keyword(String),
	Identifier(String),
	Literal(usize),
	BoundParam(u32),
	Operator(String),
	Punctuator(String),
	Comment(String)
}

#[derive(Debug,PartialEq,Clone)]
pub enum LiteralToken {
    LiteralString(usize, String),
    LiteralLong(usize, String),
    LiteralDouble(usize, String),
    LiteralBool(usize, String),
    LiteralNull(usize)
}

impl LiteralToken {
    fn to_readable(&self) -> String {
        match *self {
            LiteralToken::LiteralString(_, ref value) => format!("'{}'", value),
            LiteralToken::LiteralLong(_, ref value) => value.clone(),
            LiteralToken::LiteralDouble(_, ref value) => value.clone(),
            LiteralToken::LiteralBool(_, ref value) => value.clone(),
            LiteralToken::LiteralNull(_) => "NULL".into(),
        }
    }
}
// Parser APIs
pub trait Parser<D: Dialect> {
	fn parse(&self) -> Result<ASTNode, Box<ZeroError>>;
	fn parse_expr(&self, precedence: u8) -> Result<ASTNode, Box<ZeroError>>;
}


impl<'a, D: Dialect> Parser<D> for Tokens<'a, D> {
	fn parse(&self) -> Result<ASTNode, Box<ZeroError>> { self.parse_expr(0) }

	fn parse_expr(&self, precedence: u8) -> Result<ASTNode, Box<ZeroError>> {
		let mut expr = self.dialect.parse_prefix(self)?;
		while let Some(_) = self.peek() {
			let next_precedence = self.dialect.get_precedence(self)?;

			if precedence >= next_precedence {
				break;
			}

			expr = self.dialect.parse_infix(self, expr.unwrap(), next_precedence)?;
		}

		Ok(expr.unwrap())
	}

}

#[derive(Debug, PartialEq)]
pub enum InsertMode {
	INSERT,
	IGNORE,
//	REPLACE,
}

#[derive(Debug, PartialEq)]
pub enum ASTNode {
	// ANSISQL nodes
	SQLIdentifier{id: String, parts: Vec<String>},
	SQLBinary{left: Box<ASTNode>, op: Operator, right: Box<ASTNode>},
	SQLNested(Box<ASTNode>),
	SQLUnary{operator: Operator, expr: Box<ASTNode>},
	SQLLiteral(usize),
	SQLBoundParam(u32),
	SQLAlias{expr: Box<ASTNode>, alias: Box<ASTNode>},
	SQLExprList(Vec<ASTNode>),
    SQLOrderBy{expr: Box<ASTNode>, is_asc: bool},
    SQLSelect{
        expr_list: Box<ASTNode>,
        relation: Option<Box<ASTNode>>,
        selection: Option<Box<ASTNode>>,
        order: Option<Box<ASTNode>>,
        for_update: bool
    },
    SQLInsert {
        table: Box<ASTNode>,
		insert_mode: InsertMode,
        column_list: Box<ASTNode>,
        values_list: Vec<ASTNode>
    },
    SQLUpdate {
        table: Box<ASTNode>,
        assignments: Box<ASTNode>,
        selection: Option<Box<ASTNode>>
    },
	SQLDelete {
		table: Box<ASTNode>,
		selection: Option<Box<ASTNode>>
	},
    SQLUnion{left: Box<ASTNode>, union_type: UnionType, right: Box<ASTNode>},
    SQLJoin{left: Box<ASTNode>, join_type: JoinType, right: Box<ASTNode>, on_expr: Option<Box<ASTNode>>},
	SQLFunctionCall{identifier: Box<ASTNode>, args: Vec<ASTNode>},

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
    MySQLTableOption(MySQLTableOption),
	MySQLUse(Box<ASTNode>)
}

#[derive(Debug, PartialEq, Clone)]
pub enum Operator {
	ADD,
	SUB,
	MULT,
	DIV,
	MOD,
	GT,
	LT,
	GTEQ,
	LTEQ,
	EQ,
    NEQ,
	OR,
	AND
}

impl Operator {
    fn to_readable(&self) -> String {
        match *self {
            Operator::ADD => "+".into(),
            Operator::SUB => "-".into(),
            Operator::MULT => "*".into(),
            Operator::DIV => "/".into(),
            Operator::MOD => "%".into(),
            Operator::GT => ">".into(),
            Operator::LT => "<".into(),
            Operator::GTEQ => ">=".into(),
            Operator::LTEQ => "<=".into(),
            Operator::EQ => "=".into(),
            Operator::NEQ => "!=".into(),
            Operator::OR => "OR".into(),
            Operator::AND => "AND".into(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum UnionType {
	UNION,
	ALL,
	DISTINCT
}

#[derive(Debug, PartialEq, Clone)]
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
	fn plan(&self, dialects: D, ast: ASTNode) -> Result<Option<RelNode>, Box<ZeroError>>;
}

#[allow(dead_code)]
pub enum RelNode {
	Rel
}


// Writer apis
pub trait Writer {
    fn write(&self, node: &ASTNode) -> Result<String, Box<ZeroError>>;
	fn _write(&self, builder: &mut String, node: &ASTNode) -> Result<(), Box<ZeroError>>;
}

// returning true/false denotes whether this variant wrote the expression
pub trait ExprWriter {
    fn write(&self, writer: &Writer, builder: &mut String, node: &ASTNode) -> Result<bool, Box<ZeroError>>;
}

pub struct SQLWriter<'a> {
    variants: Vec<&'a ExprWriter>
}

impl<'a> Default for SQLWriter<'a> {
	fn default() -> Self {SQLWriter::new(vec![])}
}

impl<'a> SQLWriter<'a> {
	pub fn new(variants: Vec<&'a ExprWriter>) -> Self {
		SQLWriter{variants: variants}
	}
}

impl<'a> Writer for SQLWriter<'a> {
    fn write(&self, node: &ASTNode) -> Result<String, Box<ZeroError>> {
        let mut builder = String::new();
        self._write(&mut builder, node)?;
        Ok(builder)
    }

    fn _write(&self, builder: &mut String, node: &ASTNode) -> Result<(), Box<ZeroError>> {
		for v in self.variants.iter() {
			if v.write(self, builder, node)? {
				return Ok(())
			}
		}
        Err(ZeroError::DecryptionError{
            message: format!("No provided ExprWriter writes expr {:?}", node).into(),
            code: "1064".into()
        }.into())
    }
}
