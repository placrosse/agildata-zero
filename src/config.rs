extern crate xml;
use std::fs::File;
use std::io::Read;
use std::collections::HashMap;
use std::process;
use self::xml::Xml;

use encrypt::*;

use query::{Tokenizer, ASTNode};
use query::MySQLDataType::*;
use query::dialects::ansisql::*;
use query::dialects::mysqlsql::*;



pub fn parse_config(path: &str) -> Config {
	debug!("parse_config() path: {}", path);
	let mut rdr = match File::open(path) {
        Ok(file) => file,
        Err(err) => {
            println!("Unable to open configuration file: {}", path);
            process::exit(1);
        }
    };

	let mut p = xml::Parser::new();
    let mut e = xml::ElementBuilder::new();
	let mut b = ConfigBuilder::new();

    let mut string = String::new();
    if let Err(err) = rdr.read_to_string(&mut string) {
        error!("Reading failed: {}", err);
        process::exit(1);
    };

    p.feed_str(&string);
    for event in p.filter_map(|x| e.handle_event(x)) {
        match event {
            Ok(e) => {
				match e {
					xml::Element{name: n, children: c, .. } => match &n as &str {
						"client-config" => parse_client_config(&mut b, c),
						_ => panic!("Unrecognized parent XML element {}", n)
					}
				}
			},
            Err(e) => error!("{}", e),
        }
    }

	b.build()
}

fn parse_client_config(builder: &mut ConfigBuilder, children: Vec<Xml>) {

	for node in children {
		match node {
			Xml::ElementNode(e) => {
				match &e.name as &str {
					"schema" => {
						let mut sb = SchemaConfigBuilder::new();
						sb.set_name(get_attr_or_fail("name", &e));
						parse_schema_config(&mut sb, e.children);
						builder.add_schema(sb.build());
					},
					"connection" => {
						for prop in e.children {
							match prop {
								Xml::ElementNode(n) => match &n.name as &str {
									"property" => {
										let key = get_attr_or_fail("name", &n);
										let val = get_attr_or_fail("value", &n);
										builder.add_conn_prop(key, val)
									},
									_ => panic!("expected property, received {}", n.name)
								},
								_ => {} // dont care yet
							}
						}
					},
					"client" => {
						for prop in e.children {
							match prop {
								Xml::ElementNode(n) => match &n.name as &str {
									"property" => {
										let key = get_attr_or_fail("name", &n);
										let val = get_attr_or_fail("value", &n);
										builder.add_client_prop(key, val)
									},
									_ => panic!("expected property, received {}", n.name)
								},
								_ => {} // dont care yet
							}
						}
					},
                    "parsing" => {
                        for prop in e.children {
                            match prop {
                                Xml::ElementNode(n) => match &n.name as &str {
                                    "property" => {
                                        let key = get_attr_or_fail("name", &n);
                                        let val = get_attr_or_fail("value", &n);
                                        builder.add_parsing_prop(key, val)
                                    },
                                    _ => panic!("expected property, received {}", n.name)
                                },
                                _ => {} // dont care yet
                            }
                        }
                    },
					_ => panic!("Unexpected element tag {}", e.name)
				}
			},
			_ => {} // dont care
		}
	}
}

fn parse_schema_config(builder: &mut SchemaConfigBuilder, children: Vec<Xml>) {
	for node in children {
		match node {
			Xml::ElementNode(e) => match &e.name as &str {
				"table" => {
					let mut tb = TableConfigBuilder::new();
					tb.set_name(get_attr_or_fail("name", &e));
					parse_table_config(&mut tb, e.children);
					builder.add_table(tb.build());

				},
				_ => panic!("Unexpected element tag {}", e.name)
			},
			_ => {} // dont' care yet
		}
	}
}

fn parse_table_config(builder: &mut TableConfigBuilder, children: Vec<Xml>) {
    use std::env;
    let tbl_name: String = builder.name.clone().unwrap().to_uppercase();  // TODO do we need the schema name as well?

	for node in children {
		match node {
			Xml::ElementNode(e) => match &e.name as &str {
				"column" => {
                    let name = get_attr_or_fail("name", &e);
					let native_type = get_attr_or_fail("type", &e);
                    let encryption = get_attr_or_fail("encryption", &e);
                    let key = if encryption.to_uppercase() != "NONE" {
                                  determine_key(&
                                      env::var(format!("ZERO_{}_{}", &tbl_name, &name.to_uppercase()))
                                        .ok()
                                        .unwrap_or_else(|| get_attr_or_fail("key", &e))
                                  )
                              } else {
                                  [0u8; 32]
                              };
					builder.add_column(ColumnConfig{
						name: name,
                        native_type: determine_native_type(&native_type),
                        encryption: determine_encryption(&encryption),
                        key: key,
					});
				},
				_ => panic!("Unexpected element tag {}", e.name)
			},
			_ => {} // dont' care yet
		}
	}
}

