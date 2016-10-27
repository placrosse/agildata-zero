
use rustc_serialize::Decodable;
use rustc_serialize::Decoder;
use toml;
use std::env;
use std::str::FromStr;

use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub struct Config {
    client: ClientConfig,
    connection: ConnectionConfig,
    parsing: ParsingConfig,
    schemas: HashMap<String, SchemaConfig>,
}

#[derive(Debug, PartialEq)]
pub struct ConnectionConfig {
    host: String,
    user: String,
    password: String
}

#[derive(Debug, PartialEq)]
pub struct ClientConfig {
    host: String,
    port: String
}

#[derive(Debug, PartialEq)]
pub struct ParsingConfig {
    mode: Mode
}

#[derive(Debug, PartialEq)]
pub struct SchemaConfig {
    name: String,
    tables: HashMap<String, TableConfig>
}

#[derive(Debug, PartialEq)]
pub struct TableConfig {
    name: String,
    columns: HashMap<String, ColumnConfig>
}

#[derive(Debug, PartialEq)]
pub struct ColumnConfig {
    name: String,
    native_type: String,
    encryption: String
}

trait Builder {
    type Output;
    fn new() -> Self;
    fn build(self) -> Result<Self::Output, String>;
    fn merge(&mut self, b: Self);
}

#[derive(Debug)]
struct ConfigBuilder {
    client: ClientConfigBuilder,
    connection: ConnectionConfigBuilder,
    parsing: ParsingConfigBuilder,
    schemas: SchemaMapBuilder
}


impl Builder for ConfigBuilder {
    type Output = Config;

    fn build(self) -> Result<Self::Output, String> {
        Ok(Config{
            client: self.client.build()?,
            connection: self.connection.build()?,
            parsing: self.parsing.build()?,
            schemas: self.schemas.build()?
        })
    }

    fn merge(&mut self, b: ConfigBuilder) {
        self.client.merge(b.client);
        self.connection.merge(b.connection);
        self.parsing.merge(b.parsing);
        self.schemas.merge(b.schemas);
    }

    fn new() -> Self {
        ConfigBuilder {
            client: ClientConfigBuilder::new(),
            connection: ConnectionConfigBuilder::new(),
            parsing: ParsingConfigBuilder::new(),
            schemas: SchemaMapBuilder::new()
        }
    }
}

impl Decodable for ConfigBuilder {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        d.read_struct("ConfigBuilder", 4, |_d| -> _ {
            Ok(
                ConfigBuilder {
                    client: _d.read_struct_field("client", 2, Decodable::decode)?,
                    connection: _d.read_struct_field("connection", 3, Decodable::decode)?,
                    parsing: _d.read_struct_field("parsing", 1, Decodable::decode)?,
                    schemas: _d.read_map(SchemaMapBuilder::decode)?
                }
            )
        })
    }
}

#[derive(Debug)]
struct ConnectionConfigBuilder {
    host: Option<String>,
    user: Option<String>,
    password: Option<String>
}

fn missing_err(prop: &str) -> String {
    format!("Missing required property {}", prop)
}

impl Builder for ConnectionConfigBuilder {
    type Output = ConnectionConfig;
    fn build(self) -> Result<Self::Output, String> {

        Ok(ConnectionConfig {
            host: self.host.ok_or(missing_err(&"connection.host"))?.resolve()?,
            user: self.user.ok_or(missing_err(&"connection.user"))?.resolve()?,
            password: self.password.ok_or(missing_err(&"connection.password"))?.resolve()?
        })
    }

    fn merge(&mut self, b: ConnectionConfigBuilder) {
        if b.host.is_some() {self.host = b.host}
        if b.user.is_some() {self.user = b.user}
        if b.password.is_some() {self.password = b.password}
    }

    fn new() -> Self {
        ConnectionConfigBuilder {
            host: None,
            user: None,
            password: None
        }
    }
}

impl Decodable for ConnectionConfigBuilder {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        d.read_struct("ConnectionConfig", 3, |_d| -> _ {
            Ok(
                ConnectionConfigBuilder {
                    host: _d.read_struct_field("host", 0, Decodable::decode)?,
                    user: _d.read_struct_field("user", 1, Decodable::decode)?,
                    password: _d.read_struct_field("password", 0, Decodable::decode)?
                }
            )
        })
    }
}


