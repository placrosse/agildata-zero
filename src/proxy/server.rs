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
use error::ZeroError;
use encrypt::{Decrypt, NativeType, EncryptionType};
use super::encrypt_visitor::EncryptVisitor;

use super::schema_provider::MySQLBackedSchemaProvider;
use super::writers::*;

use query::{Tokenizer, Parser, Writer, SQLWriter, ASTNode};
use query::dialects::mysqlsql::*;
use query::dialects::ansisql::*;
use query::planner::{Planner, TupleType, HasTupleType, RelVisitor, Rel};

use decimal::*;
use chrono::{DateTime, TimeZone, NaiveDateTime};

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
        info!("Binding to {}", bind_addr);

        // determine address of the MySQL instance we are proxying for
        let conn = temp.get_connection_config();
        let conn_host = conn.props.get("host").unwrap();
        let default_port = &String::from("3306");
        let conn_port = conn.props.get("port").unwrap_or(default_port);
        let conn_addr = format!("{}:{}",conn_host,conn_port);
        let mysql_addr = conn_addr.parse::<SocketAddr>().unwrap();
        info!("MySQL server: {}", mysql_addr);

        // Create the tokio event loop that will drive this server
        let mut l = Core::new().unwrap();

        // Get a reference to the reactor event loop
        let handle = l.handle();

        // Create a TCP listener which will listen for incoming connections
        let socket = TcpListener::bind(&bind_addr, &l.handle()).unwrap();
        info!("Listening on: {}", bind_addr);

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

                error!("Failed to spawn future: {:?}", err);
            }));

            // everything is great!
            Ok(())

        });
        l.run(done).unwrap();
    }
}

#[derive(PartialEq, Debug)]
enum HandlerState {
    /// Expect response from the server
    ExpectServerResponse,
    /// Done processing expected results, waiting for next request from client
    ExpectClientRequest,
    /// Expecting a connection handshake packet
    Handshake,
    /// Expecting a COM_QUERY response
    ComQueryResponse,
    /// Expecting 1 or more field definitions as part of a COM_QUERY response
    ComQueryFieldPacket(usize),
    // Expecting a COM_QUERY result row
    ExpectResultRow,
    /// Expecting a COM_STMT_PREPARE response
    StmtPrepareResponse,
    /// Expecting column definitions for column and parameters as part of a COM_STMT_PREPARE response
    StmtPrepareFieldPacket(Option<u16>, Option<u16>),
    /// Expecting a COM_STMT_EXECUTE response
    StmtExecuteResponse,
    /// Expecting 1 or more field definitions as part of a COM_QUERY response
    StmtExecuteFieldPacket(usize),
    // Binary result row
    StmtExecuteResultRow,
    /// Instructs the packet handler to ignore all further result rows (due to an earlier error)
    IgnoreFurtherResults,
    /// Forward any further packets
    ForwardAll
}

