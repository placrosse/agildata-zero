use super::super::*;


use std::iter::Peekable;
use std::str::Chars;
use std::sync::atomic::{AtomicU32, Ordering};
use std::ascii::AsciiExt;
use std::str::FromStr;
use std::fmt::Write;

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

    fn get_keywords(&self) -> Vec<&'static str> {
        let mut k = Vec::new();
        k.extend_from_slice(KEYWORDS);
        k
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

    fn parse_infix<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>, left: ASTNode, precedence: u8)-> Result<Option<ASTNode>, String> {
        println!("parse_infix() {}", precedence);
		match tokens.peek() {
			Some(token) => match token {
				&Token::Operator(_) => Ok(Some(try!(self.parse_binary(left, tokens)))),
				&Token::Keyword(ref t) => match &t as &str {
					"UNION" => Ok(Some(try!(self.parse_union(left, tokens)))),
					"JOIN" | "INNER" | "RIGHT" | "LEFT" | "CROSS" | "FULL" => Ok(Some(try!(self.parse_join(left, tokens)))),
					"AS" => Ok(Some(try!(self.parse_alias(left, tokens)))),
					_ => {
						println!("Returning no infix for keyword {:?}", t);
						Ok(None)
					}
				},
				_ => {
					println!("Returning no infix for token {:?}", token);
					Ok(None)
				}

			},
			None => Ok(None)
		}
    }

}

impl AnsiSQLDialect {
    fn parse_insert<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<ASTNode,  String>
		 {

		println!("parse_insert()");

		// TODO validation
		tokens.consume_keyword("INSERT");
		tokens.consume_keyword("INTO");

		let table = try!(self.parse_identifier(tokens));

		let columns = if tokens.consume_punctuator("(") {
			let ret = try!(self.parse_expr_list(tokens));
			tokens.consume_punctuator(")");
			ret
		} else {
			return Err(format!("Expected column list paren, received {:?}", &tokens.peek()));
		};

		tokens.consume_keyword("VALUES");
		tokens.consume_punctuator("(");
		let values = try!(self.parse_expr_list(tokens));
		tokens.consume_keyword(")");

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

		let from = match tokens.peek() {
			Some(&Token::Keyword(ref t)) => match &t as &str {
				"FROM" => {
                    tokens.next();
					Some(Box::new(self.parse_relation(tokens)?))
				},
				_ => None
			},
			_ => None
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
			if tokens.consume_keyword(&"ORDER") {
				if tokens.consume_keyword(&"BY") {
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

		tokens.consume_keyword("UPDATE");

		let table = try!(self.parse_identifier(tokens));

		tokens.consume_keyword("SET");

		let assignments = try!(self.parse_expr_list(tokens));

		let selection = if tokens.consume_keyword("WHERE") {
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

	pub fn parse_expr_list<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<ASTNode,  String>
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

		if tokens.consume_keyword(&"DESC") {
			false
		} else {
			tokens.consume_keyword(&"ASC");
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
				"+" => Operator::ADD,
				"-" => Operator::SUB,
				"*" => Operator::MULT,
				"/" => Operator::DIV,
				"%" => Operator::MOD,
				">" => Operator::GT,
				"<" => Operator::LT,
				"=" => Operator::EQ,
				"AND" => Operator::AND,
				"OR" => Operator::OR,
				_ => return Err(format!("Unsupported operator {}", t))
			},
			_ => return Err(format!("Expected operator, received something else"))
		};

		Ok(ASTNode::SQLBinary {left: Box::new(left), op: operator, right: Box::new(tokens.parse_expr(precedence)?)})
	}

	pub fn parse_identifier<'a, D: Dialect>(&self, tokens: &Tokens<'a, D>) -> Result<ASTNode,  String>
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
				"+" => Operator::ADD,
				"-" => Operator::SUB,
				_ => return Err(format!("Illegal operator for unary {}", o))
			},
			_ => return Err(format!("Illegal state"))
		};
		Ok(ASTNode::SQLUnary{operator: op, expr: Box::new(tokens.parse_expr(precedence)?)})

	}

	fn parse_union<'a, D: Dialect>(&self, left: ASTNode, tokens: &Tokens<'a, D>) -> Result<ASTNode,  String>
		 {

		// consume the UNION
		tokens.next();

		let union_type = match tokens.peek() {
			Some(&Token::Keyword(ref t)) => match &t as &str {
				"ALL" => UnionType::ALL,
				"DISTINCT" => UnionType::DISTINCT,
				_ => UnionType::UNION
			},
			_ => UnionType::UNION
		};

		let right = Box::new(tokens.parse_expr(0)?);

		Ok(ASTNode::SQLUnion{left: Box::new(left), union_type: union_type, right: right})

	}

	fn parse_join<'a, D: Dialect>(&self, left: ASTNode, tokens: &Tokens<'a, D>) -> Result<ASTNode,  String>
		 {

		// TODO better protection on expected keyword sequences
		let join_type = {
			if tokens.consume_keyword("JOIN") || tokens.consume_keyword("INNER") {
				tokens.consume_keyword("JOIN");
				JoinType::INNER
			} else if tokens.consume_keyword("LEFT") {
				tokens.consume_keyword("OUTER");
				tokens.consume_keyword("JOIN");
				JoinType::LEFT
			} else if tokens.consume_keyword("RIGHT") {
				tokens.consume_keyword("OUTER");
				tokens.consume_keyword("JOIN");
				JoinType::RIGHT
			} else if tokens.consume_keyword("FULL") {
				tokens.consume_keyword("OUTER");
				tokens.consume_keyword("JOIN");
				JoinType::FULL
			} else if tokens.consume_keyword("CROSS") {
				tokens.consume_keyword("JOIN");
				JoinType::LEFT
			} else {
				return Err(format!("Unsupported join keyword {:?}", tokens.peek()))
			}
		};

		let right = Box::new(tokens.parse_expr(0)?);

		let on = {
			if tokens.consume_keyword("ON") {
				Some(Box::new(tokens.parse_expr(0)?))
			} else if join_type != JoinType::CROSS {
				return Err(format!("Expected ON, received token {:?}", tokens.peek()))
			} else {
				None
			}
		};

		Ok(ASTNode::SQLJoin {left: Box::new(left), join_type: join_type, right: right, on_expr: on})
	}

	fn parse_alias<'a, D: Dialect>(&self, left: ASTNode, tokens: &Tokens<'a, D>) -> Result<ASTNode,  String>
		 {

		if tokens.consume_keyword(&"AS") {
			Ok(ASTNode::SQLAlias{expr: Box::new(left), alias: Box::new(try!(self.parse_identifier(tokens)))})
		} else {
			Err(format!("Illegal state, expected AS, received token {:?}", tokens.peek()))
		}
	}

	// TODO more helper methods like consume_keyword_sequence, required_keyword_sequence, etc

}