#[derive(Debug)]
struct ClientConfigBuilder {
    host: Option<String>,
    port: Option<String>
}

impl Builder for ClientConfigBuilder {
    type Output = ClientConfig;

    fn build(self) -> Result<Self::Output, String> {
        Ok(ClientConfig {
            host: self.host.ok_or(missing_err(&"client.host"))?.resolve()?,
            port: self.port.ok_or(missing_err(&"client.port"))?.resolve()?
        })
    }

    fn merge(&mut self, b: ClientConfigBuilder) {
        if b.host.is_some() {self.host = b.host}
        if b.port.is_some() {self.port = b.port}
    }

    fn new() -> Self {
        ClientConfigBuilder {
            host: None,
            port: None
        }
    }
}

impl Decodable for ClientConfigBuilder {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        d.read_struct("ClientConfigBuilder", 2, |_d| -> _ {
            Ok(
                ClientConfigBuilder {
                    host: _d.read_struct_field("host", 0, Decodable::decode)?,
                    port: _d.read_struct_field("port", 1, Decodable::decode)?,
                }
            )
        })
    }
}

#[derive(Debug, RustcDecodable)]
struct ParsingConfigBuilder {
    mode: Option<String>
}

#[derive(Debug, PartialEq)]
enum Mode {
    STRICT,
    PERMISSIVE
}

impl FromStr for Mode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match &s.to_lowercase() as &str {
            "strict" => Ok(Mode::STRICT),
            "permissive" => Ok(Mode::PERMISSIVE),
            a => Err(format!("Unknown parsing mode {}", a))
        }
    }
}

impl Builder for ParsingConfigBuilder {
    type Output = ParsingConfig;

    fn build(self) -> Result<Self::Output, String> {
        Ok(ParsingConfig{
            mode: Mode::from_str(&self.mode.ok_or(missing_err(&"parsing.mode"))?.resolve()?)?
        })
    }

    fn merge(&mut self, b: Self) {
        if b.mode.is_some() {self.mode = b.mode}
    }

    fn new() -> Self {
        ParsingConfigBuilder {
            mode: None
        }
    }
}

#[derive(Debug)]
struct SchemaMapBuilder {
    schemas: HashMap<String, SchemaConfigBuilder>
}

impl Builder for SchemaMapBuilder {
    type Output = HashMap<String, SchemaConfig>;

    fn new() -> Self {
        SchemaMapBuilder{
            schemas: HashMap::new()
        }
    }

    fn build(self) -> Result<Self::Output, String> {
        // TODO avoid clone
        self.schemas.iter()
        .map(|(k, v)| fold_tuple(k.clone(), v.clone()))
        .collect::<Result<HashMap<String, SchemaConfig>, String>>()
    }

    fn merge(&mut self, b: Self) {
        for (k, v) in b.schemas {
            if self.schemas.contains_key(&k) {
                self.schemas.get_mut(&k).unwrap().merge(v)
            } else {
                self.schemas.insert(k, v);
            }
        }
    }
}

impl SchemaMapBuilder {
    fn decode<D:Decoder>(d: &mut D, l: usize) -> Result<SchemaMapBuilder, D::Error> {
        let mut schema_map: HashMap<String, SchemaConfigBuilder> = HashMap::new();
        for i in 0..l {
            let schema = d.read_map_elt_key(i, decode_key_name)?;
            let mut conf: SchemaConfigBuilder = d.read_map_elt_val(i, Decodable::decode)?;

            conf.name = Some(schema.clone());

            //conf.set_name(&schema);
            schema_map.insert(schema, conf);
        }

        Ok(SchemaMapBuilder{schemas: schema_map})
    }
}

#[derive(Debug, Clone)]
struct SchemaConfigBuilder {
    name: Option<String>,
    tables: HashMap<String, TableConfigBuilder>
}

impl Builder for SchemaConfigBuilder {
    type Output = SchemaConfig;

