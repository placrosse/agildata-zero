use query::planner::{Rel, Rex, TupleType, Element, HasTupleType};
use encrypt::{NativeType, EncryptionType};
use query::{Token, ASTNode, LiteralToken, Operator};
use error::ZeroError;

use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub enum ValueType {
    BOUND_PARAM(u32),
    LITERAL(u32),
    COLUMN
}

#[derive(Debug, PartialEq)]
pub struct EncryptionPlan {
    data_type: NativeType,
    encryption: EncryptionType,
    key: Option<[u8; 32]>
//    value_type: ValueType
}

#[derive(Debug, PartialEq)]
pub struct PPlan {
    literals: HashMap<usize, EncryptionPlan>,
    params: HashMap<usize, EncryptionPlan>,
    projection: Vec<EncryptionPlan>,
    ast: ASTNode
}

#[derive(Debug)]
pub enum PhysicalPlan {
    Plan(PPlan),
    Passthrough,
    Error(Box<ZeroError>)
}

pub struct PhysicalPlanBuilder {
    literals: HashMap<usize, EncryptionPlan>,
    params: HashMap<usize, EncryptionPlan>,
    projection: Vec<EncryptionPlan>
}

impl PhysicalPlanBuilder {

    fn new() -> Self {
        PhysicalPlanBuilder {
            literals: HashMap::new(),
            params: HashMap::new(),
            projection: Vec::new()
        }
    }

    // build consumes self
    fn build(self, ast: ASTNode) -> PhysicalPlan {
        PhysicalPlan::Plan(
            PPlan {
                literals: self.literals,
                params: self.params,
                projection: self.projection,
                ast: ast
            }
        )
    }

    fn push_literal(&mut self, index: usize, e: EncryptionPlan) {
        self.literals.insert(index, e);
    }

    fn push_param(&mut self, index: usize, e: EncryptionPlan) {
        self.params.insert(index, e);
    }

    fn push_projection(&mut self, e: EncryptionPlan) {
        self.projection.push(e);
    }
}

pub struct PhysicalPlanner {}

impl PhysicalPlanner {
    pub fn plan(&self, logical: Rel, ast: ASTNode) -> PhysicalPlan {
        let mut builder = PhysicalPlanBuilder::new();

        match self.plan_rel(&logical, &mut builder) {
            Ok(()) => builder.build(ast),
            Err(e) => PhysicalPlan::Error(e)
        }

    }

    fn plan_rel(&self, rel: &Rel, builder: &mut PhysicalPlanBuilder) -> Result<(), Box<ZeroError>> {
        match *rel {
            Rel::Projection { box ref project, box ref input, ref tt } => {
                self.plan_rex(project, builder, tt)?;
                self.plan_rel(input, builder)?;

                // push projection encryption types into builder
                for el in tt.elements.iter() {

                    let enc_plan = EncryptionPlan {
                        data_type: el.data_type.clone(),
                        encryption: el.encryption.clone(),
                        key: Some(el.key.clone())
                    };

                    builder.push_projection(enc_plan);
                }
            },
            Rel::Selection { box ref expr, box ref input } => {
                self.plan_rex(expr, builder, input.tt())?;
                self.plan_rel(input, builder)?;
            },
            Rel::TableScan { .. } => {},
            Rel::Join { box ref left, box ref right, ref on_expr, ref tt, .. } => {
                self.plan_rel(left, builder)?;
                self.plan_rel(right, builder)?;
                match on_expr {
                    &Some(box ref o) => self.plan_rex(o, builder, tt)?,
                    &None => {}
                }
            },
            Rel::AliasedRel { box ref input, .. } => self.plan_rel(input, builder)?,
            Rel::Dual { .. } => {},
            Rel::Update { ref table, box ref set_stmts, ref selection, ref tt } => {
                match set_stmts {
                    &Rex::RexExprList(ref list) => {
                        for e in list.iter() {
                            self.plan_rex(e, builder, tt);
                        }
                    },
                    _ => {}
                }
                match selection {
                    &Some(box ref s) => self.plan_rex(s, builder, tt)?,
                    &None => {}
                }
            },
            Rel::Delete { ref table, ref selection, ref tt } => {
                match selection {
                    &Some(box ref s) => self.plan_rex(s, builder, tt)?,
                    &None => {}
                }
            },
            Rel::Insert { ref table, box ref columns, box ref values, .. } => {
                match (columns, values) {
                    ( & Rex::RexExprList( ref c_list), & Rex::RexExprList( ref v_list)) => {
                        let mut it = c_list.iter().zip(v_list.iter());
                        while let Some((ref column_expr, ref value_expr)) = it.next() {
                            match *column_expr {
                                &Rex::Identifier { ref el, .. } => {

                                    let enc_plan = EncryptionPlan {
                                        data_type: el.data_type.clone(),
                                        encryption: el.encryption.clone(),
                                        key: Some(el.key.clone())
                                    };

                                    match *value_expr {
                                        &Rex::Literal(i) => builder.push_literal(i.clone(), enc_plan),
                                        &Rex::BoundParam(i) => builder.push_param(i.clone(), enc_plan),
                                        _ => return Err(self.zero_error("1064", format!("Unsupported expression for INSERT value expression: {:?}", *value_expr)))
                                    }
                                },
                                _ => return Err(self.zero_error("1064", format!("Unsupported expression for INSERT column name: {:?}", *column_expr))),
                            }
                        }
                    },
                    _ => {}
                }
            },
        }
        Ok(())
    }

