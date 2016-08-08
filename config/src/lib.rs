extern crate xml;
use std::fs::File;
use std::io::Read;
use std::collections::HashMap;
use xml::Xml;

fn parse_config(path: &'static str) -> Config {
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
        // println!("{:?}", event);
        match event {
            Ok(e) => {
				//println!("HERE {:?}", e);
				match e {
					// TODO better way than match on all fields?
					xml::Element{name, ns, attributes, children, prefixes, default_ns} => {
						if name == "client-config" {
							parse_client_config(&mut b, children)
						} else {
							panic!("Unrecognized parent XML element {}", name)
						}
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
						//builder.add_schema(parse_schema_config(e.children));
						let sb = SchemaConfigBuilder::new();
						for c in e.children {

						}
						let schema = sb.build();
						builder.add_schema(schema);
					},
					_ => {}// dont care
				}
			},
			_ => {} // dont care
		}
	}
}

fn parse_schema_config(children: Vec<Xml>) -> SchemaConfig {
	panic!("Woah")
}

enum NativeType {
	BIGINT,
	VARCHAR
}

struct ColumnConfig {
	name: &'static str,
	encryption: &'static str,
	native_type: NativeType
}

struct TableConfig {
	name: &'static str,
	column_map: HashMap<&'static str, ColumnConfig>
}

struct TableConfigBuilder {
	column_map: HashMap<&'static str, ColumnConfig>,
	name: Option<&'static str>
}

impl TableConfigBuilder {
	fn new(self) -> TableConfigBuilder {
		TableConfigBuilder{column_map: HashMap::new(), name: None}
	}

	fn set_name(mut self, name: &'static str) -> TableConfigBuilder {
		self.name = Some(name);
		self
	}

	fn add_column(mut self, column: ColumnConfig) -> TableConfigBuilder {
		self.column_map.insert(column.name, column);
		self
	}

	fn build(mut self) -> TableConfig {
		TableConfig {name: self.name.unwrap(), column_map: self.column_map}
	}
}

struct SchemaConfig {
	name: &'static str,
	table_map: HashMap<&'static str, TableConfig>
}

struct SchemaConfigBuilder {
	name: Option<&'static str>,
	table_map: HashMap<&'static str, TableConfig>
}

impl SchemaConfigBuilder {
	fn new() -> SchemaConfigBuilder {
		SchemaConfigBuilder{name: None, table_map: HashMap::new()}
	}

	fn set_name(mut self, name: &'static str) -> SchemaConfigBuilder {
		self.name = Some(name);
		self
	}

	fn add_table(mut self, table: TableConfig) -> SchemaConfigBuilder {
		self.table_map.insert(table.name, table);
		self
	}

	fn build(mut self) -> SchemaConfig {
		SchemaConfig{name: self.name.unwrap(), table_map: self.table_map}
	}
}

struct ConnectionConfig {
	//TODO
}

struct Config {
	schema_map: HashMap<&'static str, SchemaConfig>,
	//connection_config : ConnectionConfig
}

struct ConfigBuilder {
	schema_map : HashMap<&'static str, SchemaConfig>
}

impl ConfigBuilder {
	fn new() -> ConfigBuilder {
		ConfigBuilder{schema_map: HashMap::new()}
	}

	fn add_schema(mut self, schema_config: SchemaConfig) -> ConfigBuilder {
		self.schema_map.insert(schema_config.name, schema_config);
		self
	}

	fn build(mut self) -> Config {
		Config {schema_map: self.schema_map}
	}
}

trait TConfig {
	fn get_column_config(schema: &'static str, table: &'static str, column: &'static str) -> Option<ColumnConfig>;
	fn get_table_config(schema: &'static str, table: &'static str) -> Option<ColumnConfig>;
	fn get_schema_config(schema: &'static str) -> Option<SchemaConfig>;

	fn get_connection_config();
}

trait TSchemaConfig {
	fn get_table_config(table: &'static str) -> Option<TableConfig>;
}

trait TTableConfig {
	fn get_column_config(column: &'static str) -> Option<ColumnConfig>;
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn config_test() {
		let config = super::parse_config("/Users/drewmanlove/codefutures/proofs-of-concept/osp-client/src/test/resources/demo-client-config.xml");

	}
}
