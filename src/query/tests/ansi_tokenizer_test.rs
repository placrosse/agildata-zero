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

use super::super::*;
use super::super::dialects::ansisql::*;

#[test]
fn simple_tokenize() {
    let dialect = AnsiSQLDialect::new();
    let tokens = String::from("SELECT 1 + 1").tokenize(&dialect).unwrap();
    assert_eq!(
        vec![Token::Keyword("SELECT".to_string()),
            Token::Literal(0),
            Token::Operator("+".to_string()),
            Token::Literal(1)
        ],
        tokens.tokens
    );

    // let parsed = tokens.parse().unwrap();
}

#[test]
fn tokenize_comment() {
    let dialect = AnsiSQLDialect::new();
    let tokens = String::from("/* mysql-connector-java-6.0.4 ( Revision: d2d72c397f9880b5861eb144cd8950eff808bffd ) */ SELECT 1 + 1").tokenize(&dialect).unwrap();
    assert_eq!(
        vec![
            //NOTE that for now, comments are actually stripped out by the tokenizer
            //Token::Comment("/* mysql-connector-java-6.0.4 ( Revision: d2d72c397f9880b5861eb144cd8950eff808bffd ) */".to_string()),
            Token::Keyword("SELECT".to_string()),
            Token::Literal(0),
            Token::Operator("+".to_string()),
            Token::Literal(1)
        ],
        tokens.tokens
    );

    // let parsed = tokens.parse().unwrap();
}

#[test]
fn tokenize_with_null() {
    let dialect = AnsiSQLDialect::new();
    let tokens = String::from("SELECT NULL, null").tokenize(&dialect).unwrap();
    assert_eq!(
        vec![Token::Keyword("SELECT".to_string()),
            Token::Literal(0),
            Token::Punctuator(",".to_string()),
            Token::Literal(1)
        ],
        tokens.tokens
    );

    // let parsed = tokens.parse().unwrap();
}

#[test]
fn complex_tokenize() {
    let dialect = AnsiSQLDialect::new();
    let tokens = String::from("SELECT a, 'hello' FROM tOne WHERE b > 2.22 AND c != true").tokenize(&dialect).unwrap();
    assert_eq!(
        vec![Token::Keyword("SELECT".to_string()),
            Token::Identifier("a".to_string()),
            Token::Punctuator(",".to_string()),
            Token::Literal(0),
            Token::Keyword("FROM".to_string()),
            Token::Identifier("tOne".to_string()),
            Token::Keyword("WHERE".to_string()),
            Token::Identifier("b".to_string()),
            Token::Operator(">".to_string()),
            Token::Literal(1),
            Token::Operator("AND".to_string()),
            Token::Identifier("c".to_string()),
            Token::Operator("!=".to_string()),
            Token::Literal(2)
        ],
        tokens.tokens
    );

}
