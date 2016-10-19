// Copyright 2016 AgilData
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http:// www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::super::ASTNode;
use super::super::ASTNode::*;
use super::super::Operator::*;
use super::super::JoinType::*;
use super::super::UnionType::*;
use super::super::{Tokenizer, Parser, SQLWriter, Writer, InsertMode};
use super::super::dialects::ansisql::*;
use super::test_helper::*;

#[test]
fn select_wildcard() {
    let dialect = AnsiSQLDialect::new();
    let sql = String::from("SELECT * FROM foo");
    let tokens = sql.tokenize(&dialect).unwrap();
    let parsed = tokens.parse().unwrap();

    assert_eq!(
        SQLSelect {
            expr_list: Box::new(SQLExprList(vec![SQLIdentifier{id: String::from("*"), parts: vec![String::from("*")]}])),
            relation: Some(Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]})),
            selection: None,
            order: None,
            limit: None,
            for_update: false
        },
        parsed
    );

    println!("{:#?}", parsed);

    let ansi_writer = AnsiSQLWriter{literal_tokens: &tokens.literals};
    let writer = SQLWriter::new(vec![&ansi_writer]);
    let rewritten = writer.write(&parsed).unwrap();
    assert_eq!(format_sql(&rewritten), format_sql(&sql));

    println!("Rewritten: {:?}", rewritten);

}

#[test]
fn select_1() {
    let dialect = AnsiSQLDialect::new();
    let sql = String::from("SELECT 1");
    let tokens = sql.tokenize(&dialect).unwrap();
    let parsed = tokens.parse().unwrap();

    assert_eq!(
        SQLSelect {
            expr_list: Box::new(SQLExprList(vec![SQLLiteral(0)])),
            relation: None,
            selection: None,
            order: None,
            limit: None,
            for_update: false
        },
        parsed
    );

    println!("{:#?}", parsed);

    let ansi_writer = AnsiSQLWriter{literal_tokens: &tokens.literals};
    let writer = SQLWriter::new(vec![&ansi_writer]);
    let rewritten = writer.write(&parsed).unwrap();
    assert_eq!(format_sql(&rewritten), format_sql(&sql));

    println!("Rewritten: {:?}", rewritten);

}

#[test]
fn select_with_nulls() {
    let dialect = AnsiSQLDialect::new();
    let sql = String::from("SELECT a, NULL FROM foo WHERE b = null");
    let tokens = sql.tokenize(&dialect).unwrap();
    let parsed = tokens.parse().unwrap();

    assert_eq!(
        SQLSelect {
            expr_list: Box::new(SQLExprList(vec![
                SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]},
                SQLLiteral(0)
            ])),
            relation: Some(Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]})),
            selection: Some(Box::new(SQLBinary {
                left: Box::new(SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]}),
                op: EQ,
                right: Box::new(SQLLiteral(1))
            })),
            order: None,
            limit: None,
            for_update: false
        },
        parsed
    );

    println!("{:#?}", parsed);

    let ansi_writer = AnsiSQLWriter{literal_tokens: &tokens.literals};
    let writer = SQLWriter::new(vec![&ansi_writer]);
    let rewritten = writer.write(&parsed).unwrap();
    assert_eq!(format_sql(&rewritten), format_sql(&sql));

    println!("Rewritten: {:?}", rewritten);

}

