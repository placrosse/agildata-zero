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
use rustc_serialize::Decodable;
use rustc_serialize::Decoder;
use toml;
use std::env;
use std::str::FromStr;
use std::path::Path;
use std::fs::{File, read_dir};
use std::io::{Read, Error};

use std::collections::HashMap;

use encrypt::*;
use proxy::server::ParsingMode;

use query::{Tokenizer, ASTNode, MySQLColumnQualifier};
use query::MySQLDataType::*;
use query::dialects::ansisql::*;
use query::dialects::mysqlsql::*;

use error::ZeroError;
use std::process;


/// parses a default config and any configs contained within an override directory and reconciles to one Config
pub fn parse_configs(default_path: &str, dir: &str) -> Config {
    debug!("parse_configs() default: {}, override dir: {}", default_path, dir);
    let mut toml_str = _load_toml_file(default_path);

    let mut toml = toml::Parser::new(&toml_str).parse().unwrap();

    let mut decoder = toml::Decoder::new(toml::Value::Table(toml));
    let mut decoded: ConfigBuilder = ::rustc_serialize::Decodable::decode(&mut decoder).unwrap();

    // If override dir exists, load any available configs
    if Path::new(dir).exists() {
        let paths = read_dir(dir).unwrap();
        for p in paths {
            let _p = p.unwrap().path();
            let path = _p.to_str().unwrap();
            if path.ends_with(".toml") {
                toml_str = _load_toml_file(path);
                toml = toml::Parser::new(&toml_str).parse().unwrap();
                decoder = toml::Decoder::new(toml::Value::Table(toml));
                let d: ConfigBuilder = ::rustc_serialize::Decodable::decode(&mut decoder).unwrap();
                decoded.merge(d)
            }
        }
    }

    decoded.build().unwrap() // TODO
}

// read from file to string
fn _load_toml_file(path: &str) -> String {
    let mut rdr = match File::open(path) {
        Ok(file) => file,
        Err(err) => {
            println!("Unable to open configuration file '{}': {}", path, err);
            process::exit(1);
        }
    };

    let mut string = String::new();
    if let Err(err) = rdr.read_to_string(&mut string) {
        error!("Reading failed: {}", err);
        process::exit(1);
    };

    string
}

#[derive(Debug, PartialEq)]
pub struct Config {
    pub client: ClientConfig,
    pub connection: ConnectionConfig,
    pub parsing: ParsingConfig,
    schemas: HashMap<String, SchemaConfig>,
}

#[derive(Debug, PartialEq)]
pub struct ConnectionConfig {
    pub host: String,
    pub user: String,
    pub password: String,
    // optional properties
    // defaults set in build phase
    pub port: String,
}

#[derive(Debug, PartialEq)]
pub struct ClientConfig {
    pub host: String,
    pub port: String
}

#[derive(Debug, PartialEq)]
pub struct ParsingConfig {
    pub mode: ParsingMode
}

#[derive(Debug, PartialEq)]
pub struct SchemaConfig {
    pub name: String,
    tables: HashMap<String, TableConfig>
}

#[derive(Debug, PartialEq)]
pub struct TableConfig {
    pub name: String,
    columns: HashMap<String, ColumnConfig>
}

#[derive(Debug, PartialEq)]
pub struct ColumnConfig {
    pub name: String,
    pub native_type: NativeType,
    pub encryption: EncryptionType,
    pub key: [u8; 32]
}

// Supported qualifiers
#[derive(Debug, PartialEq)]
pub enum NativeTypeQualifier {
    SIGNED,
    UNSIGNED,
    OTHER
}

/// trait to access properties with a Config
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
        self.schemas.get(schema)
    }

    fn get_connection_config(&self) -> &ConnectionConfig {
        &self.connection
    }

    fn get_client_config(&self) -> &ClientConfig {
        &self.client
    }

    fn get_parsing_config(&self) -> &ParsingConfig {
        &self.parsing
    }

}

pub trait TSchemaConfig {
    fn get_table_config(&self, table: &String) -> Option<&TableConfig>;
}

impl TSchemaConfig for SchemaConfig {
    fn get_table_config(&self, table: &String) -> Option<&TableConfig> {
        self.tables.get(table)
    }
}

