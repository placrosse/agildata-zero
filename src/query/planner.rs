use std::error::Error;
use super::{ASTNode, Operator, LiteralExpr, JoinType};
use encrypt::EncryptionType;
use config::*;
use encrypt::NativeType;
use std::rc::Rc;

pub trait SchemaProvider {
    fn get_table_meta(&self, schema: &String, table: &String) -> Result<Option<Rc<TableMeta>>, Box<Error>>;
}

#[derive(Debug, Clone)]
pub struct TableMeta {
    pub columns: Vec<ColumnMeta>
}

#[derive(Debug, Clone)]
pub struct ColumnMeta {
    pub name: String,
    pub native_type: NativeType,
    pub encryption: EncryptionType
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
    pub data_type: NativeType,
    pub relation: String,
    pub p_name: Option<String>,
    pub p_relation: Option<String>
}

#[derive(Debug, Clone)]
pub enum Rex {
    //Alias { name: String, expr: Box<Rex> },
    Identifier { id: Vec<String>, el: Element },
    Literal(LiteralExpr),
    BinaryExpr{left: Box<Rex>, op: Operator, right: Box<Rex>},
    RelationalExpr(Rel),
    RexExprList(Vec<Rex>),
}

impl Rex {
    fn name(&self) -> String {
        match self {
            &Rex::Identifier { ref el, .. } => el.name.clone(),
            _ => panic!("")
        }
    }
}

#[derive(Debug, Clone)]
pub enum Rel {
    Projection { project: Box<Rex>, input: Box<Rel> , tt: TupleType},
    Selection { expr: Box<Rex>, input: Box<Rel> },
    TableScan { table: String, tt: TupleType },
    AliasedRel{alias: String, input: Box<Rel>, tt: TupleType},
    Join{left: Box<Rel>, join_type: JoinType, right: Box<Rel>, on_expr: Option<Box<Rex>>, tt: TupleType},
    Dual { tt: TupleType },
    Insert {table: String, columns: Box<Rex>, values: Box<Rex>, tt: TupleType},
	Update {table: String, assns: Box<Rex>, select: Box<Rel>}    
}

pub trait HasTupleType {
    fn tt<'a>(&'a self) -> &'a TupleType;
}

impl HasTupleType for Rel {
    fn tt<'a>(&'a self) -> &'a TupleType {
        match self {
            &Rel::Projection { ref tt, .. } => tt,
            &Rel::Selection { ref input, .. } => input.tt(),
            &Rel::TableScan { ref tt, .. } => tt,
            &Rel::Dual { ref tt, .. } => tt,
            &Rel::Insert {ref tt, ..} => tt,
            &Rel::AliasedRel{ref tt, ..} => tt,
            &Rel::Join{ref tt, ..} => tt,
            &Rel::Update{ref select, ..} => select.tt()
        }
    }
}

pub struct Planner<'a> {
    default_schema: Option<&'a String>,
    provider: &'a SchemaProvider
}

impl<'a> Planner<'a> {

    pub fn new(s: Option<&'a String>,
               p: &'a SchemaProvider) -> Self {

        Planner { default_schema: s, provider: p }
    }