    fn build(self) -> Result<Self::Output, String> {
        Ok(SchemaConfig {
            name: self.name.ok_or(String::from("Illegal: Missing Schema name"))?,
            // TODO avoid clone
            tables: self.tables.iter()
                .map(|(k,v)| fold_tuple(k.clone(), v.clone()))
                .collect::<Result<HashMap<String, TableConfig>, String>>()?
        })
    }

    fn merge(&mut self, b: Self) {
        for (k, v) in b.tables {
            if self.tables.contains_key(&k) {
                self.tables.get_mut(&k).unwrap().merge(v)
            } else {
                self.tables.insert(k, v);
            }
        }
    }

    fn new() -> Self {
        SchemaConfigBuilder {
            name: None,
            tables: HashMap::new()
        }
    }
}

impl Decodable for SchemaConfigBuilder {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        let mut table_map: HashMap<String, TableConfigBuilder> = HashMap::new();
        d.read_map(|_d, _l| -> _ {
            for i in 0.._l {
                let table = _d.read_map_elt_key(i,decode_key_name)?;
                let mut table_conf: TableConfigBuilder = _d.read_map_elt_val(i, Decodable::decode)?;

                table_conf.name = Some(table.clone());

                table_map.insert(table.to_lowercase(), table_conf);
            }
            Ok(SchemaConfigBuilder{
                name: None, // Not known here
                tables: table_map
            })
        })
    }
}


#[derive(Debug, Clone)]
struct TableConfigBuilder {
    name: Option<String>,
    columns: HashMap<String, ColumnConfigBuilder>
}

impl Builder for TableConfigBuilder {
    type Output = TableConfig;

    fn build(self) -> Result<Self::Output, String> {
        Ok(TableConfig {
            name: self.name.ok_or(String::from("Illegal: missing table name"))?,
            // TODO avoid clone
            columns: self.columns.iter()
                .map(|(k,v)| fold_tuple(k.clone(), v.clone()))
                .collect::<Result<HashMap<String, ColumnConfig>, String>>()?
        })
    }

    fn merge(&mut self, b: Self) {
        for (k, v) in b.columns {
            if self.columns.contains_key(&k) {
                self.columns.get_mut(&k).unwrap().merge(v)
            } else {
                self.columns.insert(k, v);
            }
        }
    }

    fn new() -> Self {
        TableConfigBuilder {
            name: None,
            columns: HashMap::new()
        }
    }
}


impl Decodable for TableConfigBuilder {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        let mut column_map: HashMap<String, ColumnConfigBuilder> = HashMap::new();

        d.read_map(|_d, _l| -> _ {
            for i in 0.._l {
                let column = _d.read_map_elt_key(i,decode_key_name)?;
                let mut column_conf: ColumnConfigBuilder = _d.read_map_elt_val(i, Decodable::decode)?;

                column_conf.name = Some(column.clone());

                column_map.insert(column.to_lowercase(), column_conf);
            }

            Ok(TableConfigBuilder{name: None, columns: column_map})
        })
    }
}

#[derive(Debug, Clone)]
struct ColumnConfigBuilder {
    name: Option<String>,
    native_type: Option<String>,
    encryption: Option<String>
}

impl Builder for ColumnConfigBuilder {
    type Output = ColumnConfig;

    fn build(self) -> Result<Self::Output, String> {
        Ok(ColumnConfig {
            name: self.name.ok_or(String::from("Illegal: missing table name"))?,
            native_type: self.native_type.unwrap(),
            encryption: self.encryption.unwrap()
        })
    }

    fn merge(&mut self, b: Self) {
        if b.native_type.is_some() {self.native_type = b.native_type}
        if b.encryption.is_some() {self.encryption = b.encryption}
    }

    fn new() -> Self {
        ColumnConfigBuilder {
            name: None,
            native_type: None,
            encryption: None
        }
    }
}

impl Decodable for ColumnConfigBuilder {

    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        d.read_struct("ColumnConfigBuilder", 1, |_d| -> _ {
            Ok(
                ColumnConfigBuilder {
                    name: None, // not known here
                    native_type: _d.read_struct_field("type", 0, Decodable::decode)?,
                    encryption: _d.read_struct_field("encryption", 0, Decodable::decode)?,
                }
            )
        })
    }
}