pub struct AnsiSQLWriter{}

impl ExprWriter for AnsiSQLWriter {
	fn write(&self, writer: &Writer, builder: &mut String, node: &ASTNode) -> Result<bool, String> {
		match node {
			&ASTNode::SQLSelect{box ref expr_list, ref relation, ref selection, ref order} => {
				builder.push_str("SELECT");
				writer._write(builder, expr_list)?;
				match relation {
					&Some(box ref e) => {
						builder.push_str(" FROM");
						writer._write(builder, e)?
					},
					&None => {}
				}
				match selection {
					&Some(box ref e) => {
						builder.push_str(" WHERE");
						writer._write(builder, e)?
					},
					&None => {}
				}
				match order {
					&Some(box ref e) => {
						builder.push_str(" ORDER BY");
						writer._write(builder, e)?
					},
					&None => {}
				}

			},
			&ASTNode::SQLInsert{box ref table, box ref column_list, box ref values_list} => {
				builder.push_str("INSERT INTO");
				writer._write(builder, table)?;
				builder.push_str(" (");
				writer._write(builder, column_list)?;
				builder.push_str(") VALUES(");
				writer._write(builder, values_list)?;
				builder.push_str(")");
			},
			&ASTNode::SQLUpdate{box ref table, box ref assignments, ref selection} => {
				builder.push_str("UPDATE");
				writer._write(builder, table)?;
				builder.push_str(" SET");
				writer._write(builder, assignments)?;
				match selection {
					&Some(box ref e) => {
						builder.push_str(" WHERE");
						writer._write(builder, e)?
					},
					&None => {}
				}
			},
			&ASTNode::SQLExprList(ref vector) => {
				let mut sep = "";
				for e in vector.iter() {
					builder.push_str(sep);
					writer._write(builder, e)?;
					sep = ",";
				}
			},
			&ASTNode::SQLBinary{box ref left, ref op, box ref right} => {
				writer._write(builder, left)?;
				self._write_operator(builder, op);
				writer._write(builder, right)?;

			},
			&ASTNode::SQLLiteral(ref lit) => match lit {
				&LiteralExpr::LiteralLong(_, ref l) => {
					write!(builder, " {}", l).unwrap()
				},
				&LiteralExpr::LiteralBool(_, ref b) => {
					write!(builder, "{}", b).unwrap();
				},
				&LiteralExpr::LiteralDouble(_, ref d) => {
					write!(builder, "{}", d).unwrap();
				},
				&LiteralExpr::LiteralString(_, ref s) => {
					write!(builder, " '{}'", s).unwrap()
				}
				//_ => panic!("Unsupported literal for writing {:?}", lit)
			},
			&ASTNode::SQLAlias{box ref expr, box ref alias} => {
				writer._write(builder, expr)?;
				builder.push_str(" AS");
				writer._write(builder, alias)?;
			},
			&ASTNode::SQLIdentifier{ref id, ..} => {
				write!(builder, " {}", id).unwrap();
			},
			&ASTNode::SQLNested(box ref expr) => {
				builder.push_str("(");
				writer._write(builder, expr)?;
				builder.push_str(")");
			},
			&ASTNode::SQLUnary{ref operator, box ref expr} => {
				self._write_operator(builder, operator);
				writer._write(builder, expr)?;
			},
			&ASTNode::SQLOrderBy{box ref expr, ref is_asc} => {
				writer._write(builder, expr)?;
				if !is_asc {
					builder.push_str(" DESC");
				}
			},
			&ASTNode::SQLJoin{box ref left, ref join_type, box ref right, ref on_expr} => {
				writer._write(builder, left)?;
				self._write_join_type(builder, join_type);
				writer._write(builder, right)?;
				match on_expr {
					&Some(box ref e) => {
						builder.push_str(" ON");
						writer._write(builder, e)?;
					},
					&None => {}
				}
			},
			&ASTNode::SQLUnion{box ref left, ref union_type, box ref right} => {
				writer._write(builder, left)?;
				self._write_union_type(builder, union_type);
				writer._write(builder, right)?;
			}
			_ => return Ok(false)
		}

		Ok(true)
	}
}

