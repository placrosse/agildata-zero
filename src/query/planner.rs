use super::ASTNode;

enum Rex {
    Identifier,
    Literal,
    BinaryExpr,
    RelationalExpr(Rel),
    RexExprList(Vec<Rex>)
}

enum Rel {
    Projection { project: Box<Rex>, input: Box<Rel> },
    Selection,
    TableScan,
    Dual
}

fn sql_to_rex(sql: &ASTNode) -> Result<Rex, String> {
    match sql {
        &ASTNode::SQLExprList(ref v) => Ok(Rex::RexExprList(v.iter()
            .map(|x| sql_to_rex(&x) )
            .collect()?)),
        _ => Err(String::from("oops"))
    }
}

fn sql_to_rel(sql: &ASTNode) -> Result<Rel, String> {

    match sql {
        &ASTNode::SQLSelect { box ref expr_list, ref relation, ref selection, ref order} => {
            /*
                SQLSelect{
                    expr_list: Box<ASTNode>,
                    relation: Option<Box<ASTNode>>,
                    selection: Option<Box<ASTNode>>,
                    order: Option<Box<ASTNode>>
                },
            */
            //expr_list.

            let mut input = match relation {
                &Some(box ref r) => sql_to_rel(r)?,
                &None => Rel::Dual
            };

            Ok(Rel::Projection {
                project: Box::new(sql_to_rex(expr_list)?),
                input: Box::new(input) })
        },
//        &ASTNode::SQLIdentifier { ref id, ref parts } => {
//
//        }
        //ASTNode::SQLInsert => {},
        _ => Err(String::from("oops)"))
    }

}