#[test]
fn sqlparser() {
    let dialect = AnsiSQLDialect::new();
    let sql = String::from("SELECT 1 + 1 + 1,
        a AS alias,
        (3 * (1 + 2)),
        -1 AS unary,
        (SELECT a, b, c FROM tTwo WHERE c = a) AS subselect
        FROM (SELECT a, b, c FROM tThree) AS l
        WHERE a > 10 AND b = true
        ORDER BY a DESC, (a + b) ASC, c");
    let tokens = sql.tokenize(&dialect).unwrap();
    let parsed = tokens.parse().unwrap();

    assert_eq!(
        SQLSelect {
            expr_list: Box::new(
                SQLExprList(vec![
                    SQLBinary {
                        left:  Box::new(SQLBinary{
                            left: Box::new(SQLLiteral(0)),
                            op: ADD,
                            right:  Box::new(SQLLiteral(1))
                        }),
                        op: ADD,
                        right:  Box::new(SQLLiteral(2))
                    },
                    SQLAlias{
                        expr:  Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
                        alias:  Box::new(SQLIdentifier{id: String::from("alias"), parts: vec![String::from("alias")]})
                    },
                    SQLNested(
                         Box::new(SQLBinary {
                            left:  Box::new(SQLLiteral(3)),
                            op: MULT,
                            right:  Box::new(SQLNested(
                                 Box::new(SQLBinary{
                                    left:  Box::new(SQLLiteral(4)),
                                    op: ADD,
                                    right:  Box::new(SQLLiteral(5))
                                })
                            ))
                        })
                    ),
                    SQLAlias{
                        expr:  Box::new(SQLUnary{
                            operator: SUB,
                            expr:  Box::new(SQLLiteral(6))
                        }),
                        alias:  Box::new(SQLIdentifier{id: String::from("unary"), parts: vec![String::from("unary")]})
                    },
                    SQLAlias {
                        expr:  Box::new(SQLNested(
                             Box::new(SQLSelect{
                                expr_list:  Box::new(SQLExprList(
                                    vec![
                                        SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]},
                                        SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]},
                                        SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}
                                    ]
                                )),
                                relation: Some( Box::new(SQLIdentifier{id: String::from("tTwo"), parts: vec![String::from("tTwo")]})),
                                selection: Some( Box::new(SQLBinary{
                                    left:  Box::new(SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}),
                                    op: EQ,
                                    right:  Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]})
                                })),
                                order: None,
                                limit: None,
                                for_update: false
                            })
                        )),
                        alias:  Box::new(SQLIdentifier{id: String::from("subselect"), parts: vec![String::from("subselect")]})
                    }
                    ]
                )
            ),
            relation: Some( Box::new(SQLAlias{
                expr:  Box::new(SQLNested(
                     Box::new(SQLSelect {
                        expr_list:  Box::new(SQLExprList(
                            vec![
                                SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]},
                                SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]},
                                SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}
                            ]
                        )),
                        relation: Some( Box::new(SQLIdentifier{id: String::from("tThree"), parts: vec![String::from("tThree")]})),
                        selection: None,
                        order: None,
                        limit: None,
                        for_update: false
                    })
                )),
                alias:  Box::new(SQLIdentifier{id: String::from("l"), parts: vec![String::from("l")]})
            })),
            selection: Some( Box::new(SQLBinary {
                left:  Box::new(SQLBinary{
                    left:  Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
                    op: GT,
                    right:  Box::new(SQLLiteral(7))
                }),
                op: AND,
                right:  Box::new(SQLBinary{
                    left:  Box::new(SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]}),
                    op: EQ,
                    right:  Box::new(SQLLiteral(8))
                })
            })),
            order: Some( Box::new(SQLExprList(
                vec![
                    SQLOrderBy{
                        expr:  Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
                        is_asc: false
                    },
                    SQLOrderBy{
                        expr:  Box::new(SQLNested(
                             Box::new(SQLBinary{
                                left:  Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
                                op: ADD,
                                right:  Box::new(SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]})
                            })
                        )),
                        is_asc: true
                    },
                    SQLOrderBy{
                        expr:  Box::new(SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}),
                        is_asc: true
                    },
                ]
            ))),
            limit: None,
            for_update: false
        },
        parsed
    );

    println!("{:#?}", parsed);
    let ansi_writer = AnsiSQLWriter{literal_tokens: &tokens.literals};
    let writer = SQLWriter::new(vec![&ansi_writer]);
    let rewritten = writer.write(&parsed).unwrap();
    assert_eq!(format_sql(&rewritten), format_sql(&sql));

    println!("Rewritten: {:?}", rewritten);

}