    fn zero_error(&self, code: &'static str, msg: String) -> Box<ZeroError> {
        ZeroError::EncryptionError {
            message: msg,
            code: code.into()
        }.into()
    }

    fn plan_rex(&self, rex: &Rex, builder: &mut PhysicalPlanBuilder, tt: &TupleType) -> Result<(), Box<ZeroError>>  {
        match self.get_encryption_scheme(rex, builder, &mut None) {
            Ok(_) => Ok(()),
            Err(e) => Err(e)
        }
    }

    fn get_encryption_scheme(&self, rex: &Rex, builder: &mut PhysicalPlanBuilder, potentials: &mut Option<PotentialsBuilder>) -> Result<EncScheme, Box<ZeroError>> {
        match *rex {
            Rex::Identifier{ref el, ..} => match el.encryption {
                EncryptionType::NA => Ok(EncScheme::Unencrypted),
                _ => Ok(EncScheme::Encrypted(
                    el.encryption.clone(),
                    el.data_type.clone(),
                    el.key.clone()
                ))
            },
            Rex::Literal(ref i) => {
                if let Some(ref mut p) = *potentials {
                    p.put_literal(i);
                }

                // set a default
                let enc_plan = EncryptionPlan {
                    data_type: NativeType::UNKNOWN,
                    encryption: EncryptionType::NA,
                    key: None
                };
                builder.push_literal(i.clone(), enc_plan);

                Ok(EncScheme::Potential)
            },
            Rex::BoundParam(ref i) => {
                if let Some(ref mut p) = *potentials {
                    p.put_param(i);
                }

                // set a default
                let enc_plan = EncryptionPlan {
                    data_type: NativeType::UNKNOWN,
                    encryption: EncryptionType::NA,
                    key: None
                };
                builder.push_param(i.clone(), enc_plan);

                Ok(EncScheme::Potential)
            },
            Rex::RexNested(box ref expr) => self.get_encryption_scheme(expr, builder, potentials),
            Rex::RexExprList(ref list) => {
                for e in list {
                    self.get_encryption_scheme(e, builder, potentials)?;
                }
                Ok(EncScheme::Inconsequential)
            },
            Rex::BinaryExpr{box ref left, ref op, box ref right} => {

                let mut potentials_builder = Some(PotentialsBuilder::new());
                let l = self.get_encryption_scheme(left, builder, &mut potentials_builder)?;
                let r = self.get_encryption_scheme(right, builder, &mut potentials_builder)?;

                match *op {
                    Operator::AND | Operator::OR => Ok(EncScheme::Inconsequential),
                    Operator::EQ | Operator::NEQ => {
                        match (l, r) {
                            (EncScheme::Encrypted (ref le, ref ldt, ref lk ), EncScheme::Encrypted ( ref re, ref rdt, ref rk )) => {
                                if !(le == re && ldt == rdt && lk == rk) {
                                    Err(self.zero_error(
                                        "1064",
                                        format!("Unsupported operation between columns of differing encryption and type, expr: {:?}", *rex)
                                    ))
                                } else {
                                    Ok(EncScheme::Inconsequential)
                                }
                            },
                            (EncScheme::Unencrypted, EncScheme::Unencrypted) => Ok(EncScheme::Inconsequential),
                            (EncScheme::Unencrypted, EncScheme::Encrypted(..)) | (EncScheme::Encrypted(..), EncScheme::Unencrypted) => {
                                Err(self.zero_error(
                                    "1064",
                                    format!("Unsupported operation between columns of differing encryption and type, expr: {:?}", *rex)
                                ))
                            },
                            (EncScheme::Unencrypted, _) | (_, EncScheme::Unencrypted) => Ok(EncScheme::Inconsequential), // OK
                            (EncScheme::Encrypted(ref e, ref dt, ref k), EncScheme::Potential) | (EncScheme::Potential, EncScheme::Encrypted(ref e, ref dt, ref k)) => {

                                let ps = potentials_builder.unwrap().build();
                                for p in ps.params {
                                    let enc_plan = EncryptionPlan {
                                        data_type: dt.clone(),
                                        encryption: e.clone(),
                                        key: Some(k.clone())
                                    };

                                    builder.push_param(p, enc_plan);
                                }

                                for p in ps.literals {
                                    let enc_plan = EncryptionPlan {
                                        data_type: dt.clone(),
                                        encryption: e.clone(),
                                        key: Some(k.clone())
                                    };

                                    builder.push_literal(p, enc_plan);
                                }

                                Ok(EncScheme::Inconsequential)
                            },
                            _ => {
                                Err(self.zero_error(
                                    "1064",
                                    format!("Unsupported expr: {:?}", *rex)
                                ))
                            }
                        }
                    },
                    _ => {
                        match (l, r) {
                            (EncScheme::Encrypted(..), _) | (_, EncScheme::Encrypted(..)) => {
                                Err(self.zero_error(
                                    "1064",
                                    format!("Unsupported operator with encrypted column: {:?}", *op)
                                ))
                            },
                            _ => Ok(EncScheme::UnencryptedOperation)
                        }

                    }

                }
            },
            _ => Err(self.zero_error(
                "1064",
                format!("Unsupported expr: {:?}", *rex)
            ))
        }

    }

}