fn get_attr_or_fail(name: &str, element: &xml::Element) -> String {
	match element.get_attribute(name, None) {
		Some(v) => v.to_string(),
		None => panic!("Missing attribute {}", name)
	}
}

fn determine_native_type(native_type: &String) -> NativeType {
	let ansi = AnsiSQLDialect::new();
	let dialect = MySQLDialect::new(&ansi);
	let tokens = native_type.tokenize(&dialect).unwrap();

	match dialect.parse_data_type(&tokens) {
		Ok(p) => {
//			#[derive(Debug, PartialEq)]
//			pub enum MySQLDataType {
//				Bit{display: Option<u32>},
//				TinyInt{display: Option<u32>},
//				SmallInt{display: Option<u32>},
//				MediumInt{display: Option<u32>},
//				Int{display: Option<u32>},
//				BigInt{display: Option<u32>},
//				Decimal{precision: Option<u32>, scale: Option<u32>},
//				Float{precision: Option<u32>, scale: Option<u32>},
//				Double{precision: Option<u32>, scale: Option<u32>},
//				Bool,
//				Date,
//				DateTime{fsp: Option<u32>},
//				Timestamp{fsp: Option<u32>},
//				Time{fsp: Option<u32>},
//				Year{display: Option<u32>},
//				Char{length: Option<u32>},
//				NChar{length: Option<u32>},
//				CharByte{length: Option<u32>},
//				Varchar{length: Option<u32>},
//				NVarchar{length: Option<u32>},
//				Binary{length: Option<u32>},
//				VarBinary{length: Option<u32>},
//				TinyBlob,
//				TinyText,
//				Blob{length: Option<u32>},
//				Text{length: Option<u32>},
//				MediumBlob,
//				MediumText,
//				LongBlob,
//				LongText,
//				Enum{values: Box<ASTNode>},
//				Set{values: Box<ASTNode>}
//			}
			match p {
				ASTNode::MySQLDataType(ref dt) => match dt {
					&Bit{..} | &TinyInt{..} |
						&SmallInt{..} | &MediumInt{..} |
						&Int{..} | &BigInt{..}
						=> NativeType::U64,

					&Double{..} | &Float{..} => NativeType::F64,
					&Decimal{..} => NativeType::D128,
					&Bool => NativeType::BOOL,
					&Char{ref length} | &NChar{ref length}  => NativeType::Char(match length {
						&Some(l) => l,
						&None => 1 // MySQL's default
					}),
					&Varchar{ref length} | &NVarchar{ref length} => NativeType::Varchar(match length {
						&Some(l) => l,
						&None => panic!("CHARACTER VARYING datatype requires length") // TODO parser shouldn't allow this
					}),

					_ => panic!("Unsupported data type {:?}", dt)
				},
				_ => panic!("Unexpected native type expression {:?}", p)
			}
		},
		Err(e) => panic!("Failed to parse data type {} due to : {}", native_type, e)
	}
}

fn determine_encryption(encryption: &String) -> EncryptionType {
	match &encryption.to_uppercase() as &str {
		"AES" => EncryptionType::AES,
		// "AES-SALTED" => EncryptionType::AES_SALT,
		"OPE" => EncryptionType::OPE,
		"NONE" => EncryptionType::NA,
		_ => panic!("Unsupported encryption type {}", encryption)
	}

}

fn determine_key(key: &str) -> [u8; 32] {
    hex_key(key)
}

#[derive(Debug, PartialEq)]
pub struct ColumnConfig {
	pub name: String,
	pub encryption: EncryptionType,
    pub key: [u8; 32],
	pub native_type: NativeType
}

#[derive(Debug, PartialEq)]
pub struct TableConfig {
	pub name: String,
	pub column_map: HashMap<String, ColumnConfig>
}

struct TableConfigBuilder {
	column_map: HashMap<String, ColumnConfig>,
	name: Option<String>
}

impl TableConfigBuilder {
	fn new() -> TableConfigBuilder {
		TableConfigBuilder{column_map: HashMap::new(), name: None}
	}