impl AnsiSQLWriter {
	fn _write_operator(&self, builder: &mut String, op: &Operator) {
        let op_text = match op {
            &Operator::ADD => "+",
            &Operator::SUB => "-",
            &Operator::MULT => "*",
            &Operator::DIV => "/",
            &Operator::MOD => "%",
            &Operator::GT => ">",
            &Operator::LT => "<",
            // Operator::GTEQ => ">=",
            // Operator::LTEQ => "<=",
            &Operator::EQ => "=",
            // Operator::NEQ => "!=",
            &Operator::OR => "OR",
            &Operator::AND  => "AND"
        };
        write!(builder, " {}", op_text).unwrap();
    }

    fn _write_join_type(&self, builder: &mut String, join_type: &JoinType) {
        let text = match join_type {
            &JoinType::INNER => "INNER JOIN",
            &JoinType::LEFT => "LEFT JOIN",
            &JoinType::RIGHT => "RIGHT JOIN",
            &JoinType::FULL => "FULL OUTER JOIN",
            &JoinType::CROSS => "CROSS JOIN"
        };
        write!(builder, " {}", text).unwrap();
    }

    fn _write_union_type(&self, builder: &mut String, union_type: &UnionType) {
        let text = match union_type {
            &UnionType::UNION => "UNION",
            &UnionType::ALL => "UNION ALL",
            &UnionType::DISTINCT => "UNION DISTINCT"
        };
        write!(builder, " {} ", text).unwrap();
    }
}