    fn sql_to_rex(&self, sql: &ASTNode, tt: &TupleType) -> Result<Rex, Box<Error>> {
        match sql {
            &ASTNode::SQLExprList(ref v) => Ok(Rex::RexExprList(v.iter()
                .map(|x| self.sql_to_rex(&x, tt))
                .collect()?)),
            &ASTNode::SQLIdentifier { ref id, ref parts } => {
                let (relation, name) = match parts.len() {
                    0 => return Err(format!("Illegal identifier {:?}", id).into()),
                    1 => (None, &parts[0]),
                    _ => (Some(&parts[0]), &parts[1])
                };

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
                    Some(e) => Ok(Rex::Identifier{id: parts.clone(), el: e.clone()}),
                    None => Err(format!("Invalid identifier {}", id).into()) // TODO better..
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
            _ => Err(format!("Unsupported expr {:?}", sql).into())
        }
    }

    pub fn sql_to_rel(&self, sql: &ASTNode) -> Result<Option<Rel>, Box<Error>> {
        match sql {
            &ASTNode::SQLSelect { box ref expr_list, ref relation, ref selection, ref order } => {

                let relation = match relation {
                    &Some(box ref r) => self.sql_to_rel(r)?,
                    &None => Some(Rel::Dual { tt: TupleType { elements: vec![] } })
                };

                let mut input = if let Some(r) = relation {
                    r
                } else {
                    return Ok(None)
                };


                match selection {
                    &Some(box ref expr) => {
                        let filter = self.sql_to_rex(expr, input.tt())?;
                        input = Rel::Selection { expr: Box::new(filter), input: Box::new(input)}
                    },
                    &None => {}
                }

                let project_list = self.sql_to_rex(expr_list, &input.tt() )?;
                let project_tt = reconcile_tt(&project_list);
                Ok(Some(Rel::Projection {
                    project: Box::new(project_list),
                    input: Box::new(input),
                    tt: project_tt
                }))
            },
            &ASTNode::SQLInsert {box ref table, box ref column_list, box ref values_list} => {
                match self.sql_to_rel(table)? {
                    Some(Rel::TableScan {table, tt}) => {
                        Ok(Some(Rel::Insert{
                            table: table,
                            columns: Box::new(self.sql_to_rex(column_list, &tt)?),
                            values: Box::new(self.sql_to_rex(values_list, &tt)?),
                            tt: tt
                        }))
                    },
                    Some(other) => return Err(format!("Unsupported table relation for INSERT {:?}", other).into()),
                    None => return Ok(None)
                }
            },
            //     SQLJoin{left: Box<ASTNode>, join_type: JoinType, right: Box<ASTNode>, on_expr: Option<Box<ASTNode>>},

            &ASTNode::SQLJoin{box ref left, ref join_type, box ref right, ref on_expr} => {
                let left_rel = self.sql_to_rel(left)?;
                let right_rel = self.sql_to_rel(right)?;

                match (left_rel, right_rel) {
                    //Both relations we control
                    (Some(l), Some(r)) => {
                        let mut merged: Vec<Element> = Vec::new();
                        merged.extend(l.tt().elements.clone());
                        merged.extend(r.tt().elements.clone());

                        let merged_tt = TupleType::new(merged);

                        let on_rex = match on_expr {
                            &Some(box ref o) => Some(Box::new(self.sql_to_rex(o, &merged_tt)?)),
                            &None => None
                        };

                        Ok(Some(Rel::Join{
                            left: Box::new(l),
                            join_type: join_type.clone(),
                            right: Box::new(r),
                            on_expr: on_rex,
                            tt: merged_tt
                        }))
                    },
                    // Neither relation we control
                    (None, None) => Ok(None),
                    // Mismatch
                    (Some(_), None) | (None, Some(_)) => {
                        Err(String::from("Unsupported: Mismatch join between encrypted and unencrypted relations").into())
                    }
                }
            },
            &ASTNode::SQLAlias{box ref expr, box ref alias} => {

                let input = self.sql_to_rel(expr)?;
                let a = match alias {
                    &ASTNode::SQLIdentifier{ref id, ..} => id.clone(),
                    _ => return Err(format!("Unsupported alias expr {:?}", alias).into())
                };

                match input {
                    Some(i) => {
                        let tt = TupleType::new(i.tt().elements.iter().map(|e| Element{
                            name: e.name.clone(), encryption: e.encryption.clone(),
                            data_type: e.data_type.clone(), relation: a.clone(),
                            p_name: e.p_name.clone(), p_relation: Some(e.relation.clone())
                        }).collect());

                        Ok(Some(Rel::AliasedRel{alias: a, input: Box::new(i), tt: tt}))
                    },
                    None => Ok(None) // TODO expected behaviour?
                }
            },
            &ASTNode::SQLIdentifier { ref id, ref parts } => {

                let (table_schema, table_name) = if parts.len() == 2 {
                    (Some(&parts[0]), parts[1].clone())
                } else {
                    (self.default_schema, id.clone())
                };


                match self.provider.get_table_meta(&table_schema.unwrap(), &table_name)? {
                    Some(meta) => {
                        let tt = TupleType::new(
                            meta.columns.iter()
                                .map(|c| Element {
                                    name: c.name.clone(), encryption: c.encryption.clone(),
                                    data_type: c.native_type.clone(), relation: table_name.clone(),
                                    p_name: None, p_relation: None
                                })
                                .collect()
                        );
                        Ok(Some(Rel::TableScan { table: table_name.clone(), tt: tt }))
                    },
                    None => Err(format!("Invalid table {}.{}", table_schema.unwrap(), table_name).into())
                }
            },
            &ASTNode::MySQLCreateTable{..} => Ok(None), // Dont need to plan this yet...
            &ASTNode::SQLUpdate{ box ref table, ref assignments, ref selection } => {
            	
            	let rel = self.sql_to_rel(table)?;

				// Ok(Some(Rel::Update { rel, set_list, input }))
				Ok(None)
            }
            _ => Err(format!("Unsupported expr for planning {:?}", sql).into())
        }
    }
}

fn reconcile_tt(expr: &Rex) -> TupleType {
    match expr {
        &Rex::RexExprList(ref list) => {
            let elements = list.iter().map(|e| get_element(e)).collect();
            TupleType{elements: elements}
        },
        _ => panic!("Unsupported")
    }
}

fn get_element(expr: &Rex) -> Element {
    match expr {
        &Rex::Identifier{ref el, ..} => el.clone(),
        _ => panic!("Unsupported")
    }
}

pub trait RelVisitor {
    fn visit_rel(&mut self, rel: &Rel) -> Result<(), Box<Error>>;
    fn visit_rex(&mut self, rex: &Rex, tt: &TupleType) -> Result<(), Box<Error>>;
}

#[cfg(test)]
mod tests {

