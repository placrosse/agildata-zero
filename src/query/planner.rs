use super::super::encrypt;
use super::super::config;
use super::ASTNode;

use encrypt::EncryptionType;
use config::*;

#[derive(Debug)]
pub struct TupleType {
    pub elements: Vec<Element>
}

impl TupleType {
    fn new(elements: Vec<Element>) -> Self {
        TupleType{elements: elements}
    }
}

#[derive(Debug)]
pub struct Element {
    name: String,
    encryption: EncryptionType,
//    relation: String,
//    data_type: RelType,
//    p_name: Option<String>,
//    p_relation: Option<String>
}

enum Rex {
    Identifier(Vec<String>),
    Literal,
    BinaryExpr,
    RelationalExpr(Rel),
    RexExprList(Vec<Rex>)
}

enum Rel {
    Projection { project: Box<Rex>, input: Box<Rel> },
    Selection,
    TableScan { table: String, tt: TupleType },
    Dual
}

struct Planner {
    schema: String,
    config: Config
}

impl Planner {

    fn sql_to_rex(&self, sql: &ASTNode) -> Result<Rex, String> {
        match sql {
            &ASTNode::SQLExprList(ref v) => Ok(Rex::RexExprList(v.iter()
                .map(|x| self.sql_to_rex(&x))
                .collect()?)),
            &ASTNode::SQLIdentifier { ref id, ref parts } => Ok(Rex::Identifier(parts.clone())),
            _ => Err(String::from("oops"))
        }
    }

    fn sql_to_rel(&self, sql: &ASTNode) -> Result<Option<Rel>, String> {
        match sql {
            &ASTNode::SQLSelect { box ref expr_list, ref relation, ref selection, ref order } => {

                let mut input = match relation {
                    &Some(box ref r) => self.sql_to_rel(r)?,
                    &None => Some(Rel::Dual)
                };

                //TODO: selection

                match input {
                    None => Ok(None),
                    Some(i) => {
                        Ok(Some(Rel::Projection {
                            project: Box::new(self.sql_to_rex(expr_list)?),
                            input: Box::new(i)
                        }))
                    }
                }
            },
            &ASTNode::SQLIdentifier { id: ref table_name, parts: ref table_name_parts } => {

                if let Some(table_config) = self.config.get_table_config(&self.schema, table_name) {
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