	fn set_name(&mut self, name: String) {
		self.name = Some(name);
	}

	fn add_column(&mut self, column: ColumnConfig) {
		let key = column.name.clone(); // TODO downcase
		self.column_map.insert(key, column);
	}

	fn build(self) -> TableConfig {
		TableConfig {name: self.name.unwrap(), column_map: self.column_map}
	}
}

#[derive(Debug)]
pub struct SchemaConfig {
	name: String,
	table_map: HashMap<String, TableConfig>
}

struct SchemaConfigBuilder {
	name: Option<String>,
	table_map: HashMap<String, TableConfig>
}

impl SchemaConfigBuilder {
	fn new() -> SchemaConfigBuilder {
		SchemaConfigBuilder{name: None, table_map: HashMap::new()}
	}

	fn set_name(&mut self, name: String)  {
		self.name = Some(name);
	}

	fn add_table(&mut self, table: TableConfig) {
		let key = table.name.clone(); // TODO downcase
		self.table_map.insert(key, table);
	}

	fn build(self) -> SchemaConfig {
		SchemaConfig{name: self.name.unwrap(), table_map: self.table_map}
	}
}

#[derive(Debug)]
pub struct ConnectionConfig {
	pub props: HashMap<String, String>
}

#[derive(Debug)]
pub struct ClientConfig {
	pub props: HashMap<String, String>
}

#[derive(Debug)]
pub struct ParsingConfig {
    pub props: HashMap<String, String>
}


#[derive(Debug)]
pub struct Config {
	schema_map: HashMap<String, SchemaConfig>,
	connection_config : ConnectionConfig,
	client_config: ClientConfig,
    parsing_config: ParsingConfig
}

struct ConfigBuilder {
	schema_map : HashMap<String, SchemaConfig>,
	conn_props : HashMap<String, String>,
	client_props : HashMap<String,String>,
    parsing_props : HashMap<String, String>
}

impl ConfigBuilder {
	fn new() -> ConfigBuilder {
		ConfigBuilder{
			schema_map: HashMap::new(),
			conn_props: HashMap::new(),
			client_props: HashMap::new(),
            parsing_props: HashMap::new()
		}
	}

	fn add_schema(&mut self, schema: SchemaConfig) {
		let key = schema.name.clone(); // TODO downcase
		self.schema_map.insert(key, schema);
	}

	fn add_client_prop(&mut self, key: String, value: String) {
		self.client_props.insert(key, value);
	}

    fn add_conn_prop(&mut self, key: String, value: String) {
        self.conn_props.insert(key, value);
    }

    fn add_parsing_prop(&mut self, key: String, value: String) {
        self.parsing_props.insert(key, value);
    }

	fn build(self) -> Config {
		Config {
			schema_map: self.schema_map,
			connection_config : ConnectionConfig {props: self.conn_props},
			client_config: ClientConfig {props: self.client_props},
            parsing_config: ParsingConfig{props: self.parsing_props}
		}
	}
}

pub trait TConfig {
	fn get_column_config(&self, schema: &String, table: &String, column: &String) -> Option<&ColumnConfig>;
	fn get_table_config(&self, schema: &String, table: &String) -> Option<&TableConfig>;
	fn get_schema_config(&self, schema: &String) -> Option<&SchemaConfig>;
    fn get_parsing_config(&self) -> &ParsingConfig;
	fn get_connection_config(&self) -> &ConnectionConfig;
	fn get_client_config(&self) -> &ClientConfig;
}

impl TConfig for Config {

	fn get_column_config(&self, schema: &String, table: &String, column: &String) -> Option<&ColumnConfig> {
		match self.get_table_config(schema, table) {
			Some(t) => t.get_column_config(column),
			None => None
		}
	}

	fn get_table_config(&self, schema: &String, table: &String) -> Option<&TableConfig> {
		match self.get_schema_config(schema) {
			Some(s) => s.get_table_config(table),
			None => None
		}
	}

	fn get_schema_config(&self, schema: &String) -> Option<&SchemaConfig> {
		self.schema_map.get(schema)
	}

	fn get_connection_config(&self) -> &ConnectionConfig {
		&self.connection_config
	}

	fn get_client_config(&self) -> &ClientConfig {
		&self.client_config
	}

    fn get_parsing_config(&self) -> &ParsingConfig {
        &self.parsing_config
    }

}