#[test]
fn sql_join() {

    let dialect = AnsiSQLDialect::new();
    let sql = String::from("SELECT l.a, r.b, l.c FROM tOne AS l
        JOIN (SELECT a, b, c FROM tTwo WHERE a > 0) AS r
        ON l.a = r.a
        WHERE l.b > r.b
        ORDER BY r.c DESC");

    let tokens = sql.tokenize(&dialect).unwrap();
    let parsed = tokens.parse().unwrap();

    assert_eq!(
        SQLSelect {
            expr_list: Box::new(SQLExprList(
                vec![
                    SQLIdentifier{id: String::from("l.a"), parts: vec![String::from("l"), String::from("a")]},
                    SQLIdentifier{id: String::from("r.b"), parts: vec![String::from("r"), String::from("b")]},
                    SQLIdentifier{id: String::from("l.c"), parts: vec![String::from("l"), String::from("c")]}
                ]
            )),
            relation: Some(Box::new(SQLJoin {
                left: Box::new(
                    SQLAlias {
                        expr: Box::new(SQLIdentifier{id: String::from("tOne"), parts: vec![String::from("tOne")]}),
                        alias: Box::new(SQLIdentifier{id: String::from("l"), parts: vec![String::from("l")]})
                    }
                ),
                join_type: INNER,
                right: Box::new(
                    SQLAlias {
                        expr: Box::new(SQLNested(
                            Box::new(SQLSelect{
                                expr_list: Box::new(SQLExprList(
                                    vec![
                                    SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]},
                                    SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]},
                                    SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}
                                    ]
                                )),
                                relation: Some(Box::new(SQLIdentifier{id: String::from("tTwo"), parts: vec![String::from("tTwo")]})),
                                selection: Some(Box::new(SQLBinary{
                                    left: Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
                                    op: GT,
                                    right: Box::new(SQLLiteral(0))
                                })),
                                order: None,
                                limit: None,
                                for_update: false
                            })
                        )),
                        alias: Box::new(SQLIdentifier{id: String::from("r"), parts: vec![String::from("r")]})
                    }
                ),
                on_expr: Some(Box::new(SQLBinary {
                    left: Box::new(SQLIdentifier{id: String::from("l.a"), parts: vec![String::from("l"), String::from("a")]}),
                    op: EQ,
                    right: Box::new(SQLIdentifier{id: String::from("r.a"), parts: vec![String::from("r"), String::from("a")]})
                }))
            })),
            selection: Some(Box::new(SQLBinary{
                left: Box::new(SQLIdentifier{id: String::from("l.b"), parts: vec![String::from("l"), String::from("b")]}),
                op: GT,
                right: Box::new(SQLIdentifier{id: String::from("r.b"), parts: vec![String::from("r"), String::from("b")]})
            })),
            order: Some(Box::new(SQLExprList(
                vec![
                    SQLOrderBy{
                        expr: Box::new(SQLIdentifier{id: String::from("r.c"), parts: vec![String::from("r"), String::from("c")]}),
                        is_asc: false
                    }
                ]
            ))),
            limit: None,
            for_update: false
        },
        parsed
    );

    println!("{:#?}", parsed);

    let ansi_writer = AnsiSQLWriter{literal_tokens: &tokens.literals};
    let writer = SQLWriter::new(vec![&ansi_writer]);
    let rewritten = writer.write(&parsed).unwrap();
    assert_eq!(format_sql(&rewritten), format_sql(&sql));

    println!("Rewritten: {:?}", rewritten);
}

