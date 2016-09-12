use mysql_proxy::*;

use futures::{Future};
use futures::stream::Stream;
use tokio_core::net::{TcpStream, TcpListener};
use tokio_core::reactor::{Core};
use byteorder::*;

use bytes::{Buf, Take};
use std::mem;
use std::net::{SocketAddr};
use std::io::Cursor;
use std::str::FromStr;
use std::collections::HashMap;
use std::env;
use std::rc::Rc;
use std::error::Error;

use config::{Config, TConfig, ColumnConfig};

use encrypt::{Decrypt, NativeType, EncryptionType};
use super::encrypt_visitor::EncryptVisitor;

use super::schema_provider::MySQLBackedSchemaProvider;
use super::writers::*;

use query::{Tokenizer, Parser, Writer, SQLWriter, ASTNode};
use query::dialects::mysqlsql::*;
use query::dialects::ansisql::*;
use query::planner::{Planner, TupleType, HasTupleType, RelVisitor, Rel};

pub struct Proxy {
//    server: TcpListener,
//    config: &'a Config,
//    provider: &'a MySQLBackedSchemaProvider<'a>
}

impl Proxy {

    pub fn run(config: Rc<Config>, provider: Rc<MySQLBackedSchemaProvider>) {

        //env_logger::init().unwrap();

        //TODO: refactor to reduce repeated code
        let temp = config.clone();

        // determine address for the proxy to bind to
        let conn = temp.get_client_config();
        let conn_host = conn.props.get("host").unwrap();
        let default_port = &String::from("3307");
        let conn_port = conn.props.get("port").unwrap_or(default_port);
        let conn_addr = format!("{}:{}",conn_host,conn_port);
        let bind_addr = conn_addr.parse::<SocketAddr>().unwrap();
        println!("Binding to {}", bind_addr);

        // determine address of the MySQL instance we are proxying for
        let conn = temp.get_connection_config();
        let conn_host = conn.props.get("host").unwrap();
        let default_port = &String::from("3306");
        let conn_port = conn.props.get("port").unwrap_or(default_port);
        let conn_addr = format!("{}:{}",conn_host,conn_port);
        let mysql_addr = conn_addr.parse::<SocketAddr>().unwrap();
        println!("MySQL server: {}", mysql_addr);

        // Create the tokio event loop that will drive this server
        let mut l = Core::new().unwrap();

        // Get a reference to the reactor event loop
        let handle = l.handle();

        // Create a TCP listener which will listen for incoming connections
        let socket = TcpListener::bind(&bind_addr, &l.handle()).unwrap();
        println!("Listening on: {}", bind_addr);

        // for each incoming connection
        let done = socket.incoming().for_each(move |(socket, _)| {

            let c = config.clone();
            let p = provider.clone();

            // create a future to serve requests
            let future = TcpStream::connect(&mysql_addr, &handle).and_then(move |mysql| {
                Ok((socket, mysql))
            }).and_then(move |(client, server)| {
                Pipe::new(Rc::new(client), Rc::new(server), ZeroHandler::new(c,p))
            });

            // tell the tokio reactor to run the future
            handle.spawn(future.map_err(|err| {
                println!("Failed to spawn future: {:?}", err);
            }));

            // everything is great!
            Ok(())

        });
        l.run(done).unwrap();    }
}

struct ZeroHandler {
    config: Rc<Config>,
    provider: Rc<MySQLBackedSchemaProvider>,
    handshake: bool,
    schema: Option<String>, // the current schema
    parsing_mode: ParsingMode,
    tt: Option<TupleType>
}

impl ZeroHandler {

    fn new(config: Rc<Config>, provider: Rc<MySQLBackedSchemaProvider>) -> Self {

        let parsing_mode = determine_parsing_mode(&config.get_parsing_config().props.get("mode").unwrap());

        ZeroHandler {
            config: config.clone(),
            provider: provider.clone(),
            handshake: true,
            schema: None,
            parsing_mode: parsing_mode,
            tt: None
        }
    }


    fn plan(&self, parsed: &Option<ASTNode>) -> Result<Option<Rel>, Box<Error>> {
        match parsed {
            &None => Ok(None),
            &Some(ref sql) => {
                let foo = match self.schema {
                    Some(ref s) => Some(s),
                    None => None
                };
                let planner = Planner::new(foo, self.provider.clone());
                planner.sql_to_rel(sql)
            }
        }
    }
}

