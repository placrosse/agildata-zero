// Copyright 2016 AgilData
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http:// www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::{ASTNode, Operator, JoinType, LiteralToken};
use encrypt::EncryptionType;
use encrypt::NativeType;
use error::ZeroError;
use std::rc::Rc;

pub trait SchemaProvider {
    fn get_table_meta(&self, schema: &String, table: &String) -> Result<Option<Rc<TableMeta>>, Box<ZeroError>>;
}

#[derive(Debug, Clone)]
pub struct TableMeta {
    pub columns: Vec<ColumnMeta>
}

#[derive(Debug, Clone)]
pub struct ColumnMeta {
    pub name: String,
    pub native_type: NativeType,
    pub encryption: EncryptionType,
    pub key: [u8; 32],
}


#[derive(Debug, Clone)]
pub struct TupleType {
    pub elements: Vec<Element>
}

impl TupleType {
    fn new(elements: Vec<Element>) -> Self {
        TupleType{elements: elements}
    }
}

#[derive(Debug, Clone)]
pub struct Element {
    pub name: String,
    pub encryption: EncryptionType,
    pub key: [u8; 32],
    pub data_type: NativeType,
    pub relation: String,
    pub p_name: Option<String>,
    pub p_relation: Option<String>
}

#[derive(Debug, Clone)]
/// Relational expression
pub enum Rex {
    /// An aliased expression
    Alias { name: String, expr: Box<Rex> },
    /// An identifier
    Identifier { id: Vec<String>, el: Element },
    /// literal value with index
    Literal(usize),
    /// bound parameter with index
    BoundParam(usize),
    /// Binary expression
    BinaryExpr{left: Box<Rex>, op: Operator, right: Box<Rex>},
    /// An expression that is also a relation e.g. a subquery
    RelationalExpr(Rel),
    /// Expression-list
    RexExprList(Vec<Rex>),
    /// Unary expression
    RexUnary{operator: Operator, rex: Box<Rex>},
    /// Function call
    RexFunctionCall{name: String, args: Vec<Rex>},
    /// Nested expression
    RexNested(Box<Rex>),
    /// Order by
    RexOrderBy { expr: Box<Rex>, is_asc: bool }
}

impl Rex {

    pub fn get_element(&self) -> Result<Element, Box<ZeroError>> {
        match self {
            &Rex::Alias { box ref expr, ..} => expr.get_element(),
            &Rex::Identifier{ref el, ..} => Ok(el.clone()),
            &Rex::Literal(_) => {
                Ok(Element {
                    name : "Literal".into(), // TODO
                    encryption: EncryptionType::NA,
                    key: [0_u8; 32],
                    data_type: NativeType::UNKNOWN, // TODO
                    relation: String::from("SYS"),
                    p_name: None,
                    p_relation: None
                })

            },
            &Rex::RexFunctionCall{ref name, ref args} => {
                match &name as &str {
                    "MAX" | "SUM" | "MIN" | "COALESCE" => {
                        let elements = args.iter().map(|a| {
                            let el = a.get_element()?;
                            if el.encryption != EncryptionType::NA {
                                Err(ZeroError::EncryptionError {
                                    message: format!("Function {} does not support operation on encrypted element {}.{}",
                                                     name, el.relation, el.name).into(),
                                    code: "1064".into()
                                }.into())
                            } else {
                                Ok(el)
                            }

                        }).collect::<Result<Vec<Element>,Box<ZeroError>>>()?;

                        Ok(elements[0].clone())
                    },
                    "COUNT" => Ok(Element {
                        name : name.clone(),
                        encryption: EncryptionType::NA,
                        key: [0_u8; 32],
                        data_type: NativeType::U64,
                        relation: String::from("SYS"),
                        p_name: None,
                        p_relation: None
                    }),
                    _ => Err(ZeroError::EncryptionError {
                        message: format!("Unsupported SQL Function {}", name,).into(),
                        code: "1064".into()
                    }.into())
                }
            },
            &Rex::RexNested(ref expr) => expr.get_element(),
            &Rex::RelationalExpr(ref rel) => {
                let tt = rel.tt();
                if tt.elements.len() != 1 {
                    return Err(ZeroError::EncryptionError {
                        message: format!("Subselects returning > 1 column currently unsupported").into(),
                        code: "1064".into()
                    }.into())
                }
                Ok(tt.elements[0].clone())
            },
            _ => Err(ZeroError::EncryptionError {
                message: format!("Unsupported Rex to Element : {:?}", self).into(),
                code: "1064".into()
            }.into())
        }
    }