#[test]
fn nasty() {

    let dialect = AnsiSQLDialect::new();
    let sql = String::from("((((SELECT a, b, c FROM tOne UNION (SELECT a, b, c FROM tTwo))))) UNION (((SELECT a, b, c FROM tThree) UNION ((SELECT a, b, c FROM tFour))))");
    let tokens = sql.tokenize(&dialect).unwrap();
    let parsed = tokens.parse().unwrap();

    assert_eq!(
        SQLUnion{
            left: Box::new(SQLNested(
                Box::new(SQLNested(
                    Box::new(SQLNested(
                        Box::new(SQLNested(
                            Box::new(SQLUnion{
                                left: Box::new(SQLSelect{
                                    expr_list: Box::new(SQLExprList(vec![
                                        SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]},
                                        SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]},
                                        SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}
                                    ])),
                                    relation: Some(Box::new(SQLIdentifier{id: String::from("tOne"), parts: vec![String::from("tOne")]})),
                                    selection: None,
                                    order: None,
                                    limit: None,
                                    for_update: false
                                }),
                                union_type: UNION,
                                right: Box::new(SQLNested(
                                    Box::new(SQLSelect{
                                        expr_list: Box::new(SQLExprList(vec![
                                            SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]},
                                            SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]},
                                            SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}
                                        ])),
                                        relation: Some(Box::new(SQLIdentifier{id: String::from("tTwo"), parts: vec![String::from("tTwo")]})),
                                        selection: None,
                                        order: None,
                                        limit: None,
                                        for_update: false
                                    })
                                ))
                            })
                        ))
                    ))
                ))
            )),
            union_type: UNION,
            right: Box::new(SQLNested(
                Box::new(SQLNested(
                    Box::new(SQLUnion{
                        left: Box::new(SQLNested(
                            Box::new(SQLSelect{
                                expr_list: Box::new(SQLExprList(vec![
                                    SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]},
                                    SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]},
                                    SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}
                                ])),
                                relation: Some(Box::new(SQLIdentifier{id: String::from("tThree"), parts: vec![String::from("tThree")]})),
                                selection: None,
                                order: None,
                                limit: None,
                                for_update: false
                            })
                        )),
                        union_type: UNION,
                        right: Box::new(SQLNested(
                            Box::new(SQLNested(
                                Box::new(SQLSelect{
                                    expr_list: Box::new(SQLExprList(vec![
                                        SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]},
                                        SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]},
                                        SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}
                                    ])),
                                    relation: Some(Box::new(SQLIdentifier{id: String::from("tFour"), parts: vec![String::from("tFour")]})),
                                    selection: None,
                                    order: None,
                                    limit: None,
                                    for_update: false
                                })
                            ))
                        ))
                    })
                ))
            ))
        },
        parsed
    );

    println!("{:#?}", parsed);

    let ansi_writer = AnsiSQLWriter{literal_tokens: &tokens.literals};
    let writer = SQLWriter::new(vec![&ansi_writer]);
    let rewritten = writer.write(&parsed).unwrap();
    assert_eq!(format_sql(&rewritten), format_sql(&sql));

    println!("Rewritten: {:?}", rewritten);
}

fn id_boxed(name: &'static str) -> Box<ASTNode> {
    Box::new(id(name))
}
fn id(name: &'static str) -> ASTNode {
    SQLIdentifier{id: String::from(name), parts: vec![String::from(name)]}
}