pub trait TTableConfig {
    fn get_column_config(&self, column: &String) -> Option<&ColumnConfig>;
}

impl TTableConfig for TableConfig {
    fn get_column_config(&self, column: &String) -> Option<&ColumnConfig> {
        self.columns.get(column)
    }
}


// Private methods

// generic builder trait
trait Builder {
    type Output;

    fn new() -> Self;
    // builds and validates
    fn build(self) -> Result<Self::Output, String>;

    // merges this builder with another
    // the other overrides this
    fn merge(&mut self, b: Self);
}

#[derive(Debug)]
struct ConfigBuilder {
    client: Option<ClientConfigBuilder>,
    connection: Option<ConnectionConfigBuilder>,
    parsing: Option<ParsingConfigBuilder>,
    schemas: SchemaMapBuilder
}


impl Builder for ConfigBuilder {
    type Output = Config;

    fn build(self) -> Result<Self::Output, String> {
        Ok(Config{
            client: self.client.ok_or(missing_err("client"))?.build()?,
            connection: self.connection.ok_or(missing_err("connection"))?.build()?,
            parsing: self.parsing.ok_or(missing_err("parsing"))?.build()?,
            schemas: self.schemas.build()?
        })
    }

    fn merge(&mut self, b: ConfigBuilder) {
        if let Some(ref mut s) = self.client {
            if let Some(o) = b.client {
                s.merge(o)
            }
        } else {
            self.client = b.client
        }

        if let Some(ref mut s) = self.connection {
            if let Some(o) = b.connection {
                s.merge(o)
            }
        } else {
            self.connection = b.connection
        }

        if let Some(ref mut s) = self.parsing {
            if let Some(o) = b.parsing {
                s.merge(o)
            }
        } else {
            self.parsing = b.parsing
        }

        self.schemas.merge(b.schemas);
    }

    fn new() -> Self {
        ConfigBuilder {
            client: Some(ClientConfigBuilder::new()),
            connection: Some(ConnectionConfigBuilder::new()),
            parsing: Some(ParsingConfigBuilder::new()),
            schemas: SchemaMapBuilder::new()
        }
    }
}

// Decodable implementation for toml-rs
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
    password: Option<String>,
    port: Option<String>
}

impl Builder for ConnectionConfigBuilder {
    type Output = ConnectionConfig;
    fn build(self) -> Result<Self::Output, String> {

        Ok(ConnectionConfig {
            host: self.host.ok_or(missing_err(&"connection.host"))?.resolve()?,
            user: self.user.ok_or(missing_err(&"connection.user"))?.resolve()?,
            password: self.password.ok_or(missing_err(&"connection.password"))?.resolve()?,
            port: self.port.unwrap_or("3306".into()).resolve()?
        })
    }

    fn merge(&mut self, b: ConnectionConfigBuilder) {
        if b.host.is_some() {self.host = b.host}
        if b.user.is_some() {self.user = b.user}
        if b.password.is_some() {self.password = b.password}
        if b.port.is_some() {self.port = b.port}
    }

    fn new() -> Self {
        ConnectionConfigBuilder {
            host: None,
            user: None,
            password: None,
            port: None
        }
    }
}


