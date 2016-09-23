use query::planner::{Rel, Rex, TupleType, Element, HasTupleType};
use encrypt::{NativeType, EncryptionType};
use query::{Token, ASTNode, LiteralToken};
use error::ZeroError;

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
    key: [u8; 32]
//    value_type: ValueType
}

#[derive(Debug, PartialEq)]
pub struct PPlan {
    literals: Vec<EncryptionPlan>,
    params: Vec<EncryptionPlan>,
    result: Vec<EncryptionPlan>,
    ast: ASTNode
}

pub enum PhysicalPlan {
    Plan(PPlan),
    Passthrough,
    Error(Box<ZeroError>)
}

pub struct PhysicalPlanBuilder {
    literals: Vec<EncryptionPlan>,
    params: Vec<EncryptionPlan>,
    result: Vec<EncryptionPlan>
}

impl PhysicalPlanBuilder {

    fn new() -> Self {
        PhysicalPlanBuilder {
            literals: Vec::new(),
            params: Vec::new(),
            result: Vec::new()
        }
    }

    // build consumes self
    fn build(self, ast: ASTNode) -> PhysicalPlan {
        PhysicalPlan::Plan(
            PPlan {
                literals: self.literals,
                params: self.params,
                result: self.result,
                ast: ast
            }
        )
    }

    fn push_literal(&mut self, e: EncryptionPlan) {
        self.literals.push(e);
    }

    fn push_param(&mut self, e: EncryptionPlan) {
        self.params.push(e);
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
                                        key: el.key.clone()
                                    };

                                    match *value_expr {
                                        &Rex::Literal(i) => builder.push_literal(enc_plan),
                                        &Rex::BoundParam(i) => builder.push_param(enc_plan),
                                        _ => return self.zero_error("1064", format!("Unsupported expression for INSERT value expression: {:?}", *value_expr))
                                    }
                                },
                                _ => return self.zero_error("1064", format!("Unsupported expression for INSERT column name: {:?}", *column_expr)),
                            }
                        }
                    },
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn zero_error(&self, code: &'static str, msg: String) -> Result<(), Box<ZeroError>> {
        Err(ZeroError::EncryptionError {
            message: msg,
            code: code.into()
        }.into())
    }

//                                    if let Rex::Identifier{ref id, ref el} = c_list[index] {
//                                        if el.encryption != EncryptionType::NA {
//                                            self.encrypt_literal(i, el, None)?;
//                                        }
//
//                                    } else {
//                                        return Err(ZeroError::EncryptionError{
//                                            message: format!("Expected identifier at column list index {}, received {:?}", index, c_list[index]).into(),
//                                            code: "1064".into()
//                                        }.into())
//                                    }
//                                },
//                                // TODO swap this logic out with some evaluate()
//                                &Rex::RexUnary{ref operator, rex: box Rex::Literal(ref i)} => {
//                                    if let Rex::Identifier{ref id, ref el} = c_list[index] {
//                                        if el.encryption != EncryptionType::NA {
//                                            self.encrypt_literal(i, el, Some(operator))?;
//                                        }
//
//                                    } else {
//                                        return Err(ZeroError::EncryptionError{
//                                            message: format!("Expected identifier at column list index {}, received {:?}", index, c_list[index]).into(),
//                                            code: "1064".into()
//                                        }.into())
//                                    }
//                                },
//                                _ => {}
//                            }
//                        }
//                    },
//                    _ => return Err(ZeroError::EncryptionError{
//                        message: format!("Unsupported INSERT syntax").into(),
//                        code: "1064".into()
//                    }.into())
//                }
//            }
//            //_ => return Err(format!("Unsupported rel {:?}", rel))
//        }
//
//        Ok(())
//    }

    fn plan_rex(&self, rel: &Rex, builder: &mut PhysicalPlanBuilder, tt: &TupleType) -> Result<(), Box<ZeroError>>  {
        panic!("NOT IMPLEMENTED")
    }
}