    pub fn to_readable(&self, literals: &Vec<LiteralToken>) -> String {
        match *self {
            Rex::Alias{ref name, box ref expr} => format!("{} as {}", expr.to_readable(literals), name),
            Rex::Identifier{ref id, ..} => id.join("."),
            Rex::Literal(ref index) => format!("{}", literals.get(index.clone()).unwrap().to_readable()),
            Rex::BoundParam(_) => "?".into(),
            Rex::BinaryExpr{box ref left, ref op, box ref right} => {
                format!("{} {} {}", left.to_readable(literals), op.to_readable(), right.to_readable(literals))
            },
            Rex::RelationalExpr(ref rel) => {
                format!("{:?}", rel) // TODO implement to_readable for rel
            },
            Rex::RexExprList(ref list) => list.iter().map(|e| e.to_readable(literals)).collect::<Vec<String>>().join(", "),
            Rex::RexUnary{ref operator, box ref rex} => format!("{}{}", operator.to_readable(), rex.to_readable(literals)),
            Rex::RexFunctionCall{ref name, ref args} => {
                format!("{}({})",
                    name,
                    args.iter().map(|e| e.to_readable(literals)).collect::<Vec<String>>().join(", ")
                )
            },
            Rex::RexNested(box ref expr) => format!("({})", expr.to_readable(literals)),
            Rex::RexOrderBy { box ref expr, is_asc } =>
                format!("{}{}", expr.to_readable(literals), if is_asc { "" } else { " DESC" }),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Rel {
    Projection { project: Box<Rex>, input: Box<Rel> , tt: TupleType},
    Sort { input: Box<Rel>, sort_expr: Box<Rex> },
    Limit { input: Box<Rel>, limit_expr: Box<Rex>},
    Selection { expr: Box<Rex>, input: Box<Rel> },
    TableScan { table: String, tt: TupleType },
    AliasedRel{alias: String, input: Box<Rel>, tt: TupleType},
    Join{left: Box<Rel>, join_type: JoinType, right: Box<Rel>, on_expr: Option<Box<Rex>>, tt: TupleType},
    Dual { tt: TupleType },
    Insert {table: String, columns: Box<Rex>, values: Vec<Rex>, tt: TupleType},
    Update {table: String, set_stmts: Box<Rex>, selection: Option<Box<Rex>>, tt: TupleType},
    Delete {table: String, selection: Option<Box<Rex>>, tt: TupleType},
    // MySQL-specific variants:
    MySQLDropTable,
    MySQLCreateTable,
    MySQLDropDatabase,
    MySQLCreateDatabase,
}

pub trait HasTupleType {
    fn tt<'a>(&'a self) -> &'a TupleType;
}

impl HasTupleType for Rel {
    fn tt<'a>(&'a self) -> &'a TupleType {
        match *self {
            Rel::Projection { ref tt, .. } => tt,
            Rel::Selection { ref input, .. } => input.tt(),
            Rel::Sort { ref input, .. } => input.tt(),
            Rel::TableScan { ref tt, .. } => tt,
            Rel::Dual { ref tt, .. } => tt,
            Rel::Insert {ref tt, ..} => tt,
            Rel::AliasedRel { ref tt, ..} => tt,
            Rel::Join { ref tt, ..} => tt,
            Rel::Update { ref tt, ..} => tt,
            Rel::Delete { ref tt, ..} => tt,
            _ => panic!("No tuple type")
        }
    }
}

