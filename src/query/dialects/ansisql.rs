use super::super::*;


use std::iter::Peekable;
use std::str::Chars;
use std::sync::atomic::{AtomicU32, Ordering};
use std::ascii::AsciiExt;
use std::str::FromStr;

// TODO need some way of unifying keywords between dialects
static KEYWORDS: &'static [&'static str] = &["SELECT", "FROM", "WHERE", "AND", "OR", "UNION", "FROM", "AS",
    "WHERE", "ORDER", "BY", "HAVING", "GROUP", "ASC", "DESC", "JOIN", "INNER", "LEFT", "RIGHT", "CROSS",
    "FULL", "ON", "INSERT", "UPDATE", "SET", "VALUES", "INTO"];

pub struct AnsiSQLDialect {
	lit_index: AtomicU32
}

impl AnsiSQLDialect {
	pub fn new() -> Self {AnsiSQLDialect{lit_index: AtomicU32::new(0)}}
}

impl Dialect for AnsiSQLDialect {

    fn get_keywords(&self) -> &'static [&'static str] {
        KEYWORDS
    }

	fn get_token(&self, chars: &mut Peekable<Chars>, keywords: &Vec<&'static str>) -> Result<Option<Token>, String> {
		match chars.peek() {
	        Some(&ch) => match ch {
	            ' ' | '\t' | '\n' => {
	                chars.next(); // consumer the char
	                Ok(Some(Token::Whitespace))
	            },
	            '+' | '-' | '/' | '*' | '%' | '=' => {
	                chars.next(); // consume one
	                Ok(Some(Token::Operator(ch.to_string()))) // after consume because return val
	            },
	            '>' | '<' | '!' => {

	                let mut op = chars.next().unwrap().to_string();

	                match chars.peek() {
	                    Some(&c) => match c {
	                        '=' => {
	                            op.push(c);
	                            chars.next(); // consume one
	                        }
	                        _ => {}
	                    },
	                    None => return Err(String::from("Expected token received None"))
	                }
	                Ok(Some(Token::Operator(op)))
	            },
	            '0'...'9' | '.' => {
	                let mut text = String::new();
	                while let Some(&c) = chars.peek() { // will break when it.peek() => None

	                    if c.is_numeric() || '.' == c  {
	                        text.push(c);
	                    } else {
	                        break; // leave the loop early
	                    }

	                    chars.next(); // consume one
	                }

	                if text.as_str().contains('.') {
						Ok(Some(Token::Literal(LiteralToken::LiteralDouble(self.lit_index.fetch_add(1, Ordering::SeqCst), text))))
	                } else {
						Ok(Some(Token::Literal(LiteralToken::LiteralLong(self.lit_index.fetch_add(1, Ordering::SeqCst), text))))
	                }
	            },
	            'a'...'z' | 'A'...'Z' => { // TODO this should really be any valid char for an identifier..
	                let mut text = String::new();
	                while let Some(&c) = chars.peek() { // will break when it.peek() => None

	                    if c.is_alphabetic() || c.is_numeric() || c == '.' || c == '_' {
	                        text.push(c);
	                    } else {
	                        break; // leave the loop early
	                    }

	                    chars.next(); // consume one
	                }

	                if "true".eq_ignore_ascii_case(&text) || "false".eq_ignore_ascii_case(&text) {
	                    Ok(Some(Token::Literal(LiteralToken::LiteralBool(self.lit_index.fetch_add(1, Ordering::SeqCst), text))))
	                } else if keywords.iter().position(|&r| r.eq_ignore_ascii_case(&text)).is_none() {
	                    Ok(Some(Token::Identifier(text)))
	                } else if "AND".eq_ignore_ascii_case(&text) || "OR".eq_ignore_ascii_case(&text) {
	                    Ok(Some(Token::Operator(text)))
	                } else {
	                    Ok(Some(Token::Keyword(text.to_uppercase())))
	                }
	            },
	            '\'' => {
	                chars.next();
	                let mut s = String::new();
	                loop {
	                    match chars.peek() {
	                        Some(&c) => match c {
	                            '\\' => {
	                                s.push(c);
	                                chars.next();
	                                match chars.peek() {
	                                    Some(&n) => match n {
	                                        '\'' => {
	                                            s.push(n);
	                                            chars.next();
	                                        },
	                                        _ => continue,
	                                    },
	                                    None => return Err(String::from("Unexpected end of string"))
	                                }
	                            },
	                            '\'' => {
	                                chars.next();
	                                break;
	                            },
	                            _ => {
	                                s.push(c);
	                                chars.next();
	                            }
	                        },
	                        None => return Err(String::from("Unexpected end of string"))
	                    }
	                }

					Ok(Some(Token::Literal(LiteralToken::LiteralString(self.lit_index.fetch_add(1, Ordering::SeqCst), s))))
	            },
	            ',' | '(' | ')' => {
	                chars.next();
	                Ok(Some(Token::Punctuator(ch.to_string())))
	            },
	            _ => {
	                Err(format!("Unsupported char {:?}", ch))
	            }
	        },
	        None => Ok(None),
	    }
	}

    fn parse_prefix<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<Option<ASTNode>, String> {
        match tokens.peek() {
			Some(t) => match t {
				&Token::Keyword(ref v) => match &v as &str {
					"SELECT" => Ok(Some(try!(self.parse_select(tokens)))),
					"INSERT" => Ok(Some(try!(self.parse_insert(tokens)))),
					"UPDATE" => Ok(Some(try!(self.parse_update(tokens)))),
					// "CREATE" => Ok(Some(try!(self.parse_create(tokens)))),
					_ => Err(format!("Unsupported prefix {:?}", v))
				},
				&Token::Literal(ref v) => match v {
					&LiteralToken::LiteralLong(i, ref value) => {
						tokens.next();
						Ok(Some(ASTNode::SQLLiteral(LiteralExpr::LiteralLong(i, u64::from_str(&value).unwrap()))))
					},
					&LiteralToken::LiteralBool(i, ref value) => {
						tokens.next();
						Ok(Some(ASTNode::SQLLiteral(LiteralExpr::LiteralBool(i, bool::from_str(&value).unwrap()))))
					},
					&LiteralToken::LiteralDouble(i, ref value) => {
						tokens.next();
						Ok(Some(ASTNode::SQLLiteral(LiteralExpr::LiteralDouble(i, f64::from_str(&value).unwrap()))))
					},
					&LiteralToken::LiteralString(i, ref value) => {
						tokens.next();
						Ok(Some(ASTNode::SQLLiteral(LiteralExpr::LiteralString(i, value.clone()))))
					}
					//_ => panic!("Unsupported literal {:?}", v)
				},
				&Token::Identifier(_) => Ok(Some(try!(self.parse_identifier(tokens)))),//Some(self.parse_identifier(tokens)),
				&Token::Punctuator(ref v) => match &v as &str {
					"(" => {
						Ok(Some(try!(self.parse_nested(tokens))))
					},
					_ => Err(format!("Unsupported prefix for punctuator {:?}", &v))
				},
				&Token::Operator(ref v) => match &v as &str {
					"+" | "-" => Ok(Some(try!(self.parse_unary(tokens)))),
					"*" => Ok(Some(try!(self.parse_identifier(tokens)))),
					_ => Err(format!("Unsupported operator as prefix {:?}", &v))
				},
				_ => Err(format!("parse_prefix() {:?}", &t))
			},
			None => Ok(None)
		}
    }

    fn get_precedence<'a, D:  Dialect>(&self, tokens: &Tokens<'a, D>)-> Result<u8, String> {
        println!("get_precedence() token={:?}", tokens.peek());
        let prec = match tokens.peek() {
            Some(token) => match token {
                &Token::Operator(ref t) => match &t as &str {
                    "<" | "<=" | ">" | ">=" | "<>" | "!=" => 20,
                    "-" | "+" => 33,
                    "*" | "/" => 40,
                    "=" => 11,
                    "AND" => 9,
                    "OR" => 7,

                    _ => return Err(String::from(format!("Unsupported operator {}", t)))
                },
                &Token::Keyword(ref t) => match &t as &str {
                    "UNION" => 3,
                    "JOIN" | "INNER" | "RIGHT" | "LEFT" | "CROSS" | "FULL" => 5,
                    "AS" => 6,
                    _ => 0
                },
                _ => 0
            },
            None => 0
        };

        Ok(prec)
    }

    fn parse_infix<'a, D: Dialect>
        (&self, tokens: &Tokens<'a, D>, left: ASTNode, precedence: u8)
            -> Result<Option<ASTNode>, String> {
        Err(String::from("parse_infix() not implemented"))
    }

}

