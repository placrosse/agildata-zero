use super::ASTNode;

enum Rex {
    Identifier,
    Literal,
    BinaryExpr,
    RelationalExpr(Rel)
}

enum Rel {
    Projection,
    Selection,
    TableScan
}

fn sql_to_rel(sql: &ASTNode) -> Result<Rel, String> {

    match sql {
        &ASTNode::SQLSelect { .. } => Ok(Rel::Projection),
        //ASTNode::SQLInsert => {},
        _ => Err(String::from("oops)"))
    }

}