pub struct Planner<'a> {
    default_schema: Option<&'a String>,
    provider: Rc<SchemaProvider>
}

impl<'a> Planner<'a> {

    pub fn new(s: Option<&'a String>,
               p: Rc<SchemaProvider>) -> Self {

        Planner { default_schema: s, provider: p }
    }

    fn sql_to_rex(&self, sql: &ASTNode, tt: &TupleType) -> Result<Rex, Box<ZeroError>> {
        match sql {
            &ASTNode::SQLExprList(ref v) => Ok(Rex::RexExprList(v.iter()
                .map(|x| self.sql_to_rex(&x, tt))
                .collect::<Result<Vec<Rex>, Box<ZeroError>>>()?)),
            &ASTNode::SQLIdentifier { ref id, ref parts } => {
                let (relation, name) = match parts.len() {
                    0 => return  Err(ZeroError::ParseError{
                            message: format!("Invalid identifier {}", id).into(),// TODO better..
                            code: "1064".into()
                        }.into()),
                    1 => (None, &parts[0]),
                    _ => (Some(&parts[0]), &parts[1])
                };

                if id.starts_with("@") {
                    let element = Element {
                        name : id.clone(),
                        encryption: EncryptionType::NA,
                        key: [0_u8; 32],
                        data_type: NativeType::UNKNOWN,
                        relation: String::from("SYS"),
                        p_name: None,
                        p_relation: None
                    };

                    Ok(Rex::Identifier { id: parts.clone(), el: element })
                } else if id == "*" {
                    // translate wildcard into list of columns for the table
                    Ok(Rex::RexExprList(tt.elements.iter()
                        .map(|e| Rex::Identifier { id: vec![e.name.clone()], el: e.clone() })
                    .collect::<Vec<Rex>>()))
                } else {
                    let element = tt.elements.iter()
                        .filter(|e| {
                            if &e.name == name {
                                match relation {
                                    Some(r) => {
                                        if &e.relation == r {
                                            true
                                        } else {
                                            match e.p_relation {
                                                Some(ref pr) => r == pr,
                                                None => false
                                            }
                                        }
                                    },
                                    None => true
                                }
                            } else {
                                false
                            }
                        })
                        .next();
                    match element {
                        Some(e) => Ok(Rex::Identifier { id: parts.clone(), el: e.clone() }),
                        None => Err(ZeroError::ParseError {
                            message: format!("Invalid identifier {}", id).into(), // TODO better..
                            code: "1064".into()
                        }.into())
                    }
                }
            },
            &ASTNode::SQLAlias { box ref expr, box ref alias } => {
                match alias {
                    &ASTNode::SQLIdentifier { ref id, .. } => {
                        Ok(Rex::Alias {
                            expr: Box::new(self.sql_to_rex(expr, tt)?),
                            name: format!("{}", id)
                        })
                    },
                    _ => panic!("TBD")
                }
            },
            &ASTNode::SQLBinary{box ref left, ref op, box ref right} => {
                Ok(Rex::BinaryExpr {
                    left: Box::new(self.sql_to_rex(left, tt)?),
                    op: op.clone(),
                    right: Box::new(self.sql_to_rex(right, tt)?)
                })
            },
            &ASTNode::SQLLiteral(ref literal) => Ok(Rex::Literal(literal.clone())),
            &ASTNode::SQLBoundParam(index) => Ok(Rex::BoundParam(index as usize)),
            &ASTNode::SQLUnary{ref operator, box ref expr} => {
                Ok(Rex::RexUnary{operator: operator.clone(), rex: Box::new(self.sql_to_rex(expr, tt)?)})
            },
            &ASTNode::SQLFunctionCall{box ref identifier, ref args} => {
                if let &ASTNode::SQLIdentifier{ref id, ..} = identifier {

                    let arguments = args.iter()
                      .map(|a| self.sql_to_rex(a, tt))
                      .collect::<Result<Vec<Rex>, Box<ZeroError>>>()?;

                    Ok(Rex::RexFunctionCall{name: id.clone().to_uppercase(), args: arguments})

                } else {
                    Err(ZeroError::ParseError{
                      message: format!("Illegal state, function name should be an identifier {:?}", identifier).into(),
                      code: "1064".into()
                    }.into())
                }

            },
            &ASTNode::SQLNested(box ref expr) => Ok(Rex::RexNested(Box::new(self.sql_to_rex(expr, tt)?))),
            &ASTNode::SQLSelect{..} | &ASTNode::SQLUnion{..} => {
                Ok(Rex::RelationalExpr(self.sql_to_rel(sql)?))
            },
            &ASTNode::SQLOrderBy { box ref expr, is_asc } => {
                Ok(Rex::RexOrderBy { expr: Box::new(self.sql_to_rex(expr, tt)?), is_asc: is_asc })
            },
            _ => Err(ZeroError::ParseError{
                message: format!("Unsupported rex expr for planning {:?}", sql).into(),
                code: "1064".into()
            }.into())
        }
    }

