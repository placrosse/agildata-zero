extern crate xml;
use std::fs::File;
use std::io::Read;
use std::collections::HashMap;
use std::process;
use self::xml::Xml;

use encrypt::*;

use query::{Tokenizer, ASTNode, MySQLColumnQualifier};
use query::MySQLDataType::*;
use query::dialects::ansisql::*;
use query::dialects::mysqlsql::*;
use error::ZeroError;

// Supported qualifiers
#[derive(Debug, PartialEq)]
enum NativeTypeQualifier {
	SIGNED,
	UNSIGNED
}



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
					let dt = match determine_native_type(&native_type) {
						Ok(t) => t,
						Err(e) => panic!("Failed to determine data type for {}.{} : {}", tbl_name, name, e)
					};

					let encrypt_type = determine_encryption(&encryption);
					if encrypt_type != EncryptionType::NA && !dt.is_supported() {
						panic!("Column: {}.{} Native Type {:?} is not supported for encryption {:?}",
							tbl_name, name, native_type, encrypt_type
						)
					}

					builder.add_column(ColumnConfig{
						name: name,
                        native_type: dt,
                        encryption: encrypt_type,
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

fn determine_native_type(native_type: &String) -> Result<NativeType, Box<ZeroError>> {
	let ansi = AnsiSQLDialect::new();
	let dialect = MySQLDialect::new(&ansi);
	let tokens = native_type.tokenize(&dialect).unwrap();

	let data_type = dialect.parse_data_type(&tokens)?;

	let parsed_qs = dialect.parse_column_qualifiers(&tokens)?.unwrap_or(vec![]);

	// Iterate over qualifiers and propagate error on unsupported
	// potential support could be DEFAULT, [NOT] NULL, etc
	let qualifiers = parsed_qs
		.iter().map(|o| {
			match o {
				&ASTNode::MySQLColumnQualifier(ref q) => match q {
					&MySQLColumnQualifier::Signed => Ok(NativeTypeQualifier::SIGNED),
					&MySQLColumnQualifier::Unsigned => Ok(NativeTypeQualifier::UNSIGNED),
					_ => Err(ZeroError::SchemaError{message: format!("Unsupported data type qualifier {:?}", q), code: "123".into()}.into())
				},
				_ => Err(ZeroError::SchemaError{message: format!("Unsupported option {:?}", o), code: "123".into()}.into())
			}
		}).collect::<Result<Vec<NativeTypeQualifier>, Box<ZeroError>>>()?;

	Ok(match data_type {
		ASTNode::MySQLDataType(ref dt) => match dt {
			&Bit{..} | &TinyInt{..} |
			&SmallInt{..} | &MediumInt{..} |
			&Int{..} | &BigInt{..} => {
				if qualifiers.contains(&NativeTypeQualifier::SIGNED) {
					NativeType::I64
				} else {
					NativeType::U64
				}
			},

			&Double{..} | &Float{..} => NativeType::F64,
			&Decimal{..} => NativeType::D128,
			&Bool => NativeType::BOOL,
			&Char{ref length} | &NChar{ref length}  => NativeType::Char(length.unwrap_or(1)),
			&Varchar{ref length} | &NVarchar{ref length} => NativeType::Varchar(match length {
				&Some(l) => l,
				&None => return Err(ZeroError::SchemaError{message: "CHARACTER VARYING datatype requires length".into(), code: "123".into()}.into())
			}),
			&Date => NativeType::DATE,
			&DateTime{ref fsp} => NativeType::DATETIME(fsp.unwrap_or(0)),
			&Time{ref fsp} => NativeType::TIME(fsp.unwrap_or(0)),
			&Timestamp{ref fsp} => NativeType::TIMESTAMP(fsp.unwrap_or(0)),
			&Year{ref display} => NativeType::YEAR(display.unwrap_or(4)),
			// TODO there's some length spillover here if encrypted, handle here or during translate?
			&Binary{ref length} | &CharByte{ref length} => NativeType::FIXEDBINARY(length.unwrap_or(1)),
			&VarBinary{ref length} => NativeType::VARBINARY(match length {
				&Some(l) => l,
				&None => return Err(ZeroError::SchemaError{message: "VARBINARY requires length argument".into(), code: "123".into()}.into()) // TODO parser shouldn't allow this
			}),
			&Blob{ref length} => NativeType::VARBINARY(length.unwrap_or(2_u32.pow(16))),
			&TinyBlob => NativeType::VARBINARY(2_u32.pow(8)),
			&MediumBlob => NativeType::LONGBLOB(2_u64.pow(24)),
			&LongBlob => NativeType::LONGBLOB(2_u64.pow(32)),
			&Text{ref length} => NativeType::Varchar(length.unwrap_or(2_u32.pow(16))),
			&TinyText => NativeType::Varchar(2_u32.pow(8)),
			&MediumText => NativeType::LONGTEXT(2_u64.pow(24)),
			&LongText => NativeType::LONGTEXT(2_u64.pow(32)),

			_ => return Err(ZeroError::SchemaError{message: format!("Unsupported data type {:?}", dt), code: "123".into()}.into())
		},
		_ => return Err(ZeroError::SchemaError{message: format!("Unexpected native type expression {:?}", data_type), code: "123".into()}.into())

	})
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
		let test_schema = "data_types".into();
		// Numerics

		let mut config = s_config.get_table_config(&test_schema, &"numerics".into()).unwrap();
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

		config = s_config.get_table_config(&test_schema, &"characters".into()).unwrap();
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


		config = s_config.get_table_config(&test_schema, &"temporal".into()).unwrap();
		assert_eq!(config.column_map.get("a").unwrap().native_type,DATE);
		assert_eq!(config.column_map.get("b").unwrap().native_type,DATETIME(0));
		assert_eq!(config.column_map.get("c").unwrap().native_type,DATETIME(6));
		assert_eq!(config.column_map.get("d").unwrap().native_type,TIME(0));
		assert_eq!(config.column_map.get("e").unwrap().native_type,TIME(6));
		assert_eq!(config.column_map.get("f").unwrap().native_type,TIMESTAMP(0));
		assert_eq!(config.column_map.get("g").unwrap().native_type,TIMESTAMP(6));
		assert_eq!(config.column_map.get("h").unwrap().native_type,YEAR(4));
		assert_eq!(config.column_map.get("i").unwrap().native_type,YEAR(4));

		config = s_config.get_table_config(&test_schema, &"binary".into()).unwrap();
		assert_eq!(config.column_map.get("a").unwrap().native_type,FIXEDBINARY(1));
		assert_eq!(config.column_map.get("b").unwrap().native_type,FIXEDBINARY(50));
		assert_eq!(config.column_map.get("c").unwrap().native_type,VARBINARY(50));
		assert_eq!(config.column_map.get("d").unwrap().native_type,VARBINARY(2_u32.pow(8)));
		assert_eq!(config.column_map.get("e").unwrap().native_type,Varchar(2_u32.pow(8)));
		assert_eq!(config.column_map.get("f").unwrap().native_type,VARBINARY(2_u32.pow(16)));
		assert_eq!(config.column_map.get("g").unwrap().native_type,VARBINARY(50));
		assert_eq!(config.column_map.get("h").unwrap().native_type,Varchar(2_u32.pow(16)));
		assert_eq!(config.column_map.get("i").unwrap().native_type,Varchar(100));
		assert_eq!(config.column_map.get("j").unwrap().native_type,LONGBLOB(2_u64.pow(24)));
		assert_eq!(config.column_map.get("k").unwrap().native_type,LONGTEXT(2_u64.pow(24)));
		assert_eq!(config.column_map.get("l").unwrap().native_type,LONGBLOB(2_u64.pow(32)));
		assert_eq!(config.column_map.get("m").unwrap().native_type,LONGTEXT(2_u64.pow(32)));
		assert_eq!(config.column_map.get("n").unwrap().native_type,FIXEDBINARY(1));
		assert_eq!(config.column_map.get("o").unwrap().native_type,FIXEDBINARY(50));

		config = s_config.get_table_config(&test_schema, &"numerics_signed".into()).unwrap();
		assert_eq!(config.column_map.get("a").unwrap().native_type,I64);
		assert_eq!(config.column_map.get("b").unwrap().native_type,U64);
		assert_eq!(config.column_map.get("c").unwrap().native_type,U64);
		assert_eq!(config.column_map.get("d").unwrap().native_type,I64);
		assert_eq!(config.column_map.get("e").unwrap().native_type,I64);
		assert_eq!(config.column_map.get("f").unwrap().native_type,U64);
		assert_eq!(config.column_map.get("g").unwrap().native_type,U64);
		assert_eq!(config.column_map.get("h").unwrap().native_type,I64);
		assert_eq!(config.column_map.get("i").unwrap().native_type,I64);
		assert_eq!(config.column_map.get("j").unwrap().native_type,U64);

	}

}
