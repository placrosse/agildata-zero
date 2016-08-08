extern crate xml;
use std::fs::File;
use std::io::Read;
use std::collections::HashMap;
use xml::Xml;

pub fn parse_config(path: &'static str) -> Config {
	let mut rdr = match File::open(path) {
        Ok(file) => file,
        Err(err) => {
            println!("Couldn't open file: {}", err);
            std::process::exit(1);
        }
    };

	let mut p = xml::Parser::new();
    let mut e = xml::ElementBuilder::new();
	let mut b = ConfigBuilder::new();

    let mut string = String::new();
    if let Err(err) = rdr.read_to_string(&mut string) {
        println!("Reading failed: {}", err);
        std::process::exit(1);
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
            Err(e) => println!("{}", e),
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
					"connection" => {//TODO
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
	for node in children {
		match node {
			Xml::ElementNode(e) => match &e.name as &str {
				"column" => {
					let name = get_attr_or_fail("name", &e);
					let native_type = get_attr_or_fail("type", &e);
					let encryption = get_attr_or_fail("encryption", &e);
					builder.add_column(ColumnConfig{
						name: name,
<<<<<<< HEAD
						encryption: determine_encryption(&native_type),
						native_type: native_type
=======
						encryption: determine_encryption(&encryption),
						native_type: determine_native_type(&native_type)
>>>>>>> master
					});

				},
				_ => panic!("Unexpected element tag {}", e.name)
			},
			_ => {} // dont' care yet
		}
	}
}

fn get_attr_or_fail(name: &'static str, element: &xml::Element) -> String {
	match element.get_attribute(name, None) {
		Some(v) => v.to_string(),
		None => panic!("Missing attribute {}", name)
	}
}

<<<<<<< HEAD
fn determine_encryption(native_type: &String) -> EncryptionType {
	if native_type.contains("VARCHAR") {
		EncryptionType::Varchar(50) // TODO hard coded display..
	} else {
		match native_type as &str {
			"INTEGER" => EncryptionType::U64,
			"DOUBLE" => EncryptionType::F64,
=======
fn determine_native_type(native_type: &String) -> NativeType {
	if native_type.contains("VARCHAR") {
		NativeType::Varchar(50) // TODO hard coded display..
	} else {
		match native_type as &str {
			"INTEGER" => NativeType::U64,
			"DOUBLE" => NativeType::F64,
>>>>>>> master
			_ => panic!("Unsupported native type {}", native_type)
		}
	}
}

<<<<<<< HEAD
#[derive(Debug)]
pub enum EncryptionType {
	U64,
	Varchar(u32),
	F64,
}

#[derive(Debug)]
pub struct ColumnConfig {
	pub name: String,
	pub encryption: EncryptionType,
	pub native_type: String
}

#[derive(Debug)]
=======
fn determine_encryption(encryption: &String) -> EncryptionType {
	match &encryption.to_uppercase() as &str {
		"AES" => EncryptionType::AES,
		"AES-SALTED" => EncryptionType::AES_SALT,
		"OPE" => EncryptionType::OPE,
		"NONE" => EncryptionType::NA,
		_ => panic!("Unsupported encryption type {}", encryption)
	}

}

#[derive(Debug)]
pub enum EncryptionType {
	AES,
	AES_SALT,
	OPE,
	NA,
}

#[derive(Debug)]
pub enum NativeType {
	U64,
	Varchar(u32),
	F64,
}

#[derive(Debug)]
pub struct ColumnConfig {
	pub name: String,
	pub encryption: EncryptionType,
	pub native_type: NativeType
}

#[derive(Debug)]
>>>>>>> master
pub struct TableConfig {
	name: String,
	column_map: HashMap<String, ColumnConfig>
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

	fn build(mut self) -> TableConfig {
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

	fn build(mut self) -> SchemaConfig {
		SchemaConfig{name: self.name.unwrap(), table_map: self.table_map}
	}
}

pub struct ConnectionConfig {
	//TODO
}

#[derive(Debug)]
pub struct Config {
	schema_map: HashMap<String, SchemaConfig>,
	//connection_config : ConnectionConfig
}

struct ConfigBuilder {
	schema_map : HashMap<String, SchemaConfig>
}

impl ConfigBuilder {
	fn new() -> ConfigBuilder {
		ConfigBuilder{schema_map: HashMap::new()}
	}

	fn add_schema(&mut self, schema: SchemaConfig) {
		let key = schema.name.clone(); // TODO downcase
		self.schema_map.insert(key, schema);
	}

	fn build(mut self) -> Config {
		Config {schema_map: self.schema_map}
	}
}

pub trait TConfig {
	fn get_column_config(&self, schema: &'static str, table: &'static str, column: &'static str) -> Option<ColumnConfig>;
	fn get_table_config(&self, schema: &'static str, table: &'static str) -> Option<ColumnConfig>;
	fn get_schema_config(&self, schema: &'static str) -> Option<SchemaConfig>;

	fn get_connection_config(&self);
}

impl TConfig for Config {

	fn get_column_config(&self, schema: &'static str, table: &'static str, column: &'static str) -> Option<ColumnConfig> {
		panic!("get_column_config() not implemented");
	}

	fn get_table_config(&self, schema: &'static str, table: &'static str) -> Option<ColumnConfig> {
		panic!("get_table_config() not implemented");
	}

	fn get_schema_config(&self, schema: &'static str) -> Option<SchemaConfig> {
		panic!("get_table_config() not implemented");
	}

	fn get_connection_config(&self) {
		panic!("get_table_config() not implemented");
	}

}

pub trait TSchemaConfig {
	fn get_table_config(table: &'static str) -> Option<TableConfig>;
}

pub trait TTableConfig {
	fn get_column_config(column: &'static str) -> Option<ColumnConfig>;
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn config_test() {
		let config = super::parse_config("./src/demo-client-config.xml");
		println!("Config {:#?}", config);
	}
}