#[test]
fn select_comparisons() {
    let dialect = AnsiSQLDialect::new();
    let sql = String::from("SELECT * FROM foo WHERE a > 1 AND b < 2 AND c != 3 AND d != 4 AND e >= 5 AND f <= 5");
    let tokens = sql.tokenize(&dialect).unwrap();
    let parsed = tokens.parse().unwrap();

    assert_eq!(
        SQLSelect {
            expr_list: Box::new(SQLExprList(vec![id("*")])),
            relation: Some(id_boxed("foo")),
            selection: Some(Box::new(SQLBinary {
                left: Box::new(SQLBinary {
                    left: Box::new(SQLBinary {
                        left: Box::new(SQLBinary {
                            left: Box::new(SQLBinary {
                                left: Box::new(SQLBinary {
                                    left: id_boxed("a"),
                                    op: GT,
                                    right: Box::new(SQLLiteral(0))
                                }),
                                op: AND,
                                right: Box::new(SQLBinary {
                                    left: id_boxed("b"),
                                    op: LT,
                                    right: Box::new(SQLLiteral(1))
                                })
                            }),
                            op: AND,
                            right: Box::new(SQLBinary {
                                left: id_boxed("c"),
                                op: NEQ,
                                right: Box::new(SQLLiteral(2))
                            })
                        }),
                        op: AND,
                        right: Box::new(SQLBinary {
                            left: id_boxed("d"),
                            op: NEQ,
                            right: Box::new(SQLLiteral(3))
                        })
                    }),
                    op: AND,
                    right: Box::new(SQLBinary {
                        left: id_boxed("e"),
                        op: GTEQ,
                        right: Box::new(SQLLiteral(4))
                    })
                }),
                op: AND,
                right: Box::new(SQLBinary {
                    left: id_boxed("f"),
                    op: LTEQ,
                    right: Box::new(SQLLiteral(5))
                })
            })),
            order: None,
            limit: None,
            for_update: false
        },
        parsed
    );

    println!("{:#?}", parsed);

    let ansi_writer = AnsiSQLWriter{literal_tokens: &tokens.literals};
    let writer = SQLWriter::new(vec![&ansi_writer]);
    let rewritten = writer.write(&parsed).unwrap();
    assert_eq!(format_sql(&rewritten), format_sql(&sql));

    println!("Rewritten: {:?}", rewritten);

}
#[test]
fn insert() {

    let dialect = AnsiSQLDialect::new();
    let sql = String::from("INSERT INTO foo (a, b, c) VALUES(1, 20.45, ?)");
    let tokens = sql.tokenize(&dialect).unwrap();
    let parsed = tokens.parse().unwrap();

    assert_eq!(
        SQLInsert{
            table: Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]}),
            insert_mode: InsertMode::INSERT,
            column_list: Box::new(SQLExprList(
                vec![
                    SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]},
                    SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]},
                    SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}
                ]
            )),
            values_list: vec!(SQLExprList(
                vec![
                    SQLLiteral(0),
                    SQLLiteral(1),
                    SQLBoundParam(0)
                ]
            ))
        },
        parsed
    );

    println!("{:#?}", parsed);

    let ansi_writer = AnsiSQLWriter{literal_tokens: &tokens.literals};
    let writer = SQLWriter::new(vec![&ansi_writer]);
    let rewritten = writer.write(&parsed).unwrap();
    assert_eq!(format_sql(&rewritten), format_sql(&sql));

    println!("Rewritten: {:?}", rewritten);
}

#[test]
fn insert_implicit_column_list() {

    let dialect = AnsiSQLDialect::new();
    let sql = String::from("INSERT INTO foo VALUES(1, 20.45, ?)");
    let tokens = sql.tokenize(&dialect).unwrap();
    let parsed = tokens.parse().unwrap();

    assert_eq!(
        SQLInsert{
            table: Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]}),
            insert_mode: InsertMode::INSERT,
            column_list: Box::new(SQLExprList(
                vec![
                ]
            )),
            values_list: vec!(SQLExprList(
                vec![
                    SQLLiteral(0),
                    SQLLiteral(1),
                    SQLBoundParam(0)
                ]
            ))
        },
        parsed
    );

    println!("{:#?}", parsed);

    let ansi_writer = AnsiSQLWriter{literal_tokens: &tokens.literals};
    let writer = SQLWriter::new(vec![&ansi_writer]);
    let rewritten = writer.write(&parsed).unwrap();
    assert_eq!(format_sql(&rewritten), format_sql(&sql));

    println!("Rewritten: {:?}", rewritten);
}