enum EncScheme {
    Encrypted(EncryptionType, NativeType, [u8; 32]),
    Unencrypted,
    Potential,
    Inconsequential,
    UnencryptedOperation

}

struct Potentials {
    pub params: Vec<usize>,
    pub literals: Vec<usize>
}

struct PotentialsBuilder {
    params: Vec<usize>,
    literals: Vec<usize>
}

impl PotentialsBuilder {
    pub fn new() -> Self {
        PotentialsBuilder{
            params: Vec::new(),
            literals: Vec::new()
        }
    }

    pub fn put_literal(&mut self, index: &usize) {
        self.literals.push(index.clone())
    }

    pub fn put_param(&mut self, index: &usize) {
        self.params.push(index.clone())
    }

    pub fn build(self) -> Potentials {
        Potentials {
            params: self.params,
            literals: self.literals
        }
    }
}


#[cfg(test)]
mod tests {

    use super::*;
    use config;
    use error::ZeroError;
    use std::collections::HashMap;
    use query::dialects::ansisql::*;
    use query::dialects::mysqlsql::*;
    use query::{Tokenizer, Parser, SQLWriter, Writer, ASTNode, LiteralToken};
    use query::planner::{Planner, RelVisitor, Rel, SchemaProvider, TableMeta, ColumnMeta};
    use encrypt::{EncryptionType, NativeType};
    use std::rc::Rc;
    use std::error::Error;
    use super::super::writers::*;

