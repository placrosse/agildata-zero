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

use query::planner::{SchemaProvider, TableMeta, ColumnMeta};
use config::*;
use std::str::FromStr;
use std::collections::HashMap;
use std::sync::Mutex;
use query::{Parser, Tokenizer, ASTNode, MySQLDataType};
use query::dialects::ansisql::*;
use query::dialects::mysqlsql::*;
use encrypt::{NativeType, EncryptionType};
use std::rc::Rc;
use error::ZeroError;
use mysql;

// Mysql and config backed provider
// locks on mutex to prevent multiple threads querying the database for uncached meta
#[derive(Debug)]
pub struct MySQLBackedSchemaProvider {
    config: Rc<Config>,
    pool: mysql::Pool,
    cache: Mutex<HashMap<String, Rc<TableMeta>>>
}

impl MySQLBackedSchemaProvider {

    pub fn new(config: Rc<Config>) -> Self {
        let conn = config.get_connection_config();
        let conn_host = conn.props.get("host").unwrap().clone();
        let default_port = &String::from("3306");
        let conn_port = u16::from_str(conn.props.get("port").unwrap_or(default_port)).unwrap();
        let user = conn.props.get("user").unwrap().clone();
        let pw = conn.props.get("password").unwrap().clone();
        //
        let mut builder = mysql::conn::OptsBuilder::default();

        builder.user(Some(user))
               .pass(Some(pw))
               .ip_or_hostname(Some(conn_host))
               .tcp_port(conn_port);
        let opts: mysql::conn::Opts = builder.into();
        let pool = mysql::Pool::new(opts).unwrap();

        MySQLBackedSchemaProvider {
            config: config.clone(),
            pool: pool,
            cache: Mutex::new(HashMap::new())
        }
    }

    fn _get_meta(&self, schema: &String, table: &String) -> Result<Option<TableMeta>, Box<ZeroError>> {
        match self.pool.prep_exec(format!("SHOW CREATE TABLE {}.{}", schema, table),()) {
            Ok(mut result) => match result.next() {
                Some(Ok(row)) => {
                    let (_name, sql) = mysql::from_row::<(String,String)>(row);
                    let ansi = AnsiSQLDialect::new();
                    let dialect = MySQLDialect::new(&ansi);

                    let parsed = sql.tokenize(&dialect)?.parse()?;
                    self._build_meta(schema, parsed)

                },
                Some(Err(e)) =>  Err(ZeroError::SchemaError{
                    message: format!("{}", e).into(),
                    code: "1064".into()
                }.into()),
                None => Ok(None)
            },
            Err(e) => Err(ZeroError::SchemaError{
                message: format!("{}", e).into(),
                code: "1064".into()
            }.into()),
        }
    }

    fn _build_meta(&self, schema: &String, parsed: ASTNode) -> Result<Option<TableMeta>, Box<ZeroError>> {
        match parsed {
            ASTNode::MySQLCreateTable{table: box ASTNode::SQLIdentifier{id: ref table, ..}, ref column_list, ..} => {
                let columns = column_list.iter().map(|c| {
                    match c {
                        &ASTNode::MySQLColumnDef{column: box ASTNode::SQLIdentifier{ref id, ..}, data_type: box ref dt, ref qualifiers} => {
                            if let Some(column_config) = self.config.get_column_config(schema, table, id) {
                                Ok(ColumnMeta {
                                    name: id.clone(),
                                    native_type: column_config.native_type.clone(),
                                    encryption: column_config.encryption.clone(),
                                    key: column_config.key.clone(),
                                })
                            } else {
                                let default = vec![];
                                let qs = qualifiers.as_ref().unwrap_or(&default);
                                //qualifiers: Option<Vec<ASTNode>>
                                Ok(ColumnMeta {
                                    name: id.clone(),
                                    native_type: reconcile_native_type(dt, &reconcile_column_qualifiers(&qs, false)?)?,
                                    encryption: EncryptionType::NA,
                                    key: [0u8; 32],
                                })
                            }
                        },
                        _ => Err(ZeroError::SchemaError{
                                message: format!("Illegal").into(),
                                code: "1064".into()
                            }.into())

                    }
                }).collect::<Result<Vec<ColumnMeta>, Box<ZeroError>>>()?;

                Ok(Some(TableMeta{columns: columns}))
            },
            _ =>Err(ZeroError::SchemaError{
                    message: format!("Unsupported AST to build table meta {:?}", parsed).into(),
                    code: "1064".into()
                }.into())
        }
    }