    use query::{Tokenizer, Parser};
    use config;
    use query::dialects::ansisql::*;
    use query::dialects::mysqlsql::*;
    use std::error::Error;
    use encrypt::{NativeType, EncryptionType};
    use std::rc::Rc;
    use super::{Planner, SchemaProvider, TableMeta, ColumnMeta};

    #[test]
    fn plan_simple() {
        let provider = DummyProvider{};

        let ansi = AnsiSQLDialect::new();
        let dialect = MySQLDialect::new(&ansi);

        let sql = String::from("SELECT id, first_name, last_name, ssn, age, sex FROM users");
        let parsed = sql.tokenize(&dialect).unwrap().parse().unwrap();

        let s = String::from("zero");
        let default_schema = Some(&s);
        let planner = Planner{default_schema: default_schema, provider: &provider};

        let plan = planner.sql_to_rel(&parsed).unwrap();

        println!("Plan {:#?}", plan);
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
        let planner = Planner::new(default_schema, &provider);

        let plan = planner.sql_to_rel(&parsed).unwrap();

        println!("Plan {:#?}", plan);
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
        let planner = Planner{default_schema: default_schema, provider: &provider};

        let plan = planner.sql_to_rel(&parsed).unwrap();

        println!("Plan {:#?}", plan);
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
        let planner = Planner::new(default_schema, &provider);

        let plan = planner.sql_to_rel(&parsed).unwrap();

        println!("Plan {:#?}", plan);
    }

    struct DummyProvider {}
    impl SchemaProvider for DummyProvider {
        fn get_table_meta(&self, schema: &String, table: &String) -> Result<Option<Rc<TableMeta>>, Box<Error>> {

            let rc = match (schema as &str, table as &str) {
                ("zero", "users") => {
                    Some(Rc::new(TableMeta {
                        columns: vec![
                            ColumnMeta {name: String::from("id"), native_type: NativeType::U64, encryption: EncryptionType::NA},
                            ColumnMeta {name: String::from("first_name"), native_type: NativeType::Varchar(50), encryption: EncryptionType::AES},
                            ColumnMeta {name: String::from("last_name"), native_type: NativeType::Varchar(50), encryption: EncryptionType::AES},
                            ColumnMeta {name: String::from("ssn"), native_type: NativeType::Varchar(50), encryption: EncryptionType::AES},
                            ColumnMeta {name: String::from("age"), native_type: NativeType::U64, encryption: EncryptionType::AES},
                            ColumnMeta {name: String::from("sex"), native_type: NativeType::Varchar(50), encryption: EncryptionType::AES},

                        ]
                    }))
                },
                ("zero", "user_purchases") => {
                    Some(Rc::new(TableMeta {
                        columns: vec![
                            ColumnMeta {name: String::from("id"), native_type: NativeType::U64, encryption: EncryptionType::NA},
                            ColumnMeta {name: String::from("user_id"), native_type: NativeType::U64, encryption: EncryptionType::NA},
                            ColumnMeta {name: String::from("item_code"), native_type: NativeType::U64, encryption: EncryptionType::AES},
                            ColumnMeta {name: String::from("amount"), native_type: NativeType::F64, encryption: EncryptionType::AES}
                        ]
                    }))
                },
                _ => None
            };
            Ok(rc)
        }

    }
}