fn determine_parsing_mode(mode: &String) -> ParsingMode {
    match &mode.to_uppercase() as &str {
        "STRICT" => ParsingMode::Strict,
        "PASSIVE" => ParsingMode::Passive,
        _ => panic!("Unsupported parsing mode {}", mode)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ParsingMode {
    Strict,
    Passive
}


impl PacketHandler for ZeroHandler {

    fn handle_request(&mut self, p: &Packet) -> Action {
        if self.handshake {
            self.handshake = false;
            Action::Forward
        } else {
            match p.packet_type() {
                Ok(PacketType::ComInitDb) => self.process_init_db(p),
                Ok(PacketType::ComQuery) => self.process_query(p),
                _ => Action::Forward
            }
        }
    }

    fn handle_response(&mut self, p: &Packet) -> Action {
        Action::Forward
    }
}

impl ZeroHandler {

    fn process_init_db(&mut self, p:&Packet) -> Action {
        let schema = parse_string(&p.bytes[5..]);
        println!("COM_INIT_DB: {}", schema);
        self.schema = Some(schema);
        Action::Forward
    }

    fn process_query(&mut self, p:&Packet) -> Action {
        let query = parse_string(&p.bytes[5..]);
        println!("COM_QUERY : {}", query);

        // parse query
        let ansi = AnsiSQLDialect::new();
        let dialect = MySQLDialect::new(&ansi);

        // TODO error handling
        let parsed = match query.tokenize(&dialect) {
            Ok(tokens) => {
                match tokens.parse() {
                    Ok(parsed) => {
                        match parsed {
                            ASTNode::MySQLUse(box ASTNode::SQLIdentifier{id: ref schema, ..}) => {
                                self.schema = Some(schema.clone())
                            },
                            _ => {}
                        };
                        Some(parsed)
                    },
                    Err(e) => {
                        println!("Failed to parse with: {}", e);
                        match self.parsing_mode{
                            ParsingMode::Strict =>{
                                return create_error(e);
                            },
                            ParsingMode::Passive =>{
                                println!("In Passive mode, falling through to MySQL");
                                None
                            }
                        }
                    }
                }
            },
            Err(e) => {
                println!("Failed to tokenize with: {}", e);
                None
            }
        };

        let plan = match self.plan(&parsed) {
            Ok(p) => p,
            Err(e) => return create_error_from_err(e)
        };

        // reqwrite query
        if parsed.is_some() {

            let value_map: HashMap<u32, Result<Vec<u8>, Box<Error>>> = HashMap::new();
            let mut encrypt_vis = EncryptVisitor{valuemap: value_map};

            // Visit and conditionally encrypt (if there was a plan)
            match plan {
                Some(ref p) => {
                    match encrypt_vis.visit_rel(p) {
                        Ok(r) => r,
                        Err(e) => return create_error_from_err(e)
                    }
                },
                None => {}
            }

            let lit_writer = LiteralReplacingWriter{literals: &encrypt_vis.get_value_map()};
            let s = match self.schema {
                Some(ref s) => s.clone(),
                None => String::from("") // TODO
            };
            let translator = CreateTranslatingWriter {
                config: &self.config,
                schema: &s
            };
            let mysql_writer = MySQLWriter{};
            let ansi_writer = AnsiSQLWriter{};

            let writer = SQLWriter::new(vec![
                                        &lit_writer,
                                        &translator,
                                        &mysql_writer,
                                        &ansi_writer
                                    ]);

            let rewritten = writer.write(&parsed.unwrap()).unwrap();

            println!("REWRITTEN {}", rewritten);

            // write packed with new query
            let slice: &[u8] = rewritten.as_bytes();
            let mut packet: Vec<u8> = Vec::with_capacity(slice.len() + 4);
            packet.write_u32::<LittleEndian>((slice.len() + 1) as u32).unwrap();
            assert!(0x00 == packet[3]);
            packet.push(0x03); // packet type for COM_Query
            packet.extend_from_slice(slice);

            match plan {
                None => Action::Forward,
                Some(p) => {
                    self.tt = Some(p.tt().clone()); //TODO: don't clone
                    Action::Mutate(Packet { bytes: packet })
                }
            }

        } else {
            Action::Forward
        }

    }

}

fn create_error(e: String) -> Action {
    Action::Error {
        code: 1234,
        state: [0x34, 0x32, 0x30, 0x30, 0x30], //&String::from("42000")
        msg: e }
}

fn create_error_from_err(e: Box<Error>) -> Action {
    Action::Error {
        code: 1234,
        state: [0x34, 0x32, 0x30, 0x30, 0x30], //&String::from("42000")
        msg: e.to_string() }
}

fn parse_string(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("Invalid UTF-8")
}