pub trait TSchemaConfig {
	fn get_table_config(&self, table: &String) -> Option<&TableConfig>;
}

impl TSchemaConfig for SchemaConfig {
	fn get_table_config(&self, table: &String) -> Option<&TableConfig> {
		self.table_map.get(table)
	}
}

pub trait TTableConfig {
	fn get_column_config(&self, column: &String) -> Option<&ColumnConfig>;
}

impl TTableConfig for TableConfig {
	fn get_column_config(&self, column: &String) -> Option<&ColumnConfig> {
		self.column_map.get(column)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use encrypt::NativeType::*;

    #[test]
	fn config_test() {
		let config = super::parse_config("zero-config.xml");
		debug!("CONFIG {:#?}", config);
		debug!("HERE {:#?}", config.get_column_config(&String::from("zero"), &String::from("users"), &String::from("age")))
	}

	#[test]
	fn test_config_data_types() {
		let s_config = super::parse_config("src/test/test-zero-config.xml");
		// Numerics

		let mut config = s_config.get_table_config(&"config_test".into(), &"numerics".into()).unwrap();
		assert_eq!(config.column_map.get("a").unwrap().native_type,U64);
		assert_eq!(config.column_map.get("b").unwrap().native_type,U64);
		assert_eq!(config.column_map.get("c").unwrap().native_type,U64);
		assert_eq!(config.column_map.get("d").unwrap().native_type,U64);
		assert_eq!(config.column_map.get("e").unwrap().native_type,BOOL);
		assert_eq!(config.column_map.get("f").unwrap().native_type,BOOL);
		assert_eq!(config.column_map.get("g").unwrap().native_type,U64);
		assert_eq!(config.column_map.get("h").unwrap().native_type,U64);
		assert_eq!(config.column_map.get("i").unwrap().native_type,U64);
		assert_eq!(config.column_map.get("j").unwrap().native_type,U64);
		assert_eq!(config.column_map.get("k").unwrap().native_type,U64);
		assert_eq!(config.column_map.get("l").unwrap().native_type,U64);
		assert_eq!(config.column_map.get("m").unwrap().native_type,U64);
		assert_eq!(config.column_map.get("n").unwrap().native_type,U64);
		assert_eq!(config.column_map.get("o").unwrap().native_type,D128);
		assert_eq!(config.column_map.get("p").unwrap().native_type,D128);
		assert_eq!(config.column_map.get("q").unwrap().native_type,D128);
		assert_eq!(config.column_map.get("r").unwrap().native_type,D128);
		assert_eq!(config.column_map.get("s").unwrap().native_type,D128);
		assert_eq!(config.column_map.get("t").unwrap().native_type,D128);
		assert_eq!(config.column_map.get("u").unwrap().native_type,F64);
		assert_eq!(config.column_map.get("v").unwrap().native_type,F64);
		assert_eq!(config.column_map.get("w").unwrap().native_type,F64);
		assert_eq!(config.column_map.get("x").unwrap().native_type,F64);
		assert_eq!(config.column_map.get("y").unwrap().native_type,F64);
		assert_eq!(config.column_map.get("z").unwrap().native_type,F64);
		assert_eq!(config.column_map.get("aa").unwrap().native_type,F64);
		assert_eq!(config.column_map.get("ab").unwrap().native_type,F64);
		assert_eq!(config.column_map.get("ac").unwrap().native_type,F64);

		config = s_config.get_table_config(&"config_test".into(), &"character".into()).unwrap();
		assert_eq!(config.column_map.get("a").unwrap().native_type,Char(1));
		assert_eq!(config.column_map.get("b").unwrap().native_type,Char(1));
		assert_eq!(config.column_map.get("c").unwrap().native_type,Char(255));
		assert_eq!(config.column_map.get("d").unwrap().native_type,Char(1));
		assert_eq!(config.column_map.get("e").unwrap().native_type,Char(255));
		assert_eq!(config.column_map.get("f").unwrap().native_type,Char(1));
		assert_eq!(config.column_map.get("g").unwrap().native_type,Char(1));
		assert_eq!(config.column_map.get("h").unwrap().native_type,Char(255));
		assert_eq!(config.column_map.get("i").unwrap().native_type,Char(50));
		assert_eq!(config.column_map.get("j").unwrap().native_type,Varchar(50));
		assert_eq!(config.column_map.get("k").unwrap().native_type,Varchar(50));
		assert_eq!(config.column_map.get("l").unwrap().native_type,Varchar(50));

	}

}
