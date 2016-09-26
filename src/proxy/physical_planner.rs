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

#[derive(Debug, PartialEq, Clone)]
pub struct EncryptionPlan {
    pub data_type: NativeType,
    pub encryption: EncryptionType,
    pub key: Option<[u8; 32]>
//    value_type: ValueType
}

#[derive(Debug, PartialEq)]
pub struct PPlan {
    pub literals: HashMap<usize, EncryptionPlan>,
    pub params: HashMap<usize, EncryptionPlan>,
    pub projection: Vec<EncryptionPlan>,
    pub ast: ASTNode
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
    pub fn plan(&self, logical: Rel, ast: ASTNode, literals: &Vec<LiteralToken>) -> PhysicalPlan {
        let mut builder = PhysicalPlanBuilder::new();

        match self.plan_rel(&logical, &mut builder, literals) {
            Ok(()) => builder.build(ast),
            Err(e) => PhysicalPlan::Error(e)
        }

    }

    fn plan_rel(&self, rel: &Rel, builder: &mut PhysicalPlanBuilder, literals: &Vec<LiteralToken>) -> Result<(), Box<ZeroError>> {
        match *rel {
            Rel::Projection { box ref project, box ref input, ref tt } => {
                self.plan_rex(project, builder, tt, literals)?;
                self.plan_rel(input, builder, literals)?;

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
                self.plan_rex(expr, builder, input.tt(), literals)?;
                self.plan_rel(input, builder, literals)?;
            },
            Rel::TableScan { .. } => {},
            Rel::Join { box ref left, box ref right, ref on_expr, ref tt, .. } => {
                self.plan_rel(left, builder, literals)?;
                self.plan_rel(right, builder, literals)?;
                match on_expr {
                    &Some(box ref o) => self.plan_rex(o, builder, tt, literals)?,
                    &None => {}
                }
            },
            Rel::AliasedRel { box ref input, .. } => self.plan_rel(input, builder, literals)?,
            Rel::Dual { .. } => {},
            Rel::Update { ref table, box ref set_stmts, ref selection, ref tt } => {
                match set_stmts {
                    &Rex::RexExprList(ref list) => {
                        for e in list.iter() {
                            self.plan_rex(e, builder, tt, literals);
                        }
                    },
                    _ => {}
                }
                match selection {
                    &Some(box ref s) => self.plan_rex(s, builder, tt, literals)?,
                    &None => {}
                }
            },
            Rel::Delete { ref table, ref selection, ref tt } => {
                match selection {
                    &Some(box ref s) => self.plan_rex(s, builder, tt, literals)?,
                    &None => {}
                }
            },
            Rel::Insert { ref table, box ref columns, box ref values, .. } => {
                match (columns, values) {
                    ( & Rex::RexExprList( ref c_list), & Rex::RexExprList( ref v_list)) => {
                        let mut it = c_list.iter().zip(v_list.iter());

                        // create encryption plans for insert values reconciled to column list
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
            Rel::MySQLCreateTable => {} // TODO eventually build plans for Defaults, etc
        }
        Ok(())
    }

    fn zero_error(&self, code: &'static str, msg: String) -> Box<ZeroError> {
        ZeroError::EncryptionError {
            message: msg,
            code: code.into()
        }.into()
    }

    fn plan_rex(&self, rex: &Rex, builder: &mut PhysicalPlanBuilder, tt: &TupleType, literals: &Vec<LiteralToken>) -> Result<(), Box<ZeroError>>  {
        match self.get_encryption_scheme(rex, builder, &mut None, literals) {
            Ok(_) => Ok(()),
            Err(e) => Err(e)
        }
    }

    /// Visit rex expressions, attempt to reconcile to supported operations and encryption plans for literals and bound params
    fn get_encryption_scheme(&self, rex: &Rex, builder: &mut PhysicalPlanBuilder, potentials: &mut Option<PotentialsBuilder>, literals: &Vec<LiteralToken>) -> Result<EncScheme, Box<ZeroError>> {
        match *rex {
            // Encryption plan of the column identifier
            Rex::Identifier{ref el, ..} => match el.encryption {
                EncryptionType::NA => Ok(EncScheme::Unencrypted),
                _ => Ok(EncScheme::Encrypted(
                    el.encryption.clone(),
                    el.data_type.clone(),
                    el.key.clone()
                ))
            },
            // Potential of the literal value
            Rex::Literal(ref i) => {
                // Add self to the list of potentially encryptable values
                if let Some(ref mut p) = *potentials {
                    p.put_literal(i);
                }

                // Add a default unencrypted plan
                // set a default
                let enc_plan = EncryptionPlan {
                    data_type: NativeType::UNKNOWN,
                    encryption: EncryptionType::NA,
                    key: None
                };
                builder.push_literal(i.clone(), enc_plan);

                Ok(EncScheme::Potential)
            },
            // Potential of the bound parameter value
            Rex::BoundParam(ref i) => {
                // Add self to the list of potentially encryptable params
                if let Some(ref mut p) = *potentials {
                    p.put_param(i);
                }

                // Add a default unencrypted plan
                let enc_plan = EncryptionPlan {
                    data_type: NativeType::UNKNOWN,
                    encryption: EncryptionType::NA,
                    key: None
                };
                builder.push_param(i.clone(), enc_plan);

                Ok(EncScheme::Potential)
            },
            // Delegate to the scheme of the enclosed expression
            Rex::RexNested(box ref expr) => self.get_encryption_scheme(expr, builder, potentials, literals),
            Rex::RexExprList(ref list) => {
                for e in list {
                    self.get_encryption_scheme(e, builder, potentials, literals)?;
                }
                Ok(EncScheme::Inconsequential)
            },
            // Evaluate binary
            Rex::BinaryExpr{box ref left, ref op, box ref right} => {

                // Build up a list of potential literals and params on both sides
                let mut potentials_builder = Some(PotentialsBuilder::new());
                let l = self.get_encryption_scheme(left, builder, &mut potentials_builder, literals)?;
                let r = self.get_encryption_scheme(right, builder, &mut potentials_builder, literals)?;

                match *op {
                    // If AND||OR, the resolved uncryption scheme is unimportant
                    Operator::AND | Operator::OR => Ok(EncScheme::Inconsequential),
                    // Equality comparisons
                    Operator::EQ | Operator::NEQ => {
                        match (l, r) {
                            // An eq between two encrypted columns...
                            (EncScheme::Encrypted (ref le, ref ldt, ref lk ), EncScheme::Encrypted ( ref re, ref rdt, ref rk )) => {
                                // If both do not share the same encryption, data type, and key, fail
                                if !(le == re && ldt == rdt && lk == rk) {
                                    Err(self.zero_error(
                                        "1064",
                                        format!("Unsupported operation between columns of differing encryption and type, expr: {}", rex.to_readable(literals))
                                    ))
                                } else {
                                    // The operation is legal
                                    Ok(EncScheme::Inconsequential)
                                }
                            },
                            // An eq between two unencrypted columns, legal
                            (EncScheme::Unencrypted, EncScheme::Unencrypted) => Ok(EncScheme::Inconsequential),
                            // An eq between an unencrypted and encrypted column, illegal
                            (EncScheme::Unencrypted, EncScheme::Encrypted(..)) | (EncScheme::Encrypted(..), EncScheme::Unencrypted) => {
                                Err(self.zero_error(
                                    "1064",
                                    format!("Unsupported operation between encrypted and unencrypted columns: {}", rex.to_readable(literals))
                                ))
                            },
                            // Catch all eq between an unencrypted column and any other expression, legal, allow to delegate to dbms
                            (EncScheme::Unencrypted, _) | (_, EncScheme::Unencrypted) => Ok(EncScheme::Inconsequential), // OK
                            // EQ between an encrypted column and potentially encryptable expressions, e.g a = 1, a = (1), etc
                            (EncScheme::Encrypted(ref e, ref dt, ref k), EncScheme::Potential) | (EncScheme::Potential, EncScheme::Encrypted(ref e, ref dt, ref k)) => {

                                // Assign encryption plans for all literals and params contained within
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
                            // Anything else, default to unsupported
                            _ => {
                                Err(self.zero_error(
                                    "1064",
                                    format!("Unsupported expr: {}", rex.to_readable(literals))
                                ))
                            }
                        }
                    },
                    // Non eq comparisons and arithmetic
                    _ => {
                        match (l, r) {
                            // If either side contains an encrypted column, fail
                            (EncScheme::Encrypted(..), _) | (_, EncScheme::Encrypted(..)) => {
                                Err(self.zero_error(
                                    "1064",
                                    format!("Unsupported operation on encrypted column: {}", rex.to_readable(literals))
                                ))
                            },
                            // Otherwise delegate to mysql
                            _ => Ok(EncScheme::UnencryptedOperation)
                        }

                    }

                }
            },
            _ => Err(self.zero_error(
                "1064",
                format!("Unsupported expr: {}", rex.to_readable(literals))
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
        let pplan = planner.plan(plan, parsed, &literals);

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
        let pplan = planner.plan(plan, parsed, &literals);

        match pplan {
            PhysicalPlan::Plan(p) => {
                assert_eq!(5, p.literals.len());
                assert_eq!(0, p.params.len());
                assert_eq!(3, p.projection.len());

                let lit = p.literals.get(&(0 as usize)).unwrap();
                assert_eq!(NativeType::UNKNOWN, lit.data_type);
                assert_eq!(EncryptionType::NA, lit.encryption);
                assert_eq!(None, lit.key);

                let lit = p.literals.get(&(1 as usize)).unwrap();
                assert_eq!(NativeType::UNKNOWN, lit.data_type);
                assert_eq!(EncryptionType::NA, lit.encryption);
                assert_eq!(None, lit.key);

                let lit = p.literals.get(&(2 as usize)).unwrap();
                assert_eq!(NativeType::Varchar(50), lit.data_type);
                assert_eq!(EncryptionType::AES, lit.encryption);
                assert_eq!(true, lit.key.is_some());

                let lit = p.literals.get(&(3 as usize)).unwrap();
                assert_eq!(NativeType::UNKNOWN, lit.data_type);
                assert_eq!(EncryptionType::NA, lit.encryption);
                assert_eq!(None, lit.key);

                let lit = p.literals.get(&(4 as usize)).unwrap();
                assert_eq!(NativeType::UNKNOWN, lit.data_type);
                assert_eq!(EncryptionType::NA, lit.encryption);
                assert_eq!(None, lit.key);
            },
            _ => panic!("TEST FAIL")
        }
    }

    #[test]
    fn test_physical_plan_illegal_operations() {
        // Eq between encrypted = unencrypted
        let mut sql = String::from("SELECT id FROM users WHERE id = first_name");
        let mut res = parse_and_plan(sql).unwrap();
        let mut literals = res.0;
        let mut parsed = res.1;
        let mut plan = res.2;

        let planner = PhysicalPlanner{};
        let mut pplan = planner.plan(plan, parsed, &literals);

        match pplan {
            PhysicalPlan::Error(box ZeroError::EncryptionError{message, ..}) => {
                assert_eq!(
                String::from("Unsupported operation between encrypted and unencrypted columns: id = first_name"),
                message)
            },
            _ => panic!("TEST FAIL")
        }

        sql = String::from("SELECT id FROM users WHERE age = age + 10");
        res = parse_and_plan(sql).unwrap();
        literals = res.0;
        parsed = res.1;
        plan = res.2;

        pplan = planner.plan(plan, parsed, &literals);

        match pplan {
            PhysicalPlan::Error(box ZeroError::EncryptionError{message, ..}) => {
                assert_eq!(
                String::from("Unsupported operation on encrypted column: age + 10"),
                message)
            },
            _ => panic!("TEST FAIL")
        }

        sql = String::from("SELECT id FROM users WHERE age > 10");
        res = parse_and_plan(sql).unwrap();
        literals = res.0;
        parsed = res.1;
        plan = res.2;

        pplan = planner.plan(plan, parsed, &literals);

        match pplan {
            PhysicalPlan::Error(box ZeroError::EncryptionError{message, ..}) => {
                assert_eq!(
                String::from("Unsupported operation on encrypted column: age > 10"),
                message)
            },
            _ => panic!("TEST FAIL")
        }

        sql = String::from("SELECT id FROM users WHERE age = first_name");
        res = parse_and_plan(sql).unwrap();
        literals = res.0;
        parsed = res.1;
        plan = res.2;

        pplan = planner.plan(plan, parsed, &literals);

        match pplan {
            PhysicalPlan::Error(box ZeroError::EncryptionError{message, ..}) => {
                assert_eq!(
                String::from("Unsupported operation between columns of differing encryption and type, expr: age = first_name"),
                message)
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
        let plan = planner.sql_to_rel(&parsed)?;
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

