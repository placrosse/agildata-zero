use super::super::encrypt;
use super::super::config;
use super::ASTNode;

use encrypt::EncryptionType;
use config::*;

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
    name: String,
    encryption: EncryptionType,
//    relation: String,
//    data_type: RelType,
//    p_name: Option<String>,
//    p_relation: Option<String>
}

#[derive(Debug, Clone)]
enum Rex {
    //Alias { name: String, expr: Box<Rex> },
    Identifier { id: Vec<String>, el: Element },
    Literal,
    BinaryExpr,
    RelationalExpr(Rel),
    RexExprList(Vec<Rex>)
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
enum Rel {
    Projection { project: Box<Rex>, input: Box<Rel> , tt: TupleType},
    Selection { input: Box<Rel> },
    TableScan { table: String, tt: TupleType },
    Dual { tt: TupleType }
}

trait HasTupleType {
    fn tt<'a>(&'a self) -> &'a TupleType;
}

impl HasTupleType for Rel {
    fn tt<'a>(&'a self) -> &'a TupleType {
        match self {
            &Rel::Projection { ref tt, .. } => tt,
            &Rel::Selection { ref input, .. } => input.tt(),
            &Rel::TableScan { ref tt, .. } => tt,
            &Rel::Dual { ref tt, .. } => tt
        }
    }
}

struct Planner<'a> {
    default_schema: &'a String,
    config: &'a Config
}

impl<'a> Planner<'a> {

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
            _ => Err(String::from("oops"))
        }
    }

    fn sql_to_rel(&self, sql: &ASTNode) -> Result<Option<Rel>, String> {
        match sql {
            &ASTNode::SQLSelect { box ref expr_list, ref relation, ref selection, ref order } => {

                let mut input = match relation {
                    &Some(box ref r) => self.sql_to_rel(r)?,
                    &None => Some(Rel::Dual { tt: TupleType { elements: vec![] } })
                };

                //TODO: selection

                match input {
                    None => Ok(None),
                    Some(i) => {
                        let project_list = self.sql_to_rex(expr_list, &i.tt() )?;
                        let project_tt = reconcile_tt(&project_list);
                        Ok(Some(Rel::Projection {
                            project: Box::new(project_list),
                            input: Box::new(i),
                            tt: project_tt
                        }))
                    }
                }
            },
            &ASTNode::SQLIdentifier { ref id, ref parts } => {

                let (table_schema, table_name) = if parts.len() == 2 {
                    (&parts[0], parts[1].clone())
                } else {
                    (self.default_schema, id.clone())
                };

                if let Some(table_config) = self.config.get_table_config(table_schema, &table_name) {
                    let tt = TupleType::new(table_config.column_map
                        .iter()
                        .map(|(k,v)| Element {
                            name: v.name.clone(), encryption: v.encryption.clone()
                        })
                        .collect());
                    Ok(Some(Rel::TableScan { table: table_name.clone(), tt: tt }))
                } else {
                    Ok(None) // this isn't an encrypted table, so not our problem!
                }

            }
            //ASTNode::SQLInsert => {},
            _ => Err(String::from("oops)"))
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

        let default_schema = String::from("zero");
        let planner = Planner{default_schema: &default_schema, config: &config};

        let plan = planner.sql_to_rel(&parsed).unwrap();

        println!("Plan {:#?}", plan);
    }
}