    pub fn sql_to_rel(&self, sql: &ASTNode) -> Result<Rel, Box<ZeroError>> {
        match *sql {
            ASTNode::SQLSelect { box ref expr_list, ref relation, ref selection, ref order, ref limit, ..  } => {
                let mut input = match relation {
                    &Some(box ref r) => self.sql_to_rel(r)?,
                    &None => Rel::Dual { tt: TupleType { elements: vec![] } }
                };

                match selection {
                    &Some(box ref expr) => {
                        let filter = self.sql_to_rex(expr, input.tt())?;
                        input = Rel::Selection { expr: Box::new(filter), input: Box::new(input) }
                    },
                    &None => {}
                }

                let project_list = self.sql_to_rex(expr_list, &input.tt())?;

                let project_list = match project_list {
                    Rex::RexExprList(ref list) => {
                        let mut new_list: Vec<Rex> = vec![];
                        for rex in list {
                            match rex {
                                &Rex::RexExprList(ref list) => {
                                    for e in list {
                                        new_list.push(e.clone())
                                    }
                                },
                                _ => new_list.push(rex.clone())
                            }
                        }
                        Rex::RexExprList(new_list)
                    },
                    _ => panic!("Invalid projection expression")
                };

                let project_tt = reconcile_tt(&project_list)?;

                input = Rel::Projection {
                    project: Box::new(project_list),
                    input: Box::new(input),
                    tt: project_tt
                };

                match order {
                    &Some(box ref o) => {
                        let sort_expr = Box::new(self.sql_to_rex(o, &input.tt())?);
                        input = Rel::Sort{
                            input: Box::new(input),
                            sort_expr: sort_expr
                        };
                    },
                    &None =>{}
                }

                match limit {
                    &Some(box ref l) => {
                        let limit_expr = Box::new(self.sql_to_rex(l, &input.tt())?);
                        input = Rel::Limit {
                            input: Box::new(input),
                            limit_expr: limit_expr
                        };
                    },
                    &None => {}
                }

                Ok(input)

            },
            ASTNode::SQLInsert {box ref table, box ref column_list, ref values_list, .. } => {
                match self.sql_to_rel(table)? {
                    Rel::TableScan {table, tt} => {
                        let values: Result<Vec<_>, _> = values_list.iter().map(|v| self.sql_to_rex(v, &tt)).collect();
                        match column_list {
                            &ASTNode::SQLExprList(ref v) => if v.len() == 0 {
                                let columns = Rex::RexExprList(tt.elements.iter()
                                    .map(|e| Rex::Identifier { id: vec![e.name.clone()], el: e.clone() })
                                    .collect::<Vec<Rex>>());
                                Ok(Rel::Insert {
                                    table: table,
                                    columns: Box::new(columns),
                                    values: values?,
                                    tt: tt
                                })
                            } else {
                                Ok(Rel::Insert {
                                    table: table,
                                    columns: Box::new(self.sql_to_rex(column_list, &tt)?),
                                    values: values?,
                                    tt: tt
                                })
                            },
                            _ => Err(ZeroError::ParseError {
                                message: format!("Unsupported expr for column list").into(),
                                code: "1064".into()
                            }.into())
                        }
                    },
                    other @ _ => return Err(ZeroError::ParseError{
                        message: format!("Unsupported table relation for INSERT {:?}", other).into(),
                        code: "1064".into()
                    }.into()),
                }
            },
            ASTNode::SQLJoin{box ref left, ref join_type, box ref right, ref on_expr} => {
                let left_rel = self.sql_to_rel(left)?;
                let right_rel = self.sql_to_rel(right)?;

                let mut merged: Vec<Element> = Vec::new();
                merged.extend(left_rel.tt().elements.clone());
                merged.extend(right_rel.tt().elements.clone());

                let merged_tt = TupleType::new(merged);

                let on_rex = match on_expr {
                    &Some(box ref o) => Some(Box::new(self.sql_to_rex(o, &merged_tt)?)),
                    &None => None
                };

                Ok(Rel::Join{
                    left: Box::new(left_rel),
                    join_type: join_type.clone(),
                    right: Box::new(right_rel),
                    on_expr: on_rex,
                    tt: merged_tt
                })

            },
            ASTNode::SQLAlias{box ref expr, box ref alias} => {

                let input = self.sql_to_rel(expr)?;
                let a = match alias {
                    &ASTNode::SQLIdentifier{ref id, ..} => id.clone(),
                    _ => return Err(ZeroError::ParseError {
                            message: format!("Unsupported alias expr {:?}", alias).into(),
                            code: "1064".into()
                        }.into())
                };

                let tt = TupleType::new(input.tt().elements.iter().map(|e| Element{
                    name: e.name.clone(), encryption: e.encryption.clone(), key: e.key.clone(),
                    data_type: e.data_type.clone(), relation: a.clone(),
                    p_name: e.p_name.clone(), p_relation: Some(e.relation.clone())
                }).collect());

                Ok(Rel::AliasedRel{alias: a, input: Box::new(input), tt: tt})
            },
            ASTNode::SQLIdentifier { ref id, ref parts } => {

                let (table_schema, table_name) = if parts.len() == 2 {
                    (Some(&parts[0]), parts[1].clone())
                } else {
                    (self.default_schema, id.clone())
                };

                match table_schema {
                    None => Err(ZeroError::ParseError {
                                message: format!("Invalid table {}", table_name).into(),
                                code: "1064".into()
                            }.into()),
                    Some(schema) => {
                        match self.provider.get_table_meta(&schema, &table_name)? {
                            Some(meta) => {
                                let tt = TupleType::new(
                                    meta.columns.iter()
                                        .map(|c| Element {
                                            name: c.name.clone(), encryption: c.encryption.clone(), key: c.key.clone(),
                                            data_type: c.native_type.clone(), relation: table_name.clone(),
                                            p_name: None, p_relation: None
                                        })
                                        .collect()
                                );
                                Ok(Rel::TableScan { table: table_name.clone(), tt: tt })
                            },
                            None => Err(ZeroError::ParseError {
                                message: format!("Invalid table {}.{}", schema, table_name).into(),
                                code: "1064".into()
                            }.into())
                        }
                    }
                }

            },
            ASTNode::MySQLDropTable{..} => Ok(Rel::MySQLDropTable),
            ASTNode::MySQLCreateTable{..} => Ok(Rel::MySQLCreateTable),
            ASTNode::MySQLDropDatabase{..} => Ok(Rel::MySQLDropDatabase),
            ASTNode::MySQLCreateDatabase{..} => Ok(Rel::MySQLCreateDatabase),

            ASTNode::SQLUpdate{ box ref table, box ref assignments, ref selection } => {
                let (table, tt) = match self.sql_to_rel(table)? {
                    Rel::TableScan{table, tt} => (table, tt),
                    o @ _ => return Err(ZeroError::ParseError {
                        message: format!("Invalid rel for SQLUpdate table {:?}", o).into(),
                        code: "1064".into()
                    }.into())
                };

                Ok(Rel::Update{
                    table: table,
                    set_stmts: Box::new(self.sql_to_rex(assignments, &tt)?),
                    selection: match selection {
                        &Some(box ref expr) => Some(Box::new(self.sql_to_rex(expr, &tt)?)),
                        &None => None
                    },
                    tt: tt})

            },
            ASTNode::SQLDelete{ box ref table, ref selection } => {

                let (table, tt) = match self.sql_to_rel(table)? {
                    Rel::TableScan{table, tt} => (table, tt),
                    o @ _ => return Err(ZeroError::ParseError {
                        message: format!("Invalid rel for SQLDelete table {:?}", o).into(),
                        code: "1064".into()
                    }.into())
                };

                Ok(Rel::Delete {
                    table: table,
                    selection: match selection {
                        &Some(box ref expr) => Some(Box::new(self.sql_to_rex(expr, &tt)?)),
                        &None => None
                    },
                    tt: tt})

            },

            _ => Err(ZeroError::ParseError {
                    message: format!("Unsupported rel expr for planning {:?}", sql).into(),
                    code: "1064".into()
                }.into())

        }
    }
}

