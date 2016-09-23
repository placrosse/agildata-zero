use query::planner::Rel;
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
    value_type: ValueType
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
        panic!("NOT IMPLEMENTED")
    }

    fn plan_rex(&self, rel: &Rel, builder: &mut PhysicalPlanBuilder) -> Result<(), Box<ZeroError>>  {
        panic!("NOT IMPLEMENTED")
    }
}

