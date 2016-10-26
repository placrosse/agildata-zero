
use rustc_serialize::Decodable;
use rustc_serialize::Decoder;
use toml;
use std::env;

use std::collections::HashMap;

#[derive(Debug, PartialEq)]
struct Config {
    client: ClientConfig,
    connection: ConnectionConfig,
    parsing: ParsingConfig,
    schemas: HashMap<String, SchemaConfig>,
}

impl Decodable for Config {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        Ok(
            Config {
                client: d.read_struct_field("client", 2, ClientConfig::decode)?,
                connection: d.read_struct_field("connection", 3, ConnectionConfig::decode)?,
                parsing: d.read_struct_field("parsing", 1, ParsingConfig::decode)?,
                schemas: d.read_map(decode_schema_config)?
            }
        )
    }
}

#[derive(Debug, PartialEq)]
struct ConnectionConfig {
    host: String,
    user: String,
    password: String
}

impl Decodable for ConnectionConfig {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        d.read_struct("ConnectionConfig", 3, |_d| -> _ {
            Ok(
                ConnectionConfig {
                    host: resolve_field_str_or_env("host", 0, _d)?,
                    user: resolve_field_str_or_env("user", 1, _d)?,
                    password: resolve_field_str_or_env("password", 2, _d)?
                }
            )
        })
    }
}


#[derive(Debug, PartialEq)]
struct ClientConfig {
    host: String,
    port: String
}

impl Decodable for ClientConfig {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        d.read_struct("ClientConfig", 3, |_d| -> _ {
            Ok(
                ClientConfig {
                    host: resolve_field_str_or_env("host", 0, _d)?,
                    port: resolve_field_str_or_env("port", 1, _d)?,
                }
            )
        })
    }
}

#[derive(Debug, RustcDecodable, PartialEq)]
struct ParsingConfig {
    mode: Mode
}

#[derive(Debug, PartialEq)]
enum Mode {
    STRICT,
    PERMISSIVE
}

impl Decodable for Mode {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        match &d.read_str()?.to_lowercase() as &str {
            "strict" => Ok(Mode::STRICT),
            "permissive" => Ok(Mode::PERMISSIVE),
            a => Err(d.error(&format!("Unknown parsing mode {}", a)))
        }
    }
}

#[derive(Debug, PartialEq)]
struct SchemaConfig {
    tables: HashMap<String, TableConfig>
}


#[derive(Debug, PartialEq)]
struct TableConfig {
    columns: HashMap<String, ColumnConfig>
}

#[derive(Debug, PartialEq)]
struct ColumnConfig {
    native_type: String,
    encryption: String
}


fn decode_schema_config<D:Decoder>(d: &mut D, l: usize) -> Result<HashMap<String, SchemaConfig>, D::Error> {
    println!("decode_schema_config() len {}", l);
    let mut schema_map: HashMap<String, SchemaConfig> = HashMap::new();
    for i in 0..l {
        let schema =  d.read_map_elt_key(i,decode_key_name)?;
        let t = d.read_map_elt_val(i, decode_schema)?;
        schema_map.insert(schema, t);
    }

    Ok(schema_map)
}

fn decode_key_name<D:Decoder>(d: &mut D) -> Result<String, D::Error> {
    d.read_str()
}

fn decode_schema<D:Decoder>(d: &mut D) -> Result<SchemaConfig, D::Error> {
    let mut table_map: HashMap<String, TableConfig> = HashMap::new();

    d.read_map(|_d, _l| -> _ {
        for i in 0.._l {
            let table = _d.read_map_elt_key(i,decode_key_name)?;
            let cs = _d.read_map_elt_val(i, decode_table)?;
            table_map.insert(table.to_lowercase(), cs);
        }

        Ok(SchemaConfig{tables: table_map})
    })

}

fn decode_table<D:Decoder>(d: &mut D) -> Result<TableConfig, D::Error> {
    let mut column_map: HashMap<String, ColumnConfig> = HashMap::new();

    d.read_map(|_d, _l| -> _ {
        for i in 0.._l {
            let column = _d.read_map_elt_key(i,decode_key_name)?;
            let cs = _d.read_map_elt_val(i, decode_column)?;
            column_map.insert(column.to_lowercase(), cs);
        }

        Ok(TableConfig{columns: column_map})
    })
}

fn decode_column<D:Decoder>(d: &mut D) -> Result<ColumnConfig, D::Error> {
    d.read_struct("ColumnConfig", 1, |_d| -> _ {
        Ok(
            ColumnConfig {
                native_type: resolve_field_str_or_env("type", 0, _d)?,
                encryption: resolve_field_str_or_env("encryption", 0, _d)?,
            }
        )
    })
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

/// optionally resolve env variable to string value, else string value
fn resolve_field_str_or_env<D: Decoder>(s: &str, index: usize, d: &mut D) -> Result<String, D::Error> {
    match d.read_struct_field(s, index, ResolvedString::decode) {
        Ok(v) => Ok(v.value),
        Err(e) => Err(e)
    }
}

#[cfg(test)]
mod test {

    use super::{Config, ClientConfig, SchemaConfig, ConnectionConfig, ParsingConfig, TableConfig, ColumnConfig, Mode};
    use toml;

    use std::collections::HashMap;

    use std::env;

    #[test]
    fn test_toml() {
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
        let decoded: Config = ::rustc_serialize::Decodable::decode(&mut decoder).unwrap();
        println!("{:#?}", decoded);

        println!("#{:#?}", decoder.toml);

        assert_eq!(decoded.client, ClientConfig {
                    host: "127.0.0.1".into(),
                    port: "3307".into()
        });

        assert_eq!(decoded.connection, ConnectionConfig {
                    host: "127.0.0.1".into(),
                    user: "agiluser".into(),
                    password: "password123".into()
        });

        assert_eq!(decoded.parsing, ParsingConfig {
                    mode: Mode::PERMISSIVE
        });

        let expected_column = ColumnConfig {
            native_type: "INTEGER".into(),
            encryption: "NONE".into()
        };

        let mut expected_column_map: HashMap<String, ColumnConfig> = HashMap::new();
        expected_column_map.insert("id".into(), expected_column);
        let expected_table_conf = TableConfig{columns: expected_column_map};

        let mut expected_table_map: HashMap<String, TableConfig> = HashMap::new();
        expected_table_map.insert("users".into(), expected_table_conf);

        let mut expected_schema: HashMap<String, SchemaConfig> = HashMap::new();
        expected_schema.insert("zero".into(), SchemaConfig{tables: expected_table_map});

        assert_eq!(decoded.schemas, expected_schema);

    }

}