fn reconcile_tt(expr: &Rex) -> Result<TupleType, Box<ZeroError>> {
    match expr {
        &Rex::RexExprList(ref list) => {
            let elements = list.iter().map(|e| e.get_element())
                .collect::<Result<Vec<Element>, Box<ZeroError>>>()?;
            Ok(TupleType{elements: elements})
        },
        _ => panic!("Unsupported")
    }
}


pub trait RelVisitor {
    fn visit_rel(&mut self, rel: &Rel) -> Result<(), Box<ZeroError>>;
    fn visit_rex(&mut self, rex: &Rex, tt: &TupleType) -> Result<(), Box<ZeroError>>;
}

// TODO these tests need real assertions
#[cfg(test)]
mod tests {

    use query::{Tokenizer, Parser, ASTNode};
    use query::dialects::ansisql::*;
    use query::dialects::mysqlsql::*;
    use encrypt::{NativeType, EncryptionType};
    use std::rc::Rc;
    use super::{Planner, SchemaProvider, TableMeta, ColumnMeta, Rel};
    use error::ZeroError;


    #[test]
    fn plan_simple() {
        let provider = DummyProvider{};

        let ansi = AnsiSQLDialect::new();
        let dialect = MySQLDialect::new(&ansi);

        let sql = String::from("SELECT id, first_name, last_name, ssn, age, sex FROM users");
        let parsed = sql.tokenize(&dialect).unwrap().parse().unwrap();

        let s = String::from("zero");
        let default_schema = Some(&s);
        let planner = Planner{default_schema: default_schema, provider: Rc::new(provider) };

        let plan = planner.sql_to_rel(&parsed).unwrap();

        debug!("Plan {:#?}", plan);
    }