// For use in map Hashmap to produce correct result structure
fn fold_tuple<V: Builder>(k: String, v: V) -> Result<(String, V::Output), String> {
    Ok((k, v.build()?))
}

fn decode_key_name<D:Decoder>(d: &mut D) -> Result<String, D::Error> {
    d.read_str()
}

trait Resolvable {
    type Output;
    fn resolve(self) -> Result<Self::Output, String>;
}

impl Resolvable for String {
    type Output = String;

    fn resolve(self) -> Result<Self::Output, String> {
        let resolved = if self.starts_with("${") && self.ends_with("}") {
            let env_var =&self[2..(self.len() - 1)];
            match  env::var(env_var) {
                Ok(v) => v,
                Err(e) => return Err(format!("Cannot resolve environment variable {}", env_var))
            }
        } else {
            self
        };

        Ok(resolved)
    }
}

#[derive(Debug)]
struct ResolvedString {
    value: String
}

impl Decodable for ResolvedString {
    fn decode<D: Decoder>(d: &mut D) -> Result<ResolvedString, D::Error> {
        let val = d.read_str()?;
        let resolved = if val.starts_with("${") && val.ends_with("}") {
            let env_var =&val[2..(val.len() - 1)];
            match  env::var(env_var) {
                Ok(v) => v,
                Err(e) => return Err(d.error(&format!("Cannot resolve environment variable {}", env_var)))
            }
        } else {
            val
        };

        Ok(ResolvedString {value: resolved})
    }
}

#[cfg(test)]
mod test {

    use super::{Config, ClientConfig, SchemaConfig, ConnectionConfig, ParsingConfig, TableConfig, ColumnConfig, Mode, ConfigBuilder, Builder};
    use toml;

    use std::collections::HashMap;

    use std::env;

    #[test]
    fn test_builder_toml() {
        let toml_str = r#"
        [client]
        host = "${ENV_VAR}"
        port = "3307"

        [connection]
        user = "${MY_USER}"
        password = "${MY_PASS}"
        host = "127.0.0.1"

        [parsing]
        mode = "permissive"

        [zero.users.id]
        type="INTEGER"
        encryption="NONE"

        "#;

        env::set_var("ENV_VAR", "127.0.0.1");
        env::set_var("MY_USER", "agiluser");
        env::set_var("MY_PASS", "password123");

        let toml = toml::Parser::new(toml_str).parse().unwrap();

        let mut decoder = toml::Decoder::new(toml::Value::Table(toml));
        let decoded: ConfigBuilder = ::rustc_serialize::Decodable::decode(&mut decoder).unwrap();
        println!("{:#?}", decoded);

        println!("#{:#?}", decoder.toml);

        let config = decoded.build().unwrap();
        assert_eq!(config.client, ClientConfig {
                    host: "127.0.0.1".into(),
                    port: "3307".into()
        });

        assert_eq!(config.connection, ConnectionConfig {
                    host: "127.0.0.1".into(),
                    user: "agiluser".into(),
                    password: "password123".into()
        });

        assert_eq!(config.parsing, ParsingConfig {
                    mode: Mode::PERMISSIVE
        });

        let expected_column = ColumnConfig {
            name: "id".into(),
            native_type: "INTEGER".into(),
            encryption: "NONE".into()
        };

        let mut expected_column_map: HashMap<String, ColumnConfig> = HashMap::new();
        expected_column_map.insert("id".into(), expected_column);
        let expected_table_conf = TableConfig{name: "users".into(), columns: expected_column_map};

        let mut expected_table_map: HashMap<String, TableConfig> = HashMap::new();
        expected_table_map.insert("users".into(), expected_table_conf);

        let mut expected_schema: HashMap<String, SchemaConfig> = HashMap::new();
        expected_schema.insert("zero".into(), SchemaConfig{name: "zero".into(), tables: expected_table_map});

        assert_eq!(config.schemas, expected_schema);

    }

}