struct ZeroHandler {
    config: Rc<Config>,
    provider: Rc<MySQLBackedSchemaProvider>,
    state: HandlerState,
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
            state: HandlerState::Handshake,
            schema: None,
            parsing_mode: parsing_mode,
            tt: None
        }
    }


    fn plan(&self, parsed: &Option<ASTNode>) -> Result<Option<Rel>, Box<ZeroError>> {
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

//TODO: add this to MySQLPacketReader in mysql-proxy-rs
/// Read a little endian u16
fn read_u16_le(buf: &[u8]) -> u16 {
    ((buf[1] as u16) << 8) as u16 | buf[0] as u16
}

impl PacketHandler for ZeroHandler {

    fn handle_request(&mut self, p: &Packet) -> Action {
        if self.state == HandlerState::Handshake {
            self.state = HandlerState::ComQueryResponse;

            // see https://dev.mysql.com/doc/internals/en/connection-phase-packets.html#packet-Protocol::HandshakeResponse
            //NOTE: this code makes assumptions about what 'capabilities' are active

            let mut r = MySQLPacketParser::new(&p.bytes);
            r.skip(4); // capability flags, CLIENT_PROTOCOL_41 always set
            r.skip(4); // max-packet size
            r.skip(1); // character set
            r.skip(23); // reserved
            let username = r.read_c_string().unwrap(); // username
            debug!("user: {}", username);
            let auth_response = r.read_bytes().unwrap(); // auth-response
            debug!("auth_response: {:?}", auth_response);

            if let Some(schema) = r.read_c_string() {
                debug!("HANDSHAKE: schema={}", schema);
                self.schema = Some(schema);
            }

            Action::Forward
        } else {
            match p.packet_type() {
                Ok(PacketType::ComInitDb) => self.process_init_db(p),
                Ok(PacketType::ComQuery) => self.process_query(p),
                Ok(PacketType::ComStmtPrepare) => {
                    println!("ComStmtPrepare");
                    self.state = HandlerState::StmtPrepareResponse;
                    Action::Forward
                },
                Ok(PacketType::ComStmtExecute) => {
                    println!("ComStmtExecute");
                    self.state = HandlerState::StmtExecuteResponse;
                    Action::Forward
                },
//                Ok(PacketType::ComStmtClose) => {
//                    self.state = HandlerState::ComQueryResponse;
//                    Action::Forward
//                },
//                Ok(PacketType::ComStmtReset) => {
//                    self.state = HandlerState::ComQueryResponse;
//                    Action::Forward
//                },
                _ => {
                    self.state = HandlerState::ComQueryResponse;
                    Action::Forward
                }
            }
        }
    }

    fn handle_response(&mut self, p: &Packet) -> Action {

        let (state, action) = match self.state {
            HandlerState::Handshake => (HandlerState::Handshake, Action::Forward),
            HandlerState::ComQueryResponse => {
                // this logic only applies to the very first response packet after a request
                match p.bytes[4] {
                    0x00 | 0xfe | 0xff => (HandlerState::ExpectClientRequest, Action::Forward),
                    0xfb => panic!("not implemented"), //TODO: should not panic
                    0x03 => {
                        match self.tt {
                            Some(ref tt) => {
                                // expect one field_meta packet per column defined in tt
                                // expect 0 or more result rows
                                // expect result set terminator

                                (HandlerState::ComQueryFieldPacket(tt.elements.len()), Action::Forward)
                            },
                            None => {
                                (HandlerState::ForwardAll, Action::Forward)
                            }
                        }
                    },
                    _ => {
                        // expect a field_count packet
                        let field_count = p.bytes[4] as usize;
                        // expect field_count x field_meta packet
                        // expect 0 or more result rows
                        // expect result set terminator
                        (HandlerState::ComQueryFieldPacket(field_count), Action::Forward)
                    }
                }
            },
            HandlerState::ComQueryFieldPacket(n) => if n == 1 {
                (HandlerState::ExpectResultRow, Action::Forward)
            } else {
                (HandlerState::ComQueryFieldPacket(n -1), Action::Forward)
            },
            HandlerState::ExpectResultRow => match p.bytes[4] {
                0x00 | 0xfe | 0xff => (HandlerState::ExpectClientRequest, Action::Forward),
                _ => {
                    match self.process_result_row(p) {
                        Ok(a) => (HandlerState::ExpectResultRow, a),
                        Err(e) => (HandlerState::IgnoreFurtherResults, create_error_from_err(e))
                    }
                }
            },
            HandlerState::IgnoreFurtherResults => match p.bytes[4] {
                0x00 | 0xfe | 0xff => (HandlerState::ExpectClientRequest, Action::Drop),
                _ => (HandlerState::IgnoreFurtherResults, Action::Drop)
            },
            HandlerState::ForwardAll => match p.bytes[4] {
                0x00 | 0xfe | 0xff => (HandlerState::ExpectClientRequest, Action::Forward),
                _ => (HandlerState::ForwardAll, Action::Forward)
            },
            HandlerState::StmtPrepareResponse => {
                println!("StmtPrepareResponse");
                // COM_STMT_PREPARE_OK
                //status (1) -- [00] OK
                //statement_id (4) -- statement-id
                //num_columns (2) -- number of columns
                //num_params (2) -- number of params
                //reserved_1 (1) -- [00] filler
                //warning_count (2) -- number of warnings
                let num_columns = read_u16_le(&p.bytes[ 9..11]);
                let num_params  = read_u16_le(&p.bytes[11..13]);
                println!("StmtPrepareResponse: num_columns={}, num_params={}", num_columns, num_params);
                (HandlerState::StmtPrepareFieldPacket(
                    if num_params  > 0 { Some(num_params)  } else { None },
                    if num_columns > 0 { Some(num_columns) } else { None }
                ), Action::Forward)
            },
            HandlerState::StmtPrepareFieldPacket(num_params, num_columns) => {
                println!("StmtPrepareFieldPacket: num_columns={:?}, num_params={:?}", num_columns, num_params);

                println!("{:?}", &p.bytes);

                match num_params {
                    Some(n) => (HandlerState::StmtPrepareFieldPacket(
                        if n == 1 { None } else { Some(n-1) }, num_columns), Action::Forward),
                    None => match num_columns {
                        Some(n) => (HandlerState::StmtPrepareFieldPacket(
                            None, if n == 1 { None } else { Some(n-1) }), Action::Forward),
                        None => (HandlerState::ExpectClientRequest, Action::Forward)
                    }
                }
            },
            HandlerState::StmtExecuteResponse => {
                print_packet_chars("StmtExecuteResponse", &p.bytes);
                match p.bytes[4] {
                    0x00 | 0xfe | 0xff => (HandlerState::ExpectClientRequest, Action::Forward),
                    _ => {
                        let mut r = MySQLPacketParser::new(&p.bytes);
                        let col_count = r.read_len();
                        (HandlerState::StmtExecuteFieldPacket(col_count), Action::Forward)
                    },
                }
            },
            HandlerState::StmtExecuteFieldPacket(col_count) => {
                print_packet_chars("StmtExecuteFieldPacket", &p.bytes);
                if col_count > 1 {
                    (HandlerState::StmtExecuteFieldPacket(col_count-1), Action::Forward)
                } else {
                    (HandlerState::StmtExecuteResultRow, Action::Forward)
                }
            },
            HandlerState::StmtExecuteResultRow => {
                print_packet_chars("StmtExecuteResultRow", &p.bytes);
                match p.bytes[4] {
                    0x00 => (HandlerState::ExpectClientRequest, Action::Forward),
                    0x01 => (HandlerState::StmtExecuteResultRow, Action::Forward),
                    _ => panic!("invalid packet type {:?} for StmtExecuteResultRow", p.bytes[4])

                }
            },
            _ => {
                println!("Unsupported state {:?}", self.state);
                (HandlerState::ExpectClientRequest, Action::Forward)
            }

        };

        self.state = state;
        action


   }
}

#[allow(dead_code)]
pub fn print_packet_chars(msg: &'static str, buf: &[u8]) {
    print!("{}: [", msg);
    for i in 0..buf.len() {
        print!("{} ", buf[i] as char);
    }
    println!("]");
}

impl ZeroHandler {

    fn process_init_db(&mut self, p:&Packet) -> Action {
        let schema = parse_string(&p.bytes[5..]);
        debug!("COM_INIT_DB: {}", schema);
        self.schema = Some(schema);
        self.state = HandlerState::ComQueryResponse;
        Action::Forward
    }

    fn process_query(&mut self, p:&Packet) -> Action {
        let query = parse_string(&p.bytes[5..]);
        debug!("COM_QUERY : {}", query);

        self.tt = None;

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
                        debug!("Failed to parse with: {}", e);
                        match self.parsing_mode{
                            ParsingMode::Strict =>{
                                let q = query.to_uppercase();
                                if q.starts_with("SET") || q.starts_with("SHOW") || q.starts_with("BEGIN")
                                    || q.starts_with("COMMIT") || q.starts_with("ROLLBACK") {
                                    debug!("In Strict mode, allowing use of SET and SHOW");
                                    None
                                } else {
                                    error!("FAILED TO PARSE QUERY {}", query);
                                    return create_error(e.to_string());
                                }
                            },
                            ParsingMode::Passive =>{
                                debug!("In Passive mode, falling through to MySQL");
                                None
                            }
                        }
                    }
                }
            },
            Err(e) => {
                debug!("Failed to tokenize with: {}", e);
                None
            }
        };

        let plan = match self.plan(&parsed) {
            Ok(p) => p,
            Err(e) => {
                error!("FAILED TO PLAN QUERY {}", query);
                return create_error_from_err(e)
            }
        };

        // reqwrite query
       let action = if parsed.is_some() {

            let value_map: HashMap<u32, Vec<u8>> = HashMap::new();
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

            debug!("REWRITTEN {}", rewritten);

            // write packed with new query
            let slice: &[u8] = rewritten.as_bytes();
            let mut packet: Vec<u8> = Vec::with_capacity(slice.len() + 4);
            packet.write_u32::<LittleEndian>((slice.len() + 1) as u32).unwrap();
            assert!(0x00 == packet[3]);
            packet.push(0x03); // packet type for COM_Query
            packet.extend_from_slice(slice);

            match plan {
                None => {self.tt = None;},
                Some(p) => {
                    self.tt = Some(p.tt().clone()); //TODO: don't clone
                }
            }

           Action::Mutate(Packet { bytes: packet })
        } else {
            Action::Forward
        };

        self.state = HandlerState::ComQueryResponse;
        action
    }

    fn process_result_row(&mut self,
                          p: &Packet,
                          ) -> Result<Action, Box<ZeroError>> {

        match self.tt {
            Some(ref tt) => {

                debug!("Received row: tt={:?}", tt);

                let mut r = MySQLPacketParser::new(&p.bytes);
                let mut wtr: Vec<u8> = vec![];

                for i in 0..tt.elements.len() {
                    debug!("decrypt element {} : {:?}", i, &tt.elements[i]);

                    let value = match &tt.elements[i].encryption {
                        &EncryptionType::NA => r.read_lenenc_string(),
                        encryption @ _ => match &tt.elements[i].data_type {
                            &NativeType::U64 => {
                                let res = try!(u64::decrypt(&r.read_bytes().unwrap(), &encryption, &tt.elements[i].key));
                                Some(format!("{}", res))
                            },
                            &NativeType::I64 => {
                                let res = try!(i64::decrypt(&r.read_bytes().unwrap(), &encryption, &tt.elements[i].key));
                                Some(format!("{}", res))
                            },
                            &NativeType::Varchar(_) | &NativeType::Char(_) => { // TODO enforce length
                                let res = try!(String::decrypt(&r.read_bytes().unwrap(), &encryption, &tt.elements[i].key));
                                Some(res)
                            },
                            &NativeType::BOOL => {
                                debug!("try decrypt bool");
                                let res = bool::decrypt(&r.read_bytes().unwrap(),  &encryption, &tt.elements[i].key)?;
                                debug!("FINISH decrypt bool");
                                Some(format!("{}", res))
                            },
                            &NativeType::D128 => {
                                let res = d128::decrypt(&r.read_bytes().unwrap(),  &encryption, &tt.elements[i].key)?;
                                Some(format!("{}", res))
                            },
                            &NativeType::F64 => {
                                let res = f64::decrypt(&r.read_bytes().unwrap(),  &encryption, &tt.elements[i].key)?;
                                Some(format!("{}", res))
                            },
                            &NativeType::DATE => {

                                let res = DateTime::decrypt(&r.read_bytes().unwrap(),  &encryption, &tt.elements[i].key)?;
                                Some(res.date().format("%Y-%m-%d").to_string())
                            },
                            &NativeType::DATETIME(ref fsp) => {
                                let res = DateTime::decrypt(&r.read_bytes().unwrap(),  &encryption, &tt.elements[i].key)?;
                                let fmt = match fsp {
                                    &0 => "%Y-%m-%d %H:%M:%S",
                                    &1 => "%Y-%m-%d %H:%M:%S%.1f",
                                    &2 => "%Y-%m-%d %H:%M:%S%.2f",
                                    &3 => "%Y-%m-%d %H:%M:%S%.3f",
                                    &4 => "%Y-%m-%d %H:%M:%S%.4f",
                                    &5 => "%Y-%m-%d %H:%M:%S%.5f",
                                    &6 => "%Y-%m-%d %H:%M:%S%.6f",
                                    _ => return Err(ZeroError::EncryptionError {
                                        message: format!("Invalid fractional second precision {}, column: {}.{}",
                                                         fsp, &tt.elements[i].relation, &tt.elements[i].name).into(),
                                        code: "1064".into()
                                    }.into())
                                };
                                Some(res.format(fmt).to_string())

                            }
                            native_type @ _ => panic!("Native type {:?} not implemented", native_type)
                        }
                    };

                    // encode this field in the new packet
                    match value {
                        None => wtr.push(0xfb),
                        Some(v) => {
                            let slice = v.as_bytes();
                            //TODO: hacked to assume single byte for string length
                            wtr.write_u8(slice.len() as u8).unwrap();
                            wtr.extend_from_slice(&slice);
                        }
                    }
                }

                let mut new_packet: Vec<u8> = vec![];
                let sequence_id = p.sequence_id();
                new_packet.write_u32::<LittleEndian>(wtr.len() as u32).unwrap();
                new_packet.pop();
                new_packet.push(sequence_id);
                new_packet.extend_from_slice(&wtr);

                Ok(Action::Mutate(Packet { bytes: new_packet }))
            },
            None => Ok(Action::Forward)
        }
    }

}