    fn _reconcile_native_type(&self, data_type: &MySQLDataType) -> Result<NativeType, Box<ZeroError>> {
        match data_type {
            &MySQLDataType::Int{..} => Ok(NativeType::U64), // TODO use display
            &MySQLDataType::Varchar{ref length} => Ok(NativeType::Varchar(length.unwrap().clone())),
            _ => Err(ZeroError::SchemaError{
                    message: format!("Unsupported data type for reconciliation {:?}", data_type).into(),
                    code: "1064".into()
                }.into())
        }
    }
}

impl SchemaProvider for MySQLBackedSchemaProvider {
    fn get_table_meta(&self, schema: &String, table: &String) -> Result<Option<Rc<TableMeta>>, Box<ZeroError>> {
        // Lock and do work
        debug!("get_table_meta()");
        let mut c = self.cache.lock().unwrap();

        let key = format!("{}.{}", schema.to_lowercase(), table.to_lowercase());
        if !c.contains_key(&key){
            match self.pool.prep_exec(format!("SHOW TABLES IN {} LIKE '{}'", schema, table),()) {
                Ok(mut result) => {
                    match result.next() {
                        Some(Ok(row)) => {
                            let (t,) = mysql::from_row::<(String, )>(row);
                            if t.to_lowercase() ==  table.to_lowercase() {
                                match self._get_meta(schema, table)? {
                                    Some(m) => {
                                        c.insert(key.clone(), Rc::new(m));

                                    },
                                    None => return Ok(None)
                                }

                            } else {
                                return Err(ZeroError::SchemaError{
                                    message: format!("Illegal result table name {}", t).into(),
                                    code: "1064".into()
                                }.into())//shouldn't happen

                            }
                        },
                        Some(Err(e)) => return Err(ZeroError::SchemaError{
                                    message: format!("{}", e).into(),
                                    code: "1064".into()
                                }.into()),
                        None => return Ok(None)
                    }
                },
                Err(e) => return return Err(ZeroError::SchemaError{
                    message: format!("{}", e).into(),
                    code: "1064".into()
                }.into()),
            }
        }

        match c.get(&key) {
            Some(rc) => Ok(Some(rc.clone())),
            None => Ok(None)
        }
    }
}

// #[cfg(test)]
// mod tests {
//
//  use super::*;
//  // use query::planner::SchemaProvider;
//  use config;
//
//  use query::dialects::ansisql::*;
//  use query::dialects::mysqlsql::*;
//  use query::{Tokenizer, Parser, SQLWriter, Writer, ASTNode};
//  use query::planner::{Planner, RelVisitor, Rel, SchemaProvider, TableMeta, ColumnMeta};
//  use encrypt::{EncryptionType, NativeType};
//  use std::rc::Rc;
//  use std::error::Error;
//
//  #[test]
//  fn test_provider_controlled() {
//      let config = config::parse_config("zero-config.xml");
//
//      let mut provider = MySQLBackedSchemaProvider::new(&config);
//
//      let meta = provider.get_table_meta(&String::from("zero"), &String::from("users")).unwrap();
//
//      println!("META {:?}", meta);
//  }
//
//  #[test]
//  fn test_provider_uncontrolled() {
//      let config = config::parse_config("zero-config.xml");
//
//      let mut provider = MySQLBackedSchemaProvider::new(&config);
//
//      let meta = provider.get_table_meta(&String::from("zero"), &String::from("uncontrolled")).unwrap();
//
//      println!("META {:?}", meta);
//
//  }
//
//  #[test]
//  fn test_real() {
//      let sql = String::from("SELECT ID FROM information_schema.processlist");
//      parse_and_plan(sql).unwrap();
//  }
//
//  fn parse_and_plan(sql: String) -> Result<(ASTNode, Rel), Box<Error>> {
//      let config = config::parse_config("zero-config.xml");
//      let provider = MySQLBackedSchemaProvider::new(&config);
//      let ansi = AnsiSQLDialect::new();
//      let dialect = MySQLDialect::new(&ansi);
//
//      let parsed = sql.tokenize(&dialect)?.parse()?;
//
//      let s = String::from("zero");
//      let default_schema = Some(&s);
//      let planner = Planner::new(default_schema, &provider);
//      let plan = planner.sql_to_rel(&parsed)?.unwrap();
//      Ok((parsed, plan))
//
//  }
// }