// Decodable impl for toml-rs
impl Decodable for ConnectionConfigBuilder {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        d.read_struct("ConnectionConfig", 3, |_d| -> _ {
            Ok(
                ConnectionConfigBuilder {
                    host: _d.read_struct_field("host", 0, Decodable::decode)?,
                    user: _d.read_struct_field("user", 1, Decodable::decode)?,
                    password: _d.read_struct_field("password", 2, Decodable::decode)?,
                    port: _d.read_struct_field("port", 3, Decodable::decode)?,
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


// Decodable impl for toml-rs
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


impl Builder for ParsingConfigBuilder {
    type Output = ParsingConfig;

    fn build(self) -> Result<Self::Output, String> {
        Ok(ParsingConfig{
            mode: ParsingMode::from_str(&self.mode.ok_or(missing_err(&"parsing.mode"))?.resolve()?)?
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

    // requires custom decode function for read_map
    // this is due to use of arbitrary table names in the toml
    // [somechema.sometable.somecolumn]
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

// Decodable impl for toml-rs
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

// Decodable impl for toml-rs
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
    encryption: Option<String>,
    key: Option<String>,
    iv: Option<String>
}

// TODO errors
impl Builder for ColumnConfigBuilder {
    type Output = ColumnConfig;

    fn build(self) -> Result<Self::Output, String> {
        // Build and validate
        let name = self.name.ok_or(String::from("Illegal: missing table name"))?;
        let native_type = determine_native_type(
            &self.native_type.ok_or(
                missing_err(&format!("{}.native_type", name))
            )?
        ).map_err(|e| String::from("TODO"))?;

        let iv = match self.iv {
            Some(hex) => Some(hex_to_iv(&hex.resolve()?)),
            None => None
        };
        let encryption = determine_encryption(
            &self.encryption.unwrap_or(String::from("NONE")),
            iv
        ).map_err(|e| String::from("TODO"))?;

        let key = if self.key.is_some() {
            hex_key(&self.key.unwrap().resolve()?)
        } else {
            if encryption == EncryptionType::NA {
                [0u8;32]
            } else {
                return Err(format!("Column {}, encryption: {:?}, requires key property", name, encryption))
            }
        };

        Ok(ColumnConfig {
            name: name,
            native_type: native_type,
            encryption: encryption,
            key: key
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
            encryption: None,
            key: None,
            iv: None
        }
    }
}

// Decodable impl for toml-rs
impl Decodable for ColumnConfigBuilder {

    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        d.read_struct("ColumnConfigBuilder", 4, |_d| -> _ {
            Ok(
                ColumnConfigBuilder {
                    name: None, // not known here
                    native_type: _d.read_struct_field("type", 0, Decodable::decode)?,
                    encryption: _d.read_struct_field("encryption", 1, Decodable::decode)?,
                    key: _d.read_struct_field("key", 2, Decodable::decode)?,
                    iv: _d.read_struct_field("iv", 3, Decodable::decode)?
                }
            )
        })
    }
}

// For use in map Hashmap to produce correct result structure
fn fold_tuple<V: Builder>(k: String, v: V) -> Result<(String, V::Output), String> {
    Ok((k, v.build()?))
}

// read key string as part of decode
fn decode_key_name<D:Decoder>(d: &mut D) -> Result<String, D::Error> {
    d.read_str()
}

// shorthand helper method for err msg
fn missing_err(prop: &str) -> String {
    format!("Missing required property {}", prop)
}

// TODO errors
fn determine_encryption(encryption: &String, iv: Option<[u8;12]>) -> Result<EncryptionType, Box<ZeroError>> {
    match &encryption.to_uppercase() as &str {
        "AES" => {
            match iv {
                Some(nonce)=> Ok(EncryptionType::Aes(nonce)),
                None => panic!("iv attribute required for AES encryption")
            }
        },
        "AES_GCM" => Ok(EncryptionType::AesGcm),
        "NONE" => Ok(EncryptionType::NA),
        _ => panic!("Unsupported encryption type {}", encryption)
    }

}

// reconcile parsed type AST to encrypt::NativeType
pub fn reconcile_native_type(data_type: &ASTNode, qualifiers: &Vec<NativeTypeQualifier>) -> Result<NativeType, Box<ZeroError>> {
    Ok(match data_type {
        &ASTNode::MySQLDataType(ref dt) => match dt {
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
            &Binary{ref length} | &CharByte{ref length} => NativeType::FIXEDBINARY(length.unwrap_or(1)),
            &VarBinary{ref length} => NativeType::VARBINARY(match length {
                &Some(l) => l,
                &None => return Err(ZeroError::SchemaError{message: "VARBINARY requires length argument".into(), code: "123".into()}.into())
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

// Should fail in config parse, but not as part of get table meta
pub fn reconcile_column_qualifiers(qualifiers: &Vec<ASTNode>, fail: bool) -> Result<Vec<NativeTypeQualifier>, Box<ZeroError>> {
    // Iterate over qualifiers and propagate error on unsupported
    // potential support could be DEFAULT, [NOT] NULL, etc
    qualifiers
        .iter().map(|o| {
        match o {
            &ASTNode::MySQLColumnQualifier(ref q) => match q {
                &MySQLColumnQualifier::Signed => Ok(NativeTypeQualifier::SIGNED),
                &MySQLColumnQualifier::Unsigned => Ok(NativeTypeQualifier::UNSIGNED),
                _ => if fail {
                    Err(ZeroError::SchemaError{message: format!("Unsupported data type qualifier {:?}", q), code: "123".into()}.into())
                } else {
                    Ok(NativeTypeQualifier::OTHER)
                }
            },
            _ => Err(ZeroError::SchemaError{message: format!("Unsupported option {:?}", o), code: "123".into()}.into())
        }
    }).collect::<Result<Vec<NativeTypeQualifier>, Box<ZeroError>>>()
}

fn determine_native_type(native_type: &String) -> Result<NativeType, Box<ZeroError>> {
    let ansi = AnsiSQLDialect::new();
    let dialect = MySQLDialect::new(&ansi);
    let tokens = native_type.tokenize(&dialect).unwrap();
    let data_type = dialect.parse_data_type(&tokens)?;
    let parsed_qs = dialect.parse_column_qualifiers(&tokens)?.unwrap_or(vec![]);
    let qualifiers = reconcile_column_qualifiers(&parsed_qs, true)?;
    reconcile_native_type(&data_type, &qualifiers)
}


trait Resolvable {
    type Output;
    fn resolve(self) -> Result<Self::Output, String>;
}


// Resolves a possible variable string to env value
// i.e. ${ENV_VAR}
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

#[cfg(test)]
mod test {

    use super::{Config, ClientConfig, SchemaConfig, ConnectionConfig, ParsingConfig,
        TableConfig, ColumnConfig, ConfigBuilder, Builder, TConfig};
    use proxy::server::ParsingMode;
    use toml;

    use std::collections::HashMap;

    use std::env;

    use encrypt::*;
    use encrypt::NativeType::*;
    use encrypt::EncryptionType::*;

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

        [zero.users.first_name]
        type="VARCHAR(50)"
        encryption="AES"
        key="${ZERO_USERS_FIRST_NAME_KEY}"
        iv="${ZERO_USERS_FIRST_NAME_IV}"

        "#;

        env::set_var("ENV_VAR", "127.0.0.1");
        env::set_var("MY_USER", "agiluser");
        env::set_var("MY_PASS", "password123");
        let zero_users_first_name_key = "44E6884D78AA18FA690917F84145AA4415FC3CD560915C7AE346673B1FDA5985";
        env::set_var("ZERO_USERS_FIRST_NAME_KEY", zero_users_first_name_key);
        let zero_user_first_name_iv = "03F72E7479F3E34752E4DD91";
        env::set_var("ZERO_USERS_FIRST_NAME_IV", zero_user_first_name_iv);

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
                    password: "password123".into(),
                    port: "3306".into() // default
        });

        assert_eq!(config.parsing, ParsingConfig {
                    mode: ParsingMode::Permissive
        });

        let expected_column = ColumnConfig {
            name: "id".into(),
            native_type: NativeType::U64,
            encryption: EncryptionType::NA,
            key: [0_u8; 32]
        };

        let mut expected_column_map: HashMap<String, ColumnConfig> = HashMap::new();
        expected_column_map.insert("id".into(), expected_column);

        let expected_column = ColumnConfig {
            name: "first_name".into(),
            native_type: NativeType::Varchar(50),
            encryption: EncryptionType::Aes(hex_to_iv(&zero_user_first_name_iv)),
            key: hex_key(zero_users_first_name_key)
        };
        expected_column_map.insert("first_name".into(), expected_column);

        let expected_table_conf = TableConfig{name: "users".into(), columns: expected_column_map};

        let mut expected_table_map: HashMap<String, TableConfig> = HashMap::new();
        expected_table_map.insert("users".into(), expected_table_conf);

        let mut expected_schema: HashMap<String, SchemaConfig> = HashMap::new();
        expected_schema.insert("zero".into(), SchemaConfig{name: "zero".into(), tables: expected_table_map});

        assert_eq!(config.schemas, expected_schema);

    }

    #[test]
    fn test_config_data_types() {
        env::set_var("TEST_SHARED_KEY", "44E6884D78AA18FA690917F84145AA4415FC3CD560915C7AE346673B1FDA5985");
        env::set_var("TEST_SHARED_IV", "03F72E7479F3E34752E4DD91");

        let s_config = super::parse_configs("src/test/test-zero-config.toml", "/etc/zero.d");
        let test_schema = "zero".into();
        // Numerics

        let mut config = s_config.get_table_config(&test_schema, &"numerics".into()).unwrap();
        assert_eq!(config.columns.get("a").unwrap().native_type,U64);
        assert_eq!(config.columns.get("b").unwrap().native_type,U64);
        assert_eq!(config.columns.get("c").unwrap().native_type,U64);
        assert_eq!(config.columns.get("d").unwrap().native_type,U64);
        assert_eq!(config.columns.get("e").unwrap().native_type,BOOL);
        assert_eq!(config.columns.get("f").unwrap().native_type,BOOL);
        assert_eq!(config.columns.get("g").unwrap().native_type,U64);
        assert_eq!(config.columns.get("h").unwrap().native_type,U64);
        assert_eq!(config.columns.get("i").unwrap().native_type,U64);
        assert_eq!(config.columns.get("j").unwrap().native_type,U64);
        assert_eq!(config.columns.get("k").unwrap().native_type,U64);
        assert_eq!(config.columns.get("l").unwrap().native_type,U64);
        assert_eq!(config.columns.get("m").unwrap().native_type,U64);
        assert_eq!(config.columns.get("n").unwrap().native_type,U64);
        assert_eq!(config.columns.get("o").unwrap().native_type,D128);
        assert_eq!(config.columns.get("p").unwrap().native_type,D128);
        assert_eq!(config.columns.get("q").unwrap().native_type,D128);
        assert_eq!(config.columns.get("r").unwrap().native_type,D128);
        assert_eq!(config.columns.get("s").unwrap().native_type,D128);
        assert_eq!(config.columns.get("t").unwrap().native_type,D128);
        assert_eq!(config.columns.get("u").unwrap().native_type,F64);
        assert_eq!(config.columns.get("v").unwrap().native_type,F64);
        assert_eq!(config.columns.get("w").unwrap().native_type,F64);
        assert_eq!(config.columns.get("x").unwrap().native_type,F64);
        assert_eq!(config.columns.get("y").unwrap().native_type,F64);
        assert_eq!(config.columns.get("z").unwrap().native_type,F64);
        assert_eq!(config.columns.get("aa").unwrap().native_type,F64);
        assert_eq!(config.columns.get("ab").unwrap().native_type,F64);
        assert_eq!(config.columns.get("ac").unwrap().native_type,F64);

        config = s_config.get_table_config(&test_schema, &"characters".into()).unwrap();
        assert_eq!(config.columns.get("a").unwrap().native_type,Char(1));
        assert_eq!(config.columns.get("b").unwrap().native_type,Char(1));
        assert_eq!(config.columns.get("c").unwrap().native_type,Char(255));
        assert_eq!(config.columns.get("d").unwrap().native_type,Char(1));
        assert_eq!(config.columns.get("e").unwrap().native_type,Char(255));
        assert_eq!(config.columns.get("f").unwrap().native_type,Char(1));
        assert_eq!(config.columns.get("g").unwrap().native_type,Char(1));
        assert_eq!(config.columns.get("h").unwrap().native_type,Char(255));
        assert_eq!(config.columns.get("i").unwrap().native_type,Char(50));
        assert_eq!(config.columns.get("j").unwrap().native_type,Varchar(50));
        assert_eq!(config.columns.get("k").unwrap().native_type,Varchar(50));
        assert_eq!(config.columns.get("l").unwrap().native_type,Varchar(50));


        config = s_config.get_table_config(&test_schema, &"temporal".into()).unwrap();
        assert_eq!(config.columns.get("a").unwrap().native_type,DATE);
        assert_eq!(config.columns.get("b").unwrap().native_type,DATETIME(0));
        assert_eq!(config.columns.get("c").unwrap().native_type,DATETIME(6));
        assert_eq!(config.columns.get("d").unwrap().native_type,TIME(0));
        assert_eq!(config.columns.get("e").unwrap().native_type,TIME(6));
        assert_eq!(config.columns.get("f").unwrap().native_type,TIMESTAMP(0));
        assert_eq!(config.columns.get("g").unwrap().native_type,TIMESTAMP(6));
        assert_eq!(config.columns.get("h").unwrap().native_type,YEAR(4));
        assert_eq!(config.columns.get("i").unwrap().native_type,YEAR(4));

        config = s_config.get_table_config(&test_schema, &"binary".into()).unwrap();
        assert_eq!(config.columns.get("a").unwrap().native_type,FIXEDBINARY(1));
        assert_eq!(config.columns.get("b").unwrap().native_type,FIXEDBINARY(50));
        assert_eq!(config.columns.get("c").unwrap().native_type,VARBINARY(50));
        assert_eq!(config.columns.get("d").unwrap().native_type,VARBINARY(2_u32.pow(8)));
        assert_eq!(config.columns.get("e").unwrap().native_type,Varchar(2_u32.pow(8)));
        assert_eq!(config.columns.get("f").unwrap().native_type,VARBINARY(2_u32.pow(16)));
        assert_eq!(config.columns.get("g").unwrap().native_type,VARBINARY(50));
        assert_eq!(config.columns.get("h").unwrap().native_type,Varchar(2_u32.pow(16)));
        assert_eq!(config.columns.get("i").unwrap().native_type,Varchar(100));
        assert_eq!(config.columns.get("j").unwrap().native_type,LONGBLOB(2_u64.pow(24)));
        assert_eq!(config.columns.get("k").unwrap().native_type,LONGTEXT(2_u64.pow(24)));
        assert_eq!(config.columns.get("l").unwrap().native_type,LONGBLOB(2_u64.pow(32)));
        assert_eq!(config.columns.get("m").unwrap().native_type,LONGTEXT(2_u64.pow(32)));
        assert_eq!(config.columns.get("n").unwrap().native_type,FIXEDBINARY(1));
        assert_eq!(config.columns.get("o").unwrap().native_type,FIXEDBINARY(50));

        config = s_config.get_table_config(&test_schema, &"numerics_signed".into()).unwrap();
        assert_eq!(config.columns.get("a").unwrap().native_type,I64);
        assert_eq!(config.columns.get("b").unwrap().native_type,U64);
        assert_eq!(config.columns.get("c").unwrap().native_type,U64);
        assert_eq!(config.columns.get("d").unwrap().native_type,I64);
        assert_eq!(config.columns.get("e").unwrap().native_type,I64);
        assert_eq!(config.columns.get("f").unwrap().native_type,U64);
        assert_eq!(config.columns.get("g").unwrap().native_type,U64);
        assert_eq!(config.columns.get("h").unwrap().native_type,I64);
        assert_eq!(config.columns.get("i").unwrap().native_type,I64);
        assert_eq!(config.columns.get("j").unwrap().native_type,U64);

    }

    #[test]
    fn config_test_override() {
        env::set_var("TEST_SHARED_KEY", "44E6884D78AA18FA690917F84145AA4415FC3CD560915C7AE346673B1FDA5985");
        env::set_var("TEST_SHARED_IV", "03F72E7479F3E34752E4DD91");

        let config = super::parse_configs("src/test/test-zero-config.toml", "src/test/config_override");

        // token test sourced from the default config, i.e. not overridden anywhere
        let c = config.get_client_config();
        assert_eq!("127.0.0.1", c.host);
        assert_eq!("3307", c.port);

        // Test overrides
        let c = config.get_connection_config();
        assert_eq!("baruser", c.user);
        assert_eq!("barpassword", c.password);
        assert_eq!("localhost", c.host);

        let c = config.get_table_config(&"zero".into(), &"users".into()).unwrap();
        assert_eq!(c.columns.get("sex").unwrap().encryption, AesGcm);

        let c = config.get_table_config(&"fooschema".into(), &"footable".into()).unwrap();
        assert_eq!(c.columns.get("bar").unwrap().encryption, NA);

    }

    #[test]
    fn config_test_override_dir_doesnt_exist() {
        let config = super::parse_configs("src/test/test-zero-config.toml", "src/foo");
        // No assertions, just should not blow up.
    }

}