#[test]
fn insert_with_comments() {

    let dialect = AnsiSQLDialect::new();
    let sql = String::from("/* comment one */ INSERT INTO /* comment two */ foo (a, b, c) VALUES(1, 20.45, ?)");
    let tokens = sql.tokenize(&dialect).unwrap();
    let parsed = tokens.parse().unwrap();

    assert_eq!(
        SQLInsert{
            table: Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]}),
            insert_mode: InsertMode::INSERT,
            column_list: Box::new(SQLExprList(
                vec![
                    SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]},
                    SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]},
                    SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}
                ]
            )),
            values_list: vec!(SQLExprList(
                vec![
                    SQLLiteral(0),
                    SQLLiteral(1),
                    SQLBoundParam(0)
                ]
            ))
        },
        parsed
    );

}

#[test]
fn update() {

{
    let dialect = AnsiSQLDialect::new();
    let sql = String::from("UPDATE foo SET a = 'hello', b = 12345 WHERE c > 10");
    let tokens = sql.tokenize(&dialect).unwrap();
    let parsed = tokens.parse().unwrap();

    assert_eq!(
        SQLUpdate {
            table: Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]}),
            assignments: Box::new(SQLExprList(
                vec![
                    SQLBinary{
                        left: Box::new(SQLIdentifier{id: String::from("a"), parts: vec![String::from("a")]}),
                        op: EQ,
                        right: Box::new(SQLLiteral(0))
                    },
                    SQLBinary{
                        left: Box::new(SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]}),
                        op: EQ,
                        right: Box::new(SQLLiteral(1))
                    }
                ]
            )),
            selection: Some(Box::new(SQLBinary{
                left: Box::new(SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}),
                op: GT,
                right : Box::new(SQLLiteral(2))
            }))
        },
        parsed
    );

    println!("{:#?}", parsed);

    let ansi_writer = AnsiSQLWriter{literal_tokens: &tokens.literals};
    let writer = SQLWriter::new(vec![&ansi_writer]);
    let rewritten = writer.write(&parsed).unwrap();
    assert_eq!(format_sql(&rewritten), format_sql(&sql));

    println!("Rewritten: {:?}", rewritten);
}
{
    let dialect = AnsiSQLDialect::new();
    let sql = String::from("UPDATE warehouse SET w_ytd = w_ytd + 2117.1 WHERE w_id = 1");
    let tokens = sql.tokenize(&dialect).unwrap();
    let parsed = tokens.parse().unwrap();
    
    let upd = SQLUpdate{
        table: Box::new(SQLIdentifier{id: String::from("warehouse"), parts: vec![String::from("warehouse")] }),
            assignments: Box::new(
                SQLExprList(vec![
                    SQLBinary {
                        left: Box::new(SQLIdentifier{id: String::from("w_ytd"), parts: vec![String::from("w_ytd")]}),
                        op:EQ,
                        right: Box::new(
                            SQLBinary {
                                left: Box::new(SQLIdentifier{id: String::from("w_ytd"), 
                                        parts: vec![String::from("w_ytd")]}),
                                op: ADD,
                                right: Box::new(SQLLiteral(0))})}]
        )
    ),
    selection: Some(Box::new(
            SQLBinary {
                left: Box::new(SQLIdentifier{id: String::from("w_id"), parts: vec![String::from("w_id")]}),
                op: EQ,
                right: Box::new(SQLLiteral(1))
            })
    )
    };
    
    println!("{:#?}", parsed);
    assert_eq!(upd, parsed);

    let ansi_writer = AnsiSQLWriter{literal_tokens: &tokens.literals};
    let writer = SQLWriter::new(vec![&ansi_writer]);
    let rewritten = writer.write(&parsed).unwrap();

    assert_eq!(format_sql(&rewritten), format_sql(&sql));

    println!("Rewritten: {:?}", rewritten);
}

}