    #[test]
    fn test_physical_plan() {
        let sql = String::from("SELECT id FROM users WHERE id = 1 AND first_name = 'Janice'");
        let res = parse_and_plan(sql).unwrap();
        let literals = res.0;
        let parsed = res.1;
        let plan = res.2;

        let planner = PhysicalPlanner{};
        let pplan = planner.plan(plan, parsed);

        match pplan {
            PhysicalPlan::Plan(p) => {
                assert_eq!(2, p.literals.len());
                assert_eq!(0, p.params.len());
                assert_eq!(1, p.projection.len());

                let lit = p.literals.get(&(0 as usize)).unwrap();
                assert_eq!(NativeType::UNKNOWN, lit.data_type);
                assert_eq!(EncryptionType::NA, lit.encryption);
                assert_eq!(None, lit.key);

                let lit = p.literals.get(&(1 as usize)).unwrap();
                assert_eq!(NativeType::Varchar(50), lit.data_type);
                assert_eq!(EncryptionType::AES, lit.encryption);
                assert_eq!(true, lit.key.is_some());
            },
            _ => panic!("TEST FAIL")
        }
    }

    #[test]
    fn test_physical_plan_complex() {
        let sql = String::from("SELECT id, 1, first_name
            FROM users WHERE id = 1 AND first_name = ((('Janice'))) OR id = (1 + 1)");
        let res = parse_and_plan(sql).unwrap();
        let literals = res.0;
        let parsed = res.1;
        let plan = res.2;

        let planner = PhysicalPlanner{};
        let pplan = planner.plan(plan, parsed);

        match pplan {
            PhysicalPlan::Plan(p) => {
                assert_eq!(5, p.literals.len());
                assert_eq!(0, p.params.len());
                assert_eq!(3, p.projection.len());

//                let lit = p.literals.get(&(0 as usize)).unwrap();
//                assert_eq!(NativeType::UNKNOWN, lit.data_type);
//                assert_eq!(EncryptionType::NA, lit.encryption);
//                assert_eq!(None, lit.key);
//
//                let lit = p.literals.get(&(1 as usize)).unwrap();
//                assert_eq!(NativeType::Varchar(50), lit.data_type);
//                assert_eq!(EncryptionType::AES, lit.encryption);
//                assert_eq!(true, lit.key.is_some());
            },
            _ => panic!("TEST FAIL")
        }
    }

    fn parse_and_plan(sql: String) -> Result<(Vec<LiteralToken>, ASTNode, Rel), Box<ZeroError>> {
        let provider = DummyProvider{};

        let ansi = AnsiSQLDialect::new();
        let dialect = MySQLDialect::new(&ansi);

        let tokens = sql.tokenize(&dialect)?;
        let parsed = tokens.parse()?;

        let s = String::from("zero");
        let default_schema = Some(&s);
        let planner = Planner::new(default_schema, Rc::new(provider));
        let plan = planner.sql_to_rel(&parsed)?.unwrap();
        Ok((tokens.literals, parsed, plan))

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
                                        encryption: EncryptionType::AES,
                                        key: [0u8; 32]},
                            ColumnMeta {name: String::from("last_name"), native_type: NativeType::Varchar(50),
                                        encryption: EncryptionType::AES,
                                        key: [0u8; 32]},
                            ColumnMeta {name: String::from("ssn"), native_type: NativeType::Varchar(50),
                                        encryption: EncryptionType::AES,
                                        key: [0u8; 32]},
                            ColumnMeta {name: String::from("age"), native_type: NativeType::U64,
                                        encryption: EncryptionType::AES,
                                        key: [0u8; 32]},
                            ColumnMeta {name: String::from("sex"), native_type: NativeType::Varchar(50),
                                        encryption: EncryptionType::AES,
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
                                        encryption: EncryptionType::AES,
                                        key: [0u8; 32]},
                            ColumnMeta {name: String::from("amount"), native_type: NativeType::F64,
                                        encryption: EncryptionType::AES,
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