    #[test]
    fn plan_simple_selection() {
        let provider = DummyProvider{};

        let ansi = AnsiSQLDialect::new();
        let dialect = MySQLDialect::new(&ansi);

        let sql = String::from("SELECT id, first_name, last_name, ssn, age, sex FROM users WHERE first_name = 'Frodo'");
        let parsed = sql.tokenize(&dialect).unwrap().parse().unwrap();

        let s = String::from("zero");
        let default_schema = Some(&s);
        let planner = Planner::new(default_schema, Rc::new(provider));

        let plan = planner.sql_to_rel(&parsed).unwrap();

        debug!("Plan {:#?}", plan);
    }

    #[test]
    fn plan_simple_delete() {
        let provider = DummyProvider{};

        let ansi = AnsiSQLDialect::new();
        let dialect = MySQLDialect::new(&ansi);

        let sql = String::from("DELETE FROM users WHERE first_name = 'Frodo'");
        let parsed = sql.tokenize(&dialect).unwrap().parse().unwrap();

        let s = String::from("zero");
        let default_schema = Some(&s);
        let planner = Planner::new(default_schema, Rc::new(provider));

        let plan = planner.sql_to_rel(&parsed).unwrap();

        debug!("Plan {:#?}", plan);
    }

    #[test]
    fn plan_simple_update() {
        let provider = DummyProvider{};

        let ansi = AnsiSQLDialect::new();
        let dialect = MySQLDialect::new(&ansi);

        let sql = String::from("UPDATE users SET first_name = 'Hobbit' WHERE first_name = 'Frodo'");
        let parsed = sql.tokenize(&dialect).unwrap().parse().unwrap();

        let s = String::from("zero");
        let default_schema = Some(&s);
        let planner = Planner::new(default_schema, Rc::new(provider));

        let plan = planner.sql_to_rel(&parsed).unwrap();

        debug!("Plan {:#?}", plan);
    }