impl AnsiSQLDialect {
    fn parse_insert<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<ASTNode,  String>
		 {

		println!("parse_insert()");

		// TODO validation
		self.consume_keyword("INSERT", tokens);
		self.consume_keyword("INTO", tokens);

		let table = try!(self.parse_identifier(tokens));

		let columns = if self.consume_punctuator("(", tokens) {
			let ret = try!(self.parse_expr_list(tokens));
			self.consume_punctuator(")", tokens);
			ret
		} else {
			return Err(format!("Expected column list paren, received {:?}", &tokens.peek()));
		};

		self.consume_keyword("VALUES", tokens);
		self.consume_punctuator("(", tokens);
		let values = try!(self.parse_expr_list(tokens));
		self.consume_keyword(")", tokens);

		Ok(ASTNode::SQLInsert {
			table: Box::new(table),
			column_list: Box::new(columns),
			values_list: Box::new(values)
		})

	}

	fn parse_select<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<ASTNode,  String> {

		println!("parse_select()");
		// consume the SELECT
		tokens.next();
		let proj = Box::new(try!(self.parse_expr_list(tokens)));

        println!("HERE {:?}", tokens.peek());
		let from = match tokens.peek() {
			Some(&Token::Keyword(ref t)) => match &t as &str {
				"FROM" => {
                    println!("THERE");
					println!("HITHER {:?}",tokens.next());
                    println!("THITHER {:?}", tokens.peek());
					Some(Box::new(try!(self.parse_relation(tokens))))
				},
				_ => None
			},
			_ => return Err(format!("unexpected token {:?}", tokens.peek()))
		};

		let whr = match tokens.peek() {
			Some(&Token::Keyword(ref t)) => match &t as &str {
				"WHERE" => {
					tokens.next();
					Some(Box::new(tokens.parse_expr(0)?))
				},
				_ => None
			},
			_ => None
		};

		let ob = {
			if self.consume_keyword(&"ORDER", tokens) {
				if self.consume_keyword(&"BY", tokens) {
					Some(Box::new(try!(self.parse_order_by_list(tokens))))
				} else {
					return Err(format!("Expected ORDER BY, found ORDER {:?}", tokens.peek()));
				}
			} else {
				None
			}
		};

		Ok(ASTNode::SQLSelect{expr_list: proj, relation: from, selection: whr, order: ob})
	}