fn create_error(e: String) -> Action {
    Action::Error {
        code: 1234,
        state: [0x34, 0x32, 0x30, 0x30, 0x30], //&String::from("42000")
        msg: e }
}

fn create_error_from_err(e: Box<ZeroError>) -> Action {
    Action::Error {
        code: 1234,
        state: [0x34, 0x32, 0x30, 0x30, 0x30], //&String::from("42000")
        msg: e.to_string() }
}

fn parse_string(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("Invalid UTF-8")
}

pub struct MySQLPacketParser<'a> {
    payload: &'a [u8],
    pos: usize
}

impl<'a> MySQLPacketParser<'a> {

    pub fn new(bytes: &'a [u8]) -> Self {
        MySQLPacketParser { payload: &bytes, pos: 4 }
    }

    pub fn skip(&mut self, n: usize) {
        self.pos += n;
    }

    /// read the length of a length-encoded field
    pub fn read_len(&mut self) -> usize {
        let n = self.payload[self.pos] as usize;
        self.pos += 1;

        match n {
            //NOTE: depending on context, 0xfb could mean null and 0xff could mean error
            0xfc => {
                let len = Cursor::new(&self.payload[self.pos..]).read_u16::<LittleEndian>().unwrap() as usize;
                self.pos += 2;
                len
            },
            0xfd => panic!("no support yet for length >= 2^16"),
            0xfe => panic!("no support yet for length >= 2^24"),
            _ => {
                //debug!("read_len() returning {}", n);
                n
            }
        }
    }

    /// reads a length-encoded string
    pub fn read_lenenc_string(&mut self) -> Option<String> {
        match self.read_bytes() {
            Some(s) => Some(String::from_utf8(s.to_vec()).expect("Invalid UTF-8")),
            None => None
        }
    }

    /// reads a null terminated string
    pub fn read_c_string(&mut self) -> Option<String> {
        let start = self.pos;
        while self.payload[self.pos] != 0x00 {
            self.pos += 1;
        }
        let mut v : Vec<u8> = vec![];
        v.extend_from_slice(&self.payload[start..self.pos]);
        self.pos += 1; // skip the NULL byte
        Some(String::from_utf8(v).expect("Invalid UTF-8"))
    }

    pub fn read_bytes(&mut self) -> Option<Vec<u8>> {
        match self.read_len() {
            0xfb => None,
            n @ _ => {
                let s = &self.payload[self.pos..self.pos+n];
                self.pos += n;
                let mut v : Vec<u8> = vec![];
                v.extend_from_slice(s);
                Some(v)
            }
        }
    }

}