#[test]
fn delete() {
        let dialect = AnsiSQLDialect::new();
        let sql = String::from("DELETE FROM foo WHERE c > 10");
        let tokens = sql.tokenize(&dialect).unwrap();
        let parsed = tokens.parse().unwrap();

        assert_eq!(
        SQLDelete {
            table: Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]}),
            selection: Some(Box::new(SQLBinary{
                left: Box::new(SQLIdentifier{id: String::from("c"), parts: vec![String::from("c")]}),
                op: GT,
                right : Box::new(SQLLiteral(0))
            }))
        },
        parsed
    );

    println!("{:#?}", parsed);

    let ansi_writer = AnsiSQLWriter{literal_tokens: &tokens.literals};
    let writer = SQLWriter::new(vec![&ansi_writer]);
    let rewritten = writer.write(&parsed).unwrap();
    assert_eq!(format_sql(&rewritten), format_sql(&sql));

    println!("Rewritten: {:?}", rewritten);
}


#[test]
fn select_function_calls() {
    let dialect = AnsiSQLDialect::new();
    let sql = String::from("SELECT COUNT(id) FROM foo WHERE LOWER(b) = 'lowercase'");
    let tokens = sql.tokenize(&dialect).unwrap();
    let parsed = tokens.parse().unwrap();

    assert_eq!(
        SQLSelect {
            expr_list: Box::new(SQLExprList(vec![
                SQLFunctionCall{
                    identifier: Box::new(SQLIdentifier{id: String::from("COUNT"), parts: vec![String::from("COUNT")]}),
                    args: vec![SQLIdentifier{id: String::from("id"), parts: vec![String::from("id")]}]
                }
            ])),
            relation: Some(Box::new(SQLIdentifier{id: String::from("foo"), parts: vec![String::from("foo")]})),
            selection: Some(Box::new(
                    SQLBinary {
                        left: Box::new(
                            SQLFunctionCall{
                                identifier: Box::new(SQLIdentifier{id: String::from("LOWER"), parts: vec![String::from("LOWER")]}),
                                args: vec![SQLIdentifier{id: String::from("b"), parts: vec![String::from("b")]}]
                            }
                        ),
                        op: EQ,
                        right: Box::new(SQLLiteral(0))
                    })
            ),
            order: None,
            limit: None,
            for_update: false
        },
        parsed
    );

    println!("{:#?}", parsed);

    let ansi_writer = AnsiSQLWriter{literal_tokens: &tokens.literals};
    let writer = SQLWriter::new(vec![&ansi_writer]);
    let rewritten = writer.write(&parsed).unwrap();
    assert_eq!(format_sql(&rewritten), format_sql(&sql));

    println!("Rewritten: {:?}", rewritten);

}

#[test]
fn select_for_update() {
    let dialect = AnsiSQLDialect::new();
    let sql = String::from("SELECT id FROM users WHERE id = 1 FOR UPDATE");
    let tokens = sql.tokenize(&dialect).unwrap();
    let parsed = tokens.parse().unwrap();

    assert_eq!(
        SQLSelect {
            expr_list: Box::new(SQLExprList(vec![
                SQLIdentifier{id: String::from("id"), parts: vec![String::from("id")]}
            ])),
            relation: Some(Box::new(SQLIdentifier{id: String::from("users"), parts: vec![String::from("users")]})),
            selection: Some(Box::new(SQLBinary {
                left: Box::new(SQLIdentifier{id: String::from("id"), parts: vec![String::from("id")]}),
                op: EQ,
                right: Box::new(SQLLiteral(0))
            })),
            order: None,
            limit: None,
            for_update: true
        },
        parsed
    );

    println!("{:#?}", parsed);

    let ansi_writer = AnsiSQLWriter{literal_tokens: &tokens.literals};
    let writer = SQLWriter::new(vec![&ansi_writer]);
    let rewritten = writer.write(&parsed).unwrap();
    assert_eq!(format_sql(&rewritten), format_sql(&sql));

    println!("Rewritten: {:?}", rewritten);


}