	fn parse_update<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<ASTNode, String>
		 {

		self.consume_keyword("UPDATE", tokens);

		let table = try!(self.parse_identifier(tokens));

		self.consume_keyword("SET", tokens);

		let assignments = try!(self.parse_expr_list(tokens));

		let selection = if self.consume_keyword("WHERE", tokens) {
			Some(Box::new(tokens.parse_expr(0)?))
		} else {
			None
		};

		Ok(ASTNode::SQLUpdate {
			table: Box::new(table),
			assignments: Box::new(assignments),
			selection: selection
		})
	}

	// TODO real parse_relation
	fn parse_relation<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<ASTNode,  String>{
        tokens.parse_expr(4)
	}

	fn parse_expr_list<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<ASTNode,  String>
		 {

		println!("parse_expr_list()");
		let first = tokens.parse_expr(0)?;
		let mut v: Vec<ASTNode> = Vec::new();
		v.push(first);
		while let Some(&Token::Punctuator(ref p)) = tokens.peek() {
			if p == "," {
				tokens.next();
				v.push(tokens.parse_expr(0)?);
			} else {
				break;
			}
		}
		Ok(ASTNode::SQLExprList(v))
	}

	fn parse_order_by_list<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<ASTNode,  String>
		 {

		println!("parse_order_by_list()");
		let mut v: Vec<ASTNode> = Vec::new();
		v.push(try!(self.parse_order_by_expr(tokens)));
		while let Some(&Token::Punctuator(ref p)) = tokens.peek() {
			if p == "," {
				tokens.next();
				v.push(try!(self.parse_order_by_expr(tokens)));
			} else {
				break;
			}
		}
		Ok(ASTNode::SQLExprList(v))
	}

