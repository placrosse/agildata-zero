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
    //Alias { name: String, expr: Box<Rex> },
    Identifier { id: Vec<String>, el: Element },
    Literal,
    BinaryExpr,
    RelationalExpr(Rel),
    RexExprList(Vec<Rex>)
}

enum Rel {
    Projection { project: Box<Rex>, input: Box<Rel> },
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
            &Rel::Projection { ref input, .. } => {
                //TODO: need to filter input's tuple type based on actual projection
                input.tt()
            },
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
                let _ = tt.elements.iter().filter(|e| e.name == *id);
                //                Ok(Rex::Identifier {
                //                    id : parts.clone(), el:  })
                Err(String::from(""))
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
                        Ok(Some(Rel::Projection {
                            project: Box::new(self.sql_to_rex(expr_list, &i.tt() )?),
                            input: Box::new(i)
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