#[test]
fn select_limit() {
    let dialect = AnsiSQLDialect::new();
    let sql = String::from("SELECT id FROM users LIMIT 1");
    let tokens = sql.tokenize(&dialect).unwrap();
    let parsed = tokens.parse().unwrap();

    assert_eq!(
        SQLSelect {
            expr_list: Box::new(SQLExprList(vec![
                SQLIdentifier{id: String::from("id"), parts: vec![String::from("id")]}
            ])),
            relation: Some(Box::new(SQLIdentifier{id: String::from("users"), parts: vec![String::from("users")]})),
            selection: None,
            order: None,
            limit: Some(Box::new(SQLLiteral(0))),
            for_update: false
        },
        parsed
    );

    println!("{:#?}", parsed);

    let ansi_writer = AnsiSQLWriter{literal_tokens: &tokens.literals};
    let writer = SQLWriter::new(vec![&ansi_writer]);
    let rewritten = writer.write(&parsed).unwrap();
    assert_eq!(format_sql(&rewritten), format_sql(&sql));

    println!("Rewritten: {:?}", rewritten);


}

#[test]
fn select_with_variables() {
    let dialect = AnsiSQLDialect::new();
    let sql = String::from("SELECT  @@session.auto_increment_increment AS auto_increment_increment, @@character_set_client AS character_set_client,
        @@character_set_connection AS character_set_connection, @@character_set_results AS character_set_results,
        @@character_set_server AS character_set_server, @@init_connect AS init_connect, @@interactive_timeout AS interactive_timeout,
        @@license AS license, @@lower_case_table_names AS lower_case_table_names, @@max_allowed_packet AS max_allowed_packet,
        @@net_buffer_length AS net_buffer_length, @@net_write_timeout AS net_write_timeout, @@query_cache_size AS query_cache_size,
        @@query_cache_type AS query_cache_type, @@sql_mode AS sql_mode, @@system_time_zone AS system_time_zone, @@time_zone AS time_zone,
        @@tx_isolation AS tx_isolation, @@wait_timeout AS wait_timeout");

    let tokens = sql.tokenize(&dialect).unwrap();
    let parsed = tokens.parse().unwrap();

    let ansi_writer = AnsiSQLWriter{literal_tokens: &tokens.literals};
    let writer = SQLWriter::new(vec![&ansi_writer]);
    let rewritten = writer.write(&parsed).unwrap();
    assert_eq!(format_sql(&rewritten), format_sql(&sql));
}

#[test]
fn bulk_insert() {
    let dialect = AnsiSQLDialect::new();
    let sql = String::from("INSERT INTO new_orders (no_o_id, no_d_id, no_w_id) VALUES (1,2,3), (4,5,6)");

    let tokens = sql.tokenize(&dialect).unwrap();
    let parsed = tokens.parse().unwrap();

    assert_eq!(
        SQLInsert{
            table: Box::new(SQLIdentifier{id: String::from("new_orders"), parts: vec![String::from("new_orders")]}),
            insert_mode: InsertMode::INSERT,
            column_list: Box::new(SQLExprList(
                vec![
                    SQLIdentifier{id: String::from("no_o_id"), parts: vec![String::from("no_o_id")]},
                    SQLIdentifier{id: String::from("no_d_id"), parts: vec![String::from("no_d_id")]},
                    SQLIdentifier{id: String::from("no_w_id"), parts: vec![String::from("no_w_id")]}
                ]
            )),
            values_list: vec!(
                SQLExprList(vec![SQLLiteral(0), SQLLiteral(1), SQLLiteral(2)]),
                SQLExprList(vec![SQLLiteral(3), SQLLiteral(4), SQLLiteral(5)])
            )
        },
        parsed
    );

    let ansi_writer = AnsiSQLWriter{literal_tokens: &tokens.literals};
    let writer = SQLWriter::new(vec![&ansi_writer]);
    let rewritten = writer.write(&parsed).unwrap();
    assert_eq!(format_sql(&rewritten), format_sql(&sql));
}