	fn parse_order_by_expr<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<ASTNode,  String>
		 {

		let e = tokens.parse_expr(0)?;
		Ok(ASTNode::SQLOrderBy {expr: Box::new(e), is_asc: self.is_asc(tokens)})
	}

	fn is_asc<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) -> bool
		 {

		if self.consume_keyword(&"DESC", tokens) {
			false
		} else {
			self.consume_keyword(&"ASC", tokens);
			true
		}
	}

	fn parse_binary<'a, D: Dialect>(&self, left: ASTNode, tokens: &Tokens<'a, D>) -> Result<ASTNode,  String>
		 {

		println!("parse_binary()");
		let precedence = self.get_precedence(tokens)?;
		// determine operator
		let operator = match tokens.next().unwrap() {
			&Token::Operator(ref t) => match &t as &str {
				"+" => ASTNode::SQLOperator(Operator::ADD),
				"-" => ASTNode::SQLOperator(Operator::SUB),
				"*" => ASTNode::SQLOperator(Operator::MULT),
				"/" => ASTNode::SQLOperator(Operator::DIV),
				"%" => ASTNode::SQLOperator(Operator::MOD),
				">" => ASTNode::SQLOperator(Operator::GT),
				"<" => ASTNode::SQLOperator(Operator::LT),
				"=" => ASTNode::SQLOperator(Operator::EQ),
				"AND" => ASTNode::SQLOperator(Operator::AND),
				"OR" => ASTNode::SQLOperator(Operator::OR),
				_ => return Err(format!("Unsupported operator {}", t))
			},
			_ => return Err(format!("Expected operator, received something else"))
		};

		Ok(ASTNode::SQLBinary {left: Box::new(left), op: Box::new(operator), right: Box::new(tokens.parse_expr(precedence)?)})
	}

	fn parse_identifier<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<ASTNode,  String>
		 {

		println!("parse_identifier()");
		match tokens.next().unwrap() {
			&Token::Identifier(ref v) => Ok(ASTNode::SQLIdentifier{id: v.clone(), parts: self.get_identifier_parts(v)?}),
			&Token::Operator(ref o) => match &o as &str {
				"*" => Ok(ASTNode::SQLIdentifier{id: o.clone(), parts: vec![o.clone()]}),
				_ => Err(format!("Unsupported operator as identifier {}", o))
			},
			_ => Err(format!("Illegal state"))
		}
	}

	fn get_identifier_parts(&self, id: &String) -> Result<Vec<String>, String> {
		Ok(id.split(".").map(|s| s.to_string()).collect())
	}

	fn parse_nested<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<ASTNode,  String>
		 {

		//consume (
		tokens.next();
		let nested = tokens.parse_expr(0)?;
		// consume )
		match tokens.peek() {
			Some(&Token::Punctuator(ref v)) => match &v as &str {
				")" => {tokens.next();},
				_ => return Err(format!("Expected , punctuator, received {}", v))
			},
			_ => return Err(format!("Illegal state, expected , received {:?}", tokens.peek()))
		}

		Ok(ASTNode::SQLNested(Box::new(nested)))
	}

	fn parse_unary<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<ASTNode,  String>
		 {

		let precedence = self.get_precedence(tokens)?;
		let op = match tokens.next() {
			Some(&Token::Operator(ref o)) => match &o as &str {
				"+" => ASTNode::SQLOperator(Operator::ADD),
				"-" => ASTNode::SQLOperator(Operator::SUB),
				_ => return Err(format!("Illegal operator for unary {}", o))
			},
			_ => return Err(format!("Illegal state"))
		};
		Ok(ASTNode::SQLUnary{operator: Box::new(op), expr: Box::new(tokens.parse_expr(precedence)?)})

	}

	fn parse_union<'a, D: Dialect>(&self, left: ASTNode, tokens: &Tokens<'a, D>) -> Result<ASTNode,  String>
		 {

		// consume the UNION
		tokens.next();

		let union_type = match tokens.peek() {
			Some(&Token::Keyword(ref t)) => match &t as &str {
				"ALL" => ASTNode::SQLUnionType(UnionType::ALL),
				"DISTINCT" => ASTNode::SQLUnionType(UnionType::DISTINCT),
				_ => ASTNode::SQLUnionType(UnionType::UNION)
			},
			_ => ASTNode::SQLUnionType(UnionType::UNION)
		};

		let right = Box::new(tokens.parse_expr(0)?);

		Ok(ASTNode::SQLUnion{left: Box::new(left), union_type: Box::new(union_type), right: right})

	}

	fn parse_join<'a, D: Dialect>(&self, left: ASTNode, tokens: &Tokens<'a, D>) -> Result<ASTNode,  String>
		 {

		// TODO better protection on expected keyword sequences
		let join_type = {
			if self.consume_keyword("JOIN", tokens) || self.consume_keyword("INNER", tokens) {
				self.consume_keyword("JOIN", tokens);
				ASTNode::SQLJoinType(JoinType::INNER)
			} else if self.consume_keyword("LEFT", tokens) {
				self.consume_keyword("OUTER", tokens);
				self.consume_keyword("JOIN", tokens);
				ASTNode::SQLJoinType(JoinType::LEFT)
			} else if self.consume_keyword("RIGHT", tokens) {
				self.consume_keyword("OUTER", tokens);
				self.consume_keyword("JOIN", tokens);
				ASTNode::SQLJoinType(JoinType::RIGHT)
			} else if self.consume_keyword("FULL", tokens) {
				self.consume_keyword("OUTER", tokens);
				self.consume_keyword("JOIN", tokens);
				ASTNode::SQLJoinType(JoinType::FULL)
			} else if self.consume_keyword("CROSS", tokens) {
				self.consume_keyword("JOIN", tokens);
				ASTNode::SQLJoinType(JoinType::LEFT)
			} else {
				return Err(format!("Unsupported join keyword {:?}", tokens.peek()))
			}
		};

		let right = Box::new(tokens.parse_expr(0)?);

		let on = {
			if self.consume_keyword("ON", tokens) {
				Some(Box::new(tokens.parse_expr(0)?))
			} else if join_type != ASTNode::SQLJoinType(JoinType::CROSS) {
				return Err(format!("Expected ON, received token {:?}", tokens.peek()))
			} else {
				None
			}
		};

		Ok(ASTNode::SQLJoin {left: Box::new(left), join_type: Box::new(join_type), right: right, on_expr: on})
	}

	fn parse_alias<'a, D: Dialect>(&self, left: ASTNode, tokens: &Tokens<'a, D>) -> Result<ASTNode,  String>
		 {

		if self.consume_keyword(&"AS", tokens) {
			Ok(ASTNode::SQLAlias{expr: Box::new(left), alias: Box::new(try!(self.parse_identifier(tokens)))})
		} else {
			Err(format!("Illegal state, expected AS, received token {:?}", tokens.peek()))
		}
	}

	// TODO more helper methods like consume_keyword_sequence, required_keyword_sequence, etc
	fn consume_keyword<'a, D: Dialect>(&self, text: &str, tokens: &Tokens<'a, D>) -> bool
		 {

		match tokens.peek() {
			Some(&Token::Keyword(ref v)) | Some(&Token::Identifier(ref v)) => {
				if text.eq_ignore_ascii_case(&v) {
					tokens.next();
					true
				} else {
					false
				}
			},
			_ => false
		}
	}

	fn consume_punctuator<'a, D: Dialect>(&self, text: &str, tokens: &Tokens<'a, D>) -> bool
		 {

		match tokens.peek() {
			Some(&Token::Punctuator(ref v)) => {
				if text.eq_ignore_ascii_case(&v) {
					tokens.next();
					true
				} else {
					false
				}
			},
			_ => false
		}
	}

	fn consume_operator<'a, D: Dialect>(&self, text: &str, tokens: &Tokens<'a, D>) -> bool
		 {

		match tokens.peek() {
			Some(&Token::Operator(ref v)) => {
				if text.eq_ignore_ascii_case(&v) {
					tokens.next();
					true
				} else {
					false
				}
			},
			_ => false
		}
	}
}