    #[test]
    fn plan_simple_insert() {
        let provider = DummyProvider{};

        let ansi = AnsiSQLDialect::new();
        let dialect = MySQLDialect::new(&ansi);

        let sql = String::from("INSERT INTO users  (id, first_name, last_name, ssn, age, sex) VALUES(1, 'Frodo', 'Baggins', '123456789', 50, 'M')");
        let parsed = sql.tokenize(&dialect).unwrap().parse().unwrap();

        let s = String::from("zero");
        let default_schema = Some(&s);
        let planner = Planner{default_schema: default_schema, provider: Rc::new(provider) };

        let plan = planner.sql_to_rel(&parsed).unwrap();

        debug!("Plan {:#?}", plan);
    }

    #[test]
    fn plan_simple_join() {
        let provider = DummyProvider{};

        let ansi = AnsiSQLDialect::new();
        let dialect = MySQLDialect::new(&ansi);

        let sql = String::from("SELECT l.id, r.id, l.first_name, r.user_id
         FROM users AS l
         JOIN user_purchases AS r ON l.id = r.user_id");
        let parsed = sql.tokenize(&dialect).unwrap().parse().unwrap();

        let s = String::from("zero");
        let default_schema = Some(&s);
        let planner = Planner::new(default_schema, Rc::new(provider));

        let plan = planner.sql_to_rel(&parsed).unwrap();

        debug!("Plan {:#?}", plan);
    }

    #[test]
    fn plan_simple_func_calls() {
        let provider = DummyProvider{};

        let ansi = AnsiSQLDialect::new();
        let dialect = MySQLDialect::new(&ansi);

        let sql = String::from("SELECT COUNT(id)
         FROM users");
        let parsed = sql.tokenize(&dialect).unwrap().parse().unwrap();

        let s = String::from("zero");
        let default_schema = Some(&s);
        let planner = Planner::new(default_schema, Rc::new(provider));

        let plan = planner.sql_to_rel(&parsed).unwrap();

        debug!("Plan {:#?}", plan);
    }

    #[test]
    fn plan_rel_as_rex() {

        let sql = String::from("SELECT id FROM users WHERE id = (SELECT id FROM users)");
        let res = parse_and_plan(sql).unwrap();
        let plan = res.1;

        debug!("Plan {:#?}", plan);
    }

    #[test]
    fn plan_select_with_nulls() {

        let sql = String::from("SELECT id FROM users WHERE id = NULL");
        let res = parse_and_plan(sql).unwrap();
        let plan = res.1;

        println!("Plan {:#?}", plan);
    }

    #[test]
    fn plan_insert_with_nulls() {

        let sql = String::from("INSERT INTO users  (id, first_name, last_name, ssn, age, sex) VALUES(NULL, null, null, NULL, null, NULL)");
        let res = parse_and_plan(sql).unwrap();
        let plan = res.1;

        println!("Plan {:#?}", plan);
    }

    #[test]
    fn plan_with_alias() {

        let sql = String::from("SELECT @@foo as bar, @@bar as foo");
        let res = parse_and_plan(sql).unwrap();
        let plan = res.1;

        println!("Plan {:#?}", plan);
    }

    #[test]
    fn plan_unsupported() {
        let sql = String::from("SELECT COALESCE(id, first_name, 'foo') FROM users ");
        let plan = parse_and_plan(sql);


        match plan {
            Err(box ZeroError::EncryptionError{message, ..}) => assert_eq!(message, String::from("Function COALESCE does not support operation on encrypted element users.first_name")),
            _ => panic!("This should fail")
        }


    }

    fn parse_and_plan(sql: String) -> Result<(ASTNode, Rel), Box<ZeroError>> {
        let provider = DummyProvider{};

        let ansi = AnsiSQLDialect::new();
        let dialect = MySQLDialect::new(&ansi);

        let parsed = sql.tokenize(&dialect)?.parse()?;

        let s = String::from("zero");
        let default_schema = Some(&s);
        let planner = Planner::new(default_schema, Rc::new(provider));
        let plan = planner.sql_to_rel(&parsed)?;
        Ok((parsed, plan))

    }

    struct DummyProvider {}
    impl SchemaProvider for DummyProvider {
        fn get_table_meta(&self, schema: &String, table: &String) -> Result<Option<Rc<TableMeta>>, Box<ZeroError>> {

            let rc = match (schema as &str, table as &str) {
                ("zero", "users") => {
                    Some(Rc::new(TableMeta {
                        columns: vec![
                            ColumnMeta {name: String::from("id"), native_type: NativeType::U64,
                                        encryption: EncryptionType::NA,
                                        key: [0u8; 32]},
                            ColumnMeta {name: String::from("first_name"), native_type: NativeType::Varchar(50),
                                        encryption: EncryptionType::Aes([0u8;12]),
                                        key: [0u8; 32]},
                            ColumnMeta {name: String::from("last_name"), native_type: NativeType::Varchar(50),
                                        encryption: EncryptionType::Aes([0u8;12]),
                                        key: [0u8; 32]},
                            ColumnMeta {name: String::from("ssn"), native_type: NativeType::Varchar(50),
                                        encryption: EncryptionType::Aes([0u8;12]),
                                        key: [0u8; 32]},
                            ColumnMeta {name: String::from("age"), native_type: NativeType::U64,
                                        encryption: EncryptionType::Aes([0u8;12]),
                                        key: [0u8; 32]},
                            ColumnMeta {name: String::from("sex"), native_type: NativeType::Varchar(50),
                                        encryption: EncryptionType::Aes([0u8;12]),
                                        key: [0u8; 32]},
                        ]
                    }))
                },
                ("zero", "user_purchases") => {
                    Some(Rc::new(TableMeta {
                        columns: vec![
                            ColumnMeta {name: String::from("id"), native_type: NativeType::U64,
                                        encryption: EncryptionType::NA,
                                        key: [0u8; 32]},
                            ColumnMeta {name: String::from("user_id"), native_type: NativeType::U64,
                                        encryption: EncryptionType::NA,
                                        key: [0u8; 32]},
                            ColumnMeta {name: String::from("item_code"), native_type: NativeType::U64,
                                        encryption: EncryptionType::Aes([0u8;12]),
                                        key: [0u8; 32]},
                            ColumnMeta {name: String::from("amount"), native_type: NativeType::F64,
                                        encryption: EncryptionType::Aes([0u8;12]),
                                        key: [0u8; 32]},
                        ]
                    }))
                },
                _ => None
            };
            Ok(rc)
        }

    }
}
