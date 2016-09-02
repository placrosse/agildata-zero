use super::super::encrypt;
use super::super::config;
use super::{ASTNode, Operator, LiteralExpr};

use encrypt::EncryptionType;
use config::*;
use encrypt::NativeType;

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
    pub data_type: NativeType
//    relation: String,
//    data_type: RelType,
//    p_name: Option<String>,
//    p_relation: Option<String>
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
    Dual { tt: TupleType },
    Insert {table: String, columns: Box<Rex>, values: Box<Rex>, tt: TupleType}
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
            &Rel::Insert {ref tt, ..} => tt
        }
    }
}

pub struct Planner<'a> {
    default_schema: Option<&'a String>,
    config: &'a Config
}

impl<'a> Planner<'a> {

    pub fn new(s: Option<&'a String>,
               c: &'a Config) -> Self {

        Planner { default_schema: s, config: c }
    }

    fn sql_to_rex(&self, sql: &ASTNode, tt: &TupleType) -> Result<Rex, String> {
        match sql {
            &ASTNode::SQLExprList(ref v) => Ok(Rex::RexExprList(v.iter()
                .map(|x| self.sql_to_rex(&x, tt))
                .collect()?)),
            &ASTNode::SQLIdentifier { ref id, ref parts } => {
                let element = tt.elements.iter().filter(|e| e.name == *id).next();
                match element {
                    Some(e) => Ok(Rex::Identifier{id: parts.clone(), el: e.clone()}),
                    None => Err(format!("Invalid identifier {}", id)) // TODO better..
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
            _ => Err(format!("Unsupported expr {:?}", sql))
        }
    }

    pub fn sql_to_rel(&self, sql: &ASTNode) -> Result<Option<Rel>, String> {
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
                    Some(other) => return Err(format!("Unsupported table relation for INSERT {:?}", other)),
                    None => return Ok(None)
                }

            },
            &ASTNode::SQLIdentifier { ref id, ref parts } => {

                let (table_schema, table_name) = if parts.len() == 2 {
                    (Some(&parts[0]), parts[1].clone())
                } else {
                    (self.default_schema, id.clone())
                };

                // if no default schema and no qualified identifier, then we're not handling it
                if table_schema.is_none() {
                    return Ok(None);
                }

                if let Some(table_config) = self.config.get_table_config(table_schema.unwrap(), &table_name) {
                    let tt = TupleType::new(table_config.column_map
                        .iter()
                        .map(|(k,v)| Element {
                            name: v.name.clone(), encryption: v.encryption.clone(), data_type: v.native_type.clone()
                        })
                        .collect());
                    Ok(Some(Rel::TableScan { table: table_name.clone(), tt: tt }))
                } else {
                    Ok(None) // this isn't an encrypted table, so not our problem!
                }

            },
            //&ASTNode::MySQLCreateTable{..} => Ok(None) // Dont need to plan this yet...
            //ASTNode::SQLInsert => {},
            _ => Err(format!("Unsupported expr for planning {:?}", sql))
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
    fn visit_rel(&mut self, rel: &Rel) -> Result<(), String>;
    fn visit_rex(&mut self, rex: &Rex, tt: &TupleType) -> Result<(), String>;
}

#[cfg(test)]
mod tests {

    use query::{Tokenizer, Parser};
    use config;
    use query::dialects::ansisql::*;
    use query::dialects::mysqlsql::*;

    use super::Planner;

    #[test]
    fn plan_simple() {
        let config = config::parse_config("zero-config.xml");

        let ansi = AnsiSQLDialect::new();
        let dialect = MySQLDialect::new(&ansi);

        let sql = String::from("SELECT id, first_name, last_name, ssn, age, sex FROM users");
        let parsed = sql.tokenize(&dialect).unwrap().parse().unwrap();

        let s = String::from("zero");
        let default_schema = Some(&s);
        let planner = Planner{default_schema: default_schema, config: &config};

        let plan = planner.sql_to_rel(&parsed).unwrap();

        println!("Plan {:#?}", plan);
    }

    #[test]
    fn plan_simple_selection() {
        let config = config::parse_config("zero-config.xml");

        let ansi = AnsiSQLDialect::new();
        let dialect = MySQLDialect::new(&ansi);

        let sql = String::from("SELECT id, first_name, last_name, ssn, age, sex FROM users WHERE first_name = 'Frodo'");
        let parsed = sql.tokenize(&dialect).unwrap().parse().unwrap();

        let s = String::from("zero");
        let default_schema = Some(&s);
        let planner = Planner::new(default_schema, &config);

        let plan = planner.sql_to_rel(&parsed).unwrap();

        println!("Plan {:#?}", plan);
    }

    #[test]
    fn plan_simple_insert() {
        let config = config::parse_config("zero-config.xml");

        let ansi = AnsiSQLDialect::new();
        let dialect = MySQLDialect::new(&ansi);

        let sql = String::from("INSERT INTO users  (id, first_name, last_name, ssn, age, sex) VALUES(1, 'Frodo', 'Baggins', '123456789', 50, 'M')");
        let parsed = sql.tokenize(&dialect).unwrap().parse().unwrap();

        let s = String::from("zero");
        let default_schema = Some(&s);
        let planner = Planner{default_schema: default_schema, config: &config};

        let plan = planner.sql_to_rel(&parsed).unwrap();

        println!("Plan {:#?}", plan);
    }
}
