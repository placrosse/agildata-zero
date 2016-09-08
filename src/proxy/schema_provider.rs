use query::planner::{SchemaProvider, TableMeta, ColumnMeta};
use config::*;
use std::str::FromStr;
use std::collections::HashMap;
use std::sync::Mutex;
use query::{Parser, Tokenizer, ASTNode, MySQLDataType};
use query::dialects::ansisql::*;
use query::dialects::mysqlsql::*;
use encrypt::{NativeType, EncryptionType};

//extern crate mysql;
use mysql;

// Mysql and config backed provider
// locks on mutex to prevent multiple threads querying the database for uncached meta
pub struct MySQLBackedSchemaProvider<'a> {
	config: &'a Config,
	pool: mysql::Pool,
	cache: HashMap<String, TableMeta>,
	mutex: Mutex<u8>
}

impl<'a> MySQLBackedSchemaProvider<'a> {

	pub fn new(config: &'a Config) -> Self {
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
			config: config,
			pool: pool,
			cache: HashMap::new(),
			mutex: Mutex::new(0_u8)
		}
	}

	fn _get_meta(&self, schema: &String, table: &String) -> Result<Option<TableMeta>, String> {
		match self.pool.prep_exec(format!("SHOW CREATE TABLE {}.{}", schema, table),()) {
			Ok(mut result) => match result.next() {
				Some(Ok(row)) => {
					let (_name, sql) = mysql::from_row::<(String,String)>(row);
					let ansi = AnsiSQLDialect::new();
					let dialect = MySQLDialect::new(&ansi);

					let parsed = sql.tokenize(&dialect)?.parse()?;

					self._build_meta(schema, parsed)

				},
				Some(Err(e)) => Err(format!("{}", e)),
				None => Ok(None)
			},
			Err(e) => Err(format!("{}", e)),
		}
	}

	fn _build_meta(&self, schema: &String, parsed: ASTNode) -> Result<Option<TableMeta>, String> {
		match parsed {
			ASTNode::MySQLCreateTable{table: box ASTNode::SQLIdentifier{id: ref table, ..}, ref column_list, ..} => {
				let columns = column_list.iter().map(|c| {
					match c {
						&ASTNode::MySQLColumnDef{column: box ASTNode::SQLIdentifier{ref id, ..}, data_type: box ASTNode::MySQLDataType(ref dt), ..} => {
							if let Some(column_config) = self.config.get_column_config(schema, table, id) {
								Ok(ColumnMeta {
								    name: id.clone(),
								    native_type: column_config.native_type.clone(),
								    encryption: column_config.encryption.clone()
								})
							} else {
								Ok(ColumnMeta {
									name: id.clone(),
									native_type: self._reconcile_native_type(dt)?,
									encryption: EncryptionType::NA
								})
							}
						},
						_ => Err(format!("Illegal"))

					}
				}).collect::<Result<Vec<ColumnMeta>, String>>()?;

				Ok(Some(TableMeta{columns: columns}))
			},
			_ => Err(format!("Unsupported AST to build table meta {:?}", parsed))
		}
	}

	fn _reconcile_native_type(&self, data_type: &MySQLDataType) -> Result<NativeType, String> {
		match data_type {
			&MySQLDataType::Int{..} => Ok(NativeType::U64), // TODO use display
			&MySQLDataType::Varchar{ref length} => Ok(NativeType::Varchar(length.unwrap().clone())),
			_ => Err(format!("Unsupported data type for reconciliation {:?}", data_type))
		}
	}
}

impl<'a> SchemaProvider for MySQLBackedSchemaProvider<'a> {
	fn get_table_meta(&mut self, schema: &String, table: &String) -> Result<Option<&TableMeta>, String> {
		// Lock and do work
		println!("get_table_meta()");
		let _guard = self.mutex.lock();

		let key = format!("{}.{}", schema.to_lowercase(), table.to_lowercase());
		if self.cache.contains_key(&key){
			Ok(self.cache.get(&key))
		} else {
			match self.pool.prep_exec(format!("SHOW TABLES IN {} LIKE '{}'", schema, table),()) {
				Ok(mut result) => {
					match result.next() {
						Some(Ok(row)) => {
							let (t,) = mysql::from_row::<(String, )>(row);
							if t.to_lowercase() ==  table.to_lowercase() {
								match self._get_meta(schema, table)? {
									Some(m) => {
										self.cache.insert(key.clone(), m);
										Ok(self.cache.get(&key))
									},
									None => Ok(None)
								}

							} else {
								Err(format!("Illegal result table name {}", t)) // shouldn't happen.
							}
						},
						Some(Err(e)) => Err(format!("{}", e)),
						None => Ok(None)
					}
				},
				Err(e) => Err(format!("{}", e)),
			}
		}
	}
}

// TODO These are more of integration tests...
// #[cfg(test)]
// mod tests {
//
// 	use super::*;
// 	use query::planner::SchemaProvider;
// 	use config;
//
// 	#[test]
// 	fn test_provider_controlled() {
// 		let config = config::parse_config("zero-config.xml");
//
// 		let mut provider = MySQLBackedSchemaProvider::new(&config);
//
// 		let meta = provider.get_table_meta(&String::from("zero"), &String::from("users")).unwrap();
//
// 		println!("META {:?}", meta);
// 	}
//
// 	#[test]
// 	fn test_provider_uncontrolled() {
// 		let config = config::parse_config("zero-config.xml");
//
// 		let mut provider = MySQLBackedSchemaProvider::new(&config);
//
// 		let meta = provider.get_table_meta(&String::from("zero"), &String::from("uncontrolled")).unwrap();
//
// 		println!("META {:?}", meta);
//
// 	}
// }
