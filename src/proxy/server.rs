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
use std::ops::Deref;

use config::{Config, TConfig, ColumnConfig};
use error::ZeroError;
use encrypt::{Decrypt, NativeType, EncryptionType};

use super::schema_provider::MySQLBackedSchemaProvider;
use super::writers::*;

use super::statement_cache::*;
use super::physical_planner::*;

use query::{Tokenizer, Parser, Writer, SQLWriter, ASTNode, LiteralToken};
use query::dialects::mysqlsql::*;
use query::dialects::ansisql::*;
use query::planner::{Planner, TupleType, HasTupleType, RelVisitor, Rel};

use decimal::*;
use chrono::{DateTime, TimeZone, NaiveDateTime};

use std::sync::atomic::{AtomicU32, Ordering};

pub struct Proxy {
//    server: TcpListener,
//    config: &'a Config,
//    provider: &'a MySQLBackedSchemaProvider<'a>
}

impl Proxy {

    pub fn run(config: Rc<Config>, provider: Rc<MySQLBackedSchemaProvider>, stmt_cache: Rc<StatementCache>) {

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
            let s = stmt_cache.clone();

            // create a future to serve requests
            let future = TcpStream::connect(&mysql_addr, &handle).and_then(move |mysql| {
                Ok((socket, mysql))
            }).and_then(move |(client, server)| {
                Pipe::new(Rc::new(client), Rc::new(server), ZeroHandler::new(c,p, s))
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

#[derive(Debug)]
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
    ComQueryFieldPacket(AtomicU32),
    // Expecting a COM_QUERY result row
    ExpectResultRow,
    /// Expecting a COM_STMT_PREPARE response
    StmtPrepareResponse(Rc<PhysicalPlan>),
    /// Expecting column definitions for column and parameters as part of a COM_STMT_PREPARE response
    StmtPrepareFieldPacket(u16, Box<PStmt>, AtomicU32, AtomicU32),
    /// Expecting a COM_STMT_EXECUTE response
    StmtExecuteResponse(Box<PStmt>),
    /// Expecting 1 or more field definitions as part of a COM_QUERY response
    StmtExecuteFieldPacket(usize, Box<PStmt>),
    /// Binary result row
    StmtExecuteResultRow(Box<PStmt>),
    /// Instructs the packet handler to ignore all further result rows (due to an earlier error)
    IgnoreFurtherResults,
    /// Forward any further packets
    ForwardAll,
    /// Expect an OK or ERR packet
    OkErrResponse,
}

#[derive(Debug,Clone)]
struct PStmt {
    param_types: Vec<ProtocolBinary>,
    column_types: Vec<ProtocolBinary>,
    plan: Rc<PhysicalPlan>,
    /// does the result set need to be decrypted?
    decrypt_result_set: bool,
}

#[derive(Debug,Clone,Copy,PartialEq)]
pub enum ProtocolBinary {
    Decimal = 0x00,
    Tiny = 0x01,
    Short = 0x02,
    Long = 0x03,
    Float = 0x04,
    Double = 0x05,
    Null = 0x06,
    Timestamp = 0x07,
    LongLong = 0x08,
    Int24 = 0x09,
    Date = 0x0a,
    Time = 0x0b,
    DateTime = 0x0c,
    Year = 0x0d,
    //	NewDate = 0x0e, -- not used in protocol
    Varchar = 0x0f,
    Bit = 0x10,
    //	Timestamp2 = 0x11, -- not used in protocol
    //	Datetime2 = 0x12, -- not used in protocol
    //	Time2 = 0x13, -- not used in protocol
    NewDecimal = 0xf6,
    Enum = 0xf7,
    Set = 0xf8,
    TinyBlob = 0xf9,
    MediumBlob = 0xfa,
    LongBlob = 0xfb,
    Blob = 0xfc,
    VarString = 0xfd,
    String = 0xfe,
    Geometry = 0xff,
}

struct ZeroHandler {
    config: Rc<Config>,
    provider: Rc<MySQLBackedSchemaProvider>,
    state: HandlerState,
    schema: Option<String>, // the current schema
    parsing_mode: ParsingMode,
    tt: Option<Vec<EncryptionPlan>>,
    stmt_map: HashMap<u16, Box<PStmt>>,
    stmt_cache: Rc<StatementCache>
}

impl ZeroHandler {

    fn new(config: Rc<Config>, provider: Rc<MySQLBackedSchemaProvider>, stmt_cache: Rc<StatementCache>) -> Self {

        let parsing_mode = determine_parsing_mode(&config.get_parsing_config().props.get("mode").unwrap());

        ZeroHandler {
            config: config.clone(),
            provider: provider.clone(),
            state: HandlerState::Handshake,
            schema: None,
            parsing_mode: parsing_mode,
            tt: None,
            stmt_map: HashMap::new(),
            stmt_cache: stmt_cache
        }
    }
}

fn determine_parsing_mode(mode: &String) -> ParsingMode {
    match &mode.to_uppercase() as &str {
        "STRICT" => ParsingMode::Strict,
        "PERMISSIVE" => ParsingMode::Permissive,
        _ => panic!("Unsupported parsing mode {}", mode)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ParsingMode {
    Strict,
    Permissive
}

//TODO: add this to MySQLPacketReader in mysql-proxy-rs
/// Read a little endian u16
fn read_u16_le(buf: &[u8]) -> u16 {
    ((buf[1] as u16) << 8) as u16 | buf[0] as u16
}

impl PacketHandler for ZeroHandler {

    fn handle_request(&mut self, p: &Packet) -> Action {
//        let action = if self.state == HandlerState::Handshake {
        let action = if let HandlerState::Handshake = self.state {
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
            let auth_response = r.read_lenenc_bytes().unwrap(); // auth-response
            debug!("auth_response: {:?}", auth_response);

            if let Some(schema) = r.read_c_string() {
                debug!("HANDSHAKE: schema={}", schema);
                self.schema = Some(schema);
            }

            Action::Forward
        } else {
            match p.packet_type() {
                Ok(PacketType::ComInitDb) => self.process_init_db(p),
                Ok(PacketType::ComQuery) => self.process_com_query(p),
                Ok(PacketType::ComStmtPrepare) => self.process_com_stmt_prepare(p),
                Ok(PacketType::ComStmtExecute) => self.process_com_stmt_execute(p),
                Ok(PacketType::ComStmtClose) => {
                    // no response for this statement
                    self.state = HandlerState::ExpectClientRequest;
                    Action::Forward
                },
                Ok(PacketType::ComStmtReset) => {
                    self.state = HandlerState::OkErrResponse;
                    Action::Forward
                },
                _ => {
                    self.state = HandlerState::ComQueryResponse;
                    Action::Forward
                }
            }
        };
        debug!("Request action: {:?}", action);
        action
    }

    fn handle_response(&mut self, p: &Packet) -> Action {

        print_packet_chars("handle_response", &p.bytes);

        let (state, action) = match self.state {
            HandlerState::Handshake => (None, Action::Forward),
            HandlerState::ComQueryResponse => {
                print_packet_chars("ComQueryResponse", &p.bytes);
                // this logic only applies to the very first response packet after a request
                match p.bytes[4] {
                    0x00 | 0xfe | 0xff => (Some(HandlerState::ExpectClientRequest), Action::Forward),
                    0xfb => panic!("not implemented"), //TODO: should not panic
                    0x03 => {
                        match self.tt {
                            Some(ref tt) => {
                                // expect one field_meta packet per column defined in tt
                                // expect 0 or more result rows
                                // expect result set terminator

                                (Some(HandlerState::ComQueryFieldPacket(AtomicU32::new(tt.len() as u32))), Action::Forward)
                            },
                            None => {
                                (Some(HandlerState::ForwardAll), Action::Forward)
                            }
                        }
                    },
                    _ => {
                        // expect a field_count packet
                        let field_count = p.bytes[4] as usize;
                        // expect field_count x field_meta packet
                        // expect 0 or more result rows
                        // expect result set terminator
                        (Some(HandlerState::ComQueryFieldPacket(AtomicU32::new(field_count as u32))), Action::Forward)
                    }
                }
            },
            HandlerState::ComQueryFieldPacket(ref mut n) => {
                if n.load(Ordering::SeqCst) == 1 {
                    (Some(HandlerState::ExpectResultRow), Action::Forward)
                } else {
                    n.fetch_sub(1, Ordering::SeqCst);
                    //TODO: need to rewrite field packet to change data type for encrypted columns
                    (None, Action::Forward)
                }
            },
            HandlerState::ExpectResultRow => match p.bytes[4] {
                0x00 | 0xfe | 0xff => (Some(HandlerState::ExpectClientRequest), Action::Forward),
                _ => {
                    match self.process_result_row(p) {
                        Ok(a) => (None, a),
                        Err(e) => (Some(HandlerState::IgnoreFurtherResults), create_error_from_err(e))
                    }
                }
            },
            HandlerState::IgnoreFurtherResults => match p.bytes[4] {
                0x00 | 0xfe | 0xff => (Some(HandlerState::ExpectClientRequest), Action::Drop),
                _ => (None, Action::Drop)
            },
            HandlerState::ForwardAll => match p.bytes[4] {
                0x00 | 0xfe | 0xff => (Some(HandlerState::ExpectClientRequest), Action::Forward),
                _ => (None, Action::Forward)
            },
            HandlerState::StmtPrepareResponse(ref plan) => {
                print_packet_chars("StmtPrepareResponse", &p.bytes);
                // COM_STMT_PREPARE_OK
                //status (1) -- [00] OK
                //statement_id (4) -- statement-id
                //num_columns (2) -- number of columns
                //num_params (2) -- number of params
                //reserved_1 (1) -- [00] filler
                //warning_count (2) -- number of warnings
                let stmt_id     = read_u16_le(&p.bytes[ 5.. 9]);
                let num_columns = read_u16_le(&p.bytes[ 9..11]);
                let num_params  = read_u16_le(&p.bytes[11..13]);

                // determine if the result set contains any encrypted columns
                let decrypt_result_set = match plan.as_ref() {
                    &PhysicalPlan::Plan(ref p) => {
                        p.projection.iter().filter(|e| match e.encryption {
                            EncryptionType::AES(_) => true,
                            EncryptionType::AES_GCM => true,
                            EncryptionType::NA => false,
                        }).count() > 0
                    },
                    _ => false
                };

                let mut pstmt = PStmt {
                    param_types: Vec::with_capacity(num_params as usize),
                    column_types: Vec::with_capacity(num_columns as usize),
                    plan: plan.clone(),
                    decrypt_result_set: decrypt_result_set
                };

                debug!("StmtPrepareResponse: stmt_id={}, num_columns={}, num_params={} decrypt_result_set={}",
                       stmt_id, num_columns, num_params, decrypt_result_set);
                (Some(HandlerState::StmtPrepareFieldPacket(
                    stmt_id,
                    Box::new(pstmt),
                    AtomicU32::new(num_params as u32),
                    AtomicU32::new(num_columns as u32),
                )), Action::Forward)
            },
            HandlerState::StmtPrepareFieldPacket(stmt_id, ref mut pstmt, ref mut num_params,
                                                 ref mut num_columns) => {

                debug!("StmtPrepareFieldPacket {:?}", &p.bytes);

                let mut r = MySQLPacketParser::new(&p.bytes);
                let _ = r.read_lenenc_string(); // catalog
                let schema = r.read_lenenc_string(); // schema
                let table = r.read_lenenc_string(); // table
                let _ = r.read_lenenc_string(); // org_table
                let name = r.read_lenenc_string(); // name
                let _ = r.read_lenenc_string(); // org_name
                let _ = r.read_len(); // length of fixed-length fields [0c]
                r.skip(2); // character set
                r.skip(4); // column length
                let mysql_type = r.read_byte().unwrap_or(0); // type
                r.skip(2); // flags
                r.skip(1); // decimals
                r.skip(2); // filler [00] [00]

                /* TODO: support this eventually
                  if command was COM_FIELD_LIST {
                lenenc_int     length of default-values
                string[$len]   default values
                  }*/

                debug!("StmtPrepareFieldPacket: stmt_id={}, num_columns={:?}, num_params={:?}, {:?}:{:?}:{:?}",
                         stmt_id, num_columns, num_params, schema, table, name);

                let mysql_type = match mysql_type {
                    0x00 => ProtocolBinary::Decimal,
                    0x01 => ProtocolBinary::Tiny ,
                    0x02 => ProtocolBinary::Short ,
                    0x03 => ProtocolBinary::Long ,
                    0x04 => ProtocolBinary::Float ,
                    0x05 => ProtocolBinary::Double ,
                    0x06 => ProtocolBinary::Null ,
                    0x07 => ProtocolBinary::Timestamp ,
                    0x08 => ProtocolBinary::LongLong ,
                    0x09 => ProtocolBinary::Int24 ,
                    0x0a => ProtocolBinary::Date ,
                    0x0b => ProtocolBinary::Time ,
                    0x0c => ProtocolBinary::DateTime ,
                    0x0d => ProtocolBinary::Year ,
                    0x0f => ProtocolBinary::Varchar ,
                    0x10 => ProtocolBinary::Bit ,
                    0xf6 => ProtocolBinary::NewDecimal ,
                    0xf7 => ProtocolBinary::Enum ,
                    0xf8 => ProtocolBinary::Set ,
                    0xf9 => ProtocolBinary::TinyBlob ,
                    0xfa => ProtocolBinary::MediumBlob ,
                    0xfb => ProtocolBinary::LongBlob ,
                    0xfc => ProtocolBinary::Blob ,
                    0xfd => ProtocolBinary::VarString ,
                    0xfe => ProtocolBinary::String ,
                    0xff => ProtocolBinary::Geometry ,
                    _ => panic!("TBD")
                };

                if num_params.load(Ordering::SeqCst) > 0 {
                    pstmt.param_types.push(mysql_type);
                    num_params.fetch_sub(1, Ordering::SeqCst);
                } else if num_columns.load(Ordering::SeqCst) > 0 {
                    pstmt.column_types.push(mysql_type);
                    num_columns.fetch_sub(1, Ordering::SeqCst);
                }

                // if all params and columns have been processed, store the pstmt in the map
                if num_params.load(Ordering::SeqCst) == 0
                    && num_columns.load(Ordering::SeqCst) == 0 {
                    debug!("stmt_id = {}, pstmt = {:?}", stmt_id, pstmt);
                    let b : Box<PStmt> = pstmt.clone();
                    self.stmt_map.insert(stmt_id, b);
                    (Some(HandlerState::ExpectClientRequest), Action::Forward)
                } else {
                    (None, Action::Forward)
                }
            },
            HandlerState::StmtExecuteResponse(ref pstmt) => {
                print_packet_chars("StmtExecuteResponse", &p.bytes);
                match p.bytes[4] {
                    0x00 | 0xfe | 0xff => (Some(HandlerState::ExpectClientRequest), Action::Forward),
                    _ => {
                        let mut r = MySQLPacketParser::new(&p.bytes);
                        let col_count = r.read_len();
                        (Some(HandlerState::StmtExecuteFieldPacket(col_count, pstmt.clone())), Action::Forward)
                    },
                }
            },
            HandlerState::StmtExecuteFieldPacket(col_count, ref plan) => {
                print_packet_chars("StmtExecuteFieldPacket", &p.bytes);
                if col_count > 1 {
                    (Some(HandlerState::StmtExecuteFieldPacket(col_count-1, plan.clone())), Action::Forward)
                } else {
                    (Some(HandlerState::StmtExecuteResultRow(plan.clone())), Action::Forward)
                }
            },
            HandlerState::StmtExecuteResultRow(ref pstmt) => {
                    print_packet_chars("StmtExecuteResultRow", &p.bytes);
                match p.bytes[4] {
                    0xfe | 0xff => (Some(HandlerState::ExpectClientRequest), Action::Forward),
                    0x00 => {
                        if pstmt.decrypt_result_set {
                            debug!("need to decrypt results");
                            match pstmt.plan.as_ref() {
                                &PhysicalPlan::Plan(ref pp) => {

                                    debug!("performing decryption");

                                    let cc = pstmt.column_types.len();

                                    let mut r = MySQLPacketParser::new(&p.bytes);

                                    let mut w = MySQLPacketWriter::new(p.bytes[3]);

                                    //TODO: this could be calculated once at planning time
                                    let null_bitmap_len = (cc + 7 + 2) / 8;

                                    w.write_bytes(&p.bytes[4..4+null_bitmap_len+1]); //TODO: check math here

                                    r.skip(null_bitmap_len+1);

                                    // iterate over each column's data
                                    for i in 0..cc {

                                        let offset = 2;
                                        let null_bitmap_byte = (i + offset) / 8;
                                        let null_bitmap_bit = ((i + offset) % 8) as u8;

                                        let null_byte = p.bytes[5+null_bitmap_byte];
                                        let null_bitmask = 1_u8 << null_bitmap_bit;

                                        let is_null = (null_byte & null_bitmask) > 0;

                                        debug!("column {} type={:?} null={}", i, pstmt.column_types[i], is_null);

                                        // only process non-null values
                                        if !is_null {

                                            let ref encryption = pp.projection[i].encryption;

                                            match pstmt.column_types[i] {

                                                // unencrypted integral types
                                                ProtocolBinary::Tiny     => copy(&mut r, &mut w, 1),
                                                ProtocolBinary::Short    => copy(&mut r, &mut w, 2),
                                                ProtocolBinary::Long     => copy(&mut r, &mut w, 4),
                                                ProtocolBinary::LongLong => copy(&mut r, &mut w, 8),

                                                // unencrypted float types
                                                ProtocolBinary::Float    => copy(&mut r, &mut w, 4),
                                                ProtocolBinary::Double   => copy(&mut r, &mut w, 8),

                                                ProtocolBinary::DateTime | ProtocolBinary::Timestamp | ProtocolBinary::Date => {
                                                    let len = r.read_byte().unwrap();
                                                    w.write_byte(len as u8);
                                                    copy(&mut r, &mut w, len as usize)
                                                },

                                                // unencrypted string types
                                                ProtocolBinary::Varchar | ProtocolBinary::Enum | ProtocolBinary::Set |
                                                ProtocolBinary::Geometry | ProtocolBinary::Bit | ProtocolBinary::Decimal |
                                                ProtocolBinary::NewDecimal | ProtocolBinary::String | ProtocolBinary::VarString |
                                                ProtocolBinary::LongBlob |
                                                ProtocolBinary::MediumBlob | ProtocolBinary::Blob | ProtocolBinary::TinyBlob => {
                                                    // note that unwrap() is safe here since result rows never contain null strings
                                                    let v = r.read_lenenc_bytes().unwrap();

                                                    match encryption {
                                                        &EncryptionType::AES(_) | &EncryptionType::AES_GCM => {
                                                            match write_decrypted(&pp.projection[i], v, &mut w) {
                                                                Ok(()) => {},
                                                                Err(e) => panic!("TBD")
                                                            }
                                                        },
                                                        _ => {
                                                            v.encode(&mut w);
                                                        }
                                                    }

                                                },
                                                _ => {
                                                    panic!("no support for {:?}", pstmt.column_types[i]);
                                                }
                                            }
                                        }
                                    }

                                    w.build();

                                    let new_packet = Packet { bytes: w.payload };

                                    print_packet_chars("Decrypted packet", &new_packet.bytes);

                                    (None, Action::Mutate(new_packet))
                                },
                                _ => {
                                    debug!("could not decrypt results because no plan");
                                    (None, Action::Forward)
                                }
                            }

                        } else {
                            debug!("no need to decrypt results");
                            (None, Action::Forward)
                        }
                    },
                    _ => panic!("invalid packet type {:?} for StmtExecuteResultRow", p.bytes[4])
                }
            },
            HandlerState::OkErrResponse => {
                print_packet_chars("OkErrResponse", &p.bytes);
                match p.bytes[4] {
                    0x00 | 0xff => (Some(HandlerState::ExpectClientRequest), Action::Forward),
                    _ => panic!("invalid packet type {:?} for OkErrResponse", p.bytes[4])
                }

            },
            _ => {
                print_packet_chars("Unexpected server response", &p.bytes);
                debug!("Unsupported state {:?}", self.state);
                (Some(HandlerState::ExpectClientRequest), Action::Forward)
            }

        };

        match state {
            Some(s) => {self.state = s},
            None => {}
        }

        //self.state = state;
        action


   }

}


fn write_decrypted(e: &EncryptionPlan, v: Vec<u8>, w: &mut MySQLPacketWriter) -> Result<(), Box<ZeroError>> {

    debug!("decrypt_aes()");

    match &e.data_type {
        &NativeType::U64 => {
            let n = try!(u64::decrypt(&v, &e.encryption, &e.key.unwrap()));
            n.encode(w);
            Ok(())
        },
        //                                                                &NativeType::I64 => {
        //                                                                    let res = try!(i64::decrypt(&r.read_bytes().unwrap(), &encryption, &tt[i].key.unwrap()));
        //                                                                    Some(format!("{}", res))
        //                                                                },
        &NativeType::Varchar(_) | &NativeType::Char(_) => { // TODO enforce length
            let s = try!(String::decrypt(&v, &e.encryption, &e.key.unwrap()));
            s.encode(w);
            Ok(())
        },
        //                                                                &NativeType::BOOL => {
        //                                                                    debug!("try decrypt bool");
        //                                                                    let res = bool::decrypt(&r.read_bytes().unwrap(),  &encryption, &tt[i].key.unwrap())?;
        //                                                                    debug!("FINISH decrypt bool");
        //                                                                    Some(format!("{}", res))
        //                                                                },
        //                                                                &NativeType::D128 => {
        //                                                                    let res = d128::decrypt(&r.read_bytes().unwrap(),  &encryption, &tt[i].key.unwrap())?;
        //                                                                    Some(format!("{}", res))
        //                                                                },
        //                                                                &NativeType::F64 => {
        //                                                                    let res = f64::decrypt(&r.read_bytes().unwrap(),  &encryption, &tt[i].key.unwrap())?;
        //                                                                    Some(format!("{}", res))
        //                                                                },
        //                                                                &NativeType::DATE => {
        //
        //                                                                    let res = DateTime::decrypt(&r.read_bytes().unwrap(),  &encryption, &tt[i].key.unwrap())?;
        //                                                                    Some(res.date().format("%Y-%m-%d").to_string())
        //                                                                },
        //                                                                &NativeType::DATETIME(ref fsp) => {
        //                                                                    let res = DateTime::decrypt(&r.read_bytes().unwrap(),  &encryption, &tt[i].key.unwrap())?;
        //                                                                    let fmt = match fsp {
        //                                                                        &0 => "%Y-%m-%d %H:%M:%S",
        //                                                                        &1 => "%Y-%m-%d %H:%M:%S%.1f",
        //                                                                        &2 => "%Y-%m-%d %H:%M:%S%.2f",
        //                                                                        &3 => "%Y-%m-%d %H:%M:%S%.3f",
        //                                                                        &4 => "%Y-%m-%d %H:%M:%S%.4f",
        //                                                                        &5 => "%Y-%m-%d %H:%M:%S%.5f",
        //                                                                        &6 => "%Y-%m-%d %H:%M:%S%.6f",
        //                                                                        _ => return Err(ZeroError::EncryptionError {
        //                                                                            message: format!("Invalid fractional second precision {}", fsp).into(),
        //                                                                            code: "1064".into()
        //                                                                        }.into())
        //                                                                    };
        //                                                                    Some(res.format(fmt).to_string())
        //
        //                                                                }
        native_type @ _ => panic!("Native type {:?} not implemented", native_type)
    }
}


#[allow(dead_code)]
pub fn print_packet_chars(msg: &'static str, buf: &[u8]) {
    debug!("{} {:?}", msg, &buf);
//    print!("{}: packet_type={} [", msg, buf[4]);
//    for i in 0..buf.len() {
//        print!("{} ", buf[i] as char);
//    }
//    println!("]");
}

#[derive(Debug)]
struct PhysPlanResult {
    literals: Vec<LiteralToken>,
    physical_plan: Rc<PhysicalPlan>
}

impl ZeroHandler {

    fn process_init_db(&mut self, p:&Packet) -> Action {
        let schema = parse_string(&p.bytes[5..]);
        debug!("COM_INIT_DB: {}", schema);
        self.schema = Some(schema);
        self.state = HandlerState::ComQueryResponse;
        Action::Forward
    }

    fn process_com_query(&mut self, p:&Packet) -> Action {
        debug!("COM_QUERY : {}", parse_string(&p.bytes[5..]));
        self.state = HandlerState::ComQueryResponse;

        self.tt = None;

        let physical_plan = self.get_physical_plan(parse_string(&p.bytes[5..]));

        match physical_plan.physical_plan.as_ref() {
            &PhysicalPlan::Plan(ref p) => {
                // re-write query
                let rewritten = self.rewrite_query(p, &physical_plan.literals);

                let action = match rewritten {
                    Ok(Some(sql)) => {
                        self.tt = Some(p.projection.clone());
                        // write packet with new query
                        let mut w = MySQLPacketWriter::new(0x00); // sequence_id 0x00
                        w.payload.push(0x03); // COM_QUERY request packet type
                        w.write_bytes(sql.as_bytes());
                        w.build();
                        let new_packet = Packet { bytes: w.payload };
                        Action::Mutate(new_packet)
                    },
                    Ok(None) => Action::Forward,
                    Err(e) => return create_error_from_err(e)
                };

                action
            },
            &PhysicalPlan::Passthrough => Action::Forward,
            &PhysicalPlan::Error(ref e) => return create_error_from_err(e.clone())
        }

    }

    fn process_com_stmt_prepare(&mut self, p:&Packet) -> Action {
        debug!("COM_STMT_PREPARE : {}", parse_string(&p.bytes[5..]));
        let plan = self.get_physical_plan(parse_string(&p.bytes[5..]));
        //TODO: rewrite query if it contains literals
        self.state = HandlerState::StmtPrepareResponse(plan.physical_plan);
        Action::Forward
    }

    fn process_com_stmt_execute(&mut self, p:&Packet) -> Action {
        print_packet_chars("ComStmtExecute", &p.bytes);
        let stmt_id = read_u16_le(&p.bytes[5..9]);

        debug!("stmt_id = {}", stmt_id);

        match self.stmt_map.get(&stmt_id) {
            Some(ref pstmt) => {
                debug!("Executing with {:?}", pstmt);
                self.state = HandlerState::StmtExecuteResponse((*pstmt).clone());
                Action::Forward
            },
            None => {
                debug!("No statement in map for id {}", stmt_id);
                create_error(format!("No statement in map for id {}", stmt_id))
            }
        }
    }

    fn get_physical_plan(&mut self, query: String) -> PhysPlanResult {
        let ansi = AnsiSQLDialect::new();
        let dialect = MySQLDialect::new(&ansi);

        match query.tokenize(&dialect) {
            Ok(tokens) => {
                match self.stmt_cache.get(&tokens.tokens) {
                    // We've cached this before, return cached plan
                    Some(p) => PhysPlanResult{literals: tokens.literals, physical_plan: p},
                    // This is new sql
                    None => {
                        match tokens.parse() {
                            Ok(parsed) => {
                                // Intercept use and set connection schema
                                match parsed {
                                    ASTNode::MySQLUse(box ASTNode::SQLIdentifier{id: ref schema, ..}) => {
                                        self.schema = Some(schema.clone())
                                    },
                                    _ => {}
                                };

                                // create the logical plan
                                let s = match self.schema {
                                    Some(ref s) => Some(s),
                                    None => None
                                };
                                let planner = Planner::new(s, self.provider.clone());
                                match planner.sql_to_rel(&parsed) {
                                    // If plan OK, continue
                                    Ok(logical_plan) => {
                                        let phys_planner = PhysicalPlanner{};
                                        let physical_plan = phys_planner.plan(logical_plan, parsed, &tokens.literals);


                                        PhysPlanResult{
                                            literals: tokens.literals,
                                            physical_plan: self.stmt_cache.put(tokens.tokens, physical_plan)
                                        }
                                    },
                                    // If error, store a failure plan, return error
                                    Err(e) => {
                                        let err = Box::new(ZeroError::ParseError {
                                            message: format!("Failed to plan query {}, due to {:?}", query, e),
                                            code: "1064".into()
                                        });

                                        self.stmt_cache.put(tokens.tokens, PhysicalPlan::Error(err.clone()));
                                        PhysPlanResult{literals: vec![], physical_plan: Rc::new(PhysicalPlan::Error(err))}

                                    }
                                }
                            },
                            Err(e) => {
                                debug!("Failed to parse with: {}", e);
                                match self.parsing_mode {

                                    // If strict mode...
                                    ParsingMode::Strict => {
                                        let q = query.to_uppercase();

                                        // Allow passthrough for inconsequential SQL
                                        if q.starts_with("SET") || q.starts_with("SHOW") || q.starts_with("BEGIN")
                                            || q.starts_with("COMMIT") || q.starts_with("ROLLBACK") {
                                            debug!("In Strict mode, allowing use of SET and SHOW");
                                            PhysPlanResult{literals: vec![], physical_plan: Rc::new(PhysicalPlan::Passthrough)}
                                        } else {
                                            // Otherwise, cache this plan-to-fail, return error
                                            error!("FAILED TO PARSE QUERY {}", query);
                                            let err = Box::new(ZeroError::ParseError {
                                                message: format!("Failed to parse query {}, due to {:?}", query, e),
                                                code: "1064".into()
                                            });

                                            self.stmt_cache.put(tokens.tokens, PhysicalPlan::Error(err.clone()));

                                            PhysPlanResult{literals: vec![], physical_plan: Rc::new(PhysicalPlan::Error(err))}
                                        }
                                    },
                                    // If permissive, .. passthrough
                                    ParsingMode::Permissive => {
                                        debug!("In Passive mode, falling through to MySQL");
                                        PhysPlanResult{literals: vec![], physical_plan: Rc::new(PhysicalPlan::Passthrough)}
                                    }
                                }
                            }
                        }
                    }
                }

            },
            Err(e) => {
                debug!("Failed to tokenize with: {}", e);
                match self.parsing_mode {
                    ParsingMode::Strict => {
                        let err = Box::new(ZeroError::ParseError {
                            message: format!("Failed to tokenize with: {}", e),
                            code: "1064".into()
                        });
                        PhysPlanResult{literals: vec![], physical_plan: Rc::new(PhysicalPlan::Error(err))}
                    },
                    ParsingMode::Permissive => {
                        debug!("In Passive mode, falling through to MySQL");
                        PhysPlanResult{literals: vec![], physical_plan: Rc::new(PhysicalPlan::Passthrough)}
                    }
                }
            }
        }

    }

    fn rewrite_query(&mut self, physical_plan: &PPlan, literals: &Vec<LiteralToken>) -> Result<Option<String>, Box<ZeroError>> {

        let lit_writer = LiteralEncryptionWriter {
            literals: literals,
            literal_plans: &physical_plan.literals
        };

        let s = match self.schema {
            Some(ref s) => s.clone(),
            None => String::from("") // TODO
        };
        let translator = CreateTranslatingWriter {
            config: &self.config,
            schema: &s
        };
        let mysql_writer = MySQLWriter {};
        let ansi_writer = AnsiSQLWriter {
            literal_tokens: literals
        };

        let writer = SQLWriter::new(vec![
                                &lit_writer,
                                &translator,
                                &mysql_writer,
                                &ansi_writer
                            ]);

        let rewritten = writer.write(&physical_plan.ast).unwrap();

        debug!("Rewritten query: {}", rewritten);
        Ok(Some(rewritten))

    }

    fn process_result_row(&mut self,
                          p: &Packet,
                          ) -> Result<Action, Box<ZeroError>> {


        match self.tt {
            Some(ref tt) => {

                // do we need to decrypt anything?
                let decrypt_result_set = tt.iter().filter(|e| match e.encryption {
                    EncryptionType::AES(_) => true,
                    EncryptionType::AES_GCM => true,
                    EncryptionType::NA => false,
                }).count() > 0;

                if decrypt_result_set {
                    debug!("Received row: tt={:?}", tt);

                    let mut r = MySQLPacketParser::new(&p.bytes);
                    let mut w = MySQLPacketWriter::new(p.bytes[3]);

                    for i in 0..tt.len() {
                        debug!("decrypt element {} : {:?}", i, &tt[i]);

                        let value = match &tt[i].encryption {
                            &EncryptionType::NA => r.read_lenenc_string(),
                            encryption @ _ => {
                                match r.read_lenenc_bytes() {
                                    Some(ref v) => {
                                        match &tt[i].data_type {
                                            &NativeType::U64 => {
                                                let res = try!(u64::decrypt(&v, &encryption, &tt[i].key.unwrap()));
                                                Some(format!("{}", res))
                                            },
                                            &NativeType::I64 => {
                                                let res = try!(i64::decrypt(&v, &encryption, &tt[i].key.unwrap()));
                                                Some(format!("{}", res))
                                            },
                                            &NativeType::Varchar(_) | &NativeType::Char(_) => { // TODO enforce length
                                                let res = try!(String::decrypt(&v, &encryption, &tt[i].key.unwrap()));
                                                Some(res)
                                            },
                                            &NativeType::BOOL => {
                                                debug!("try decrypt bool");
                                                let res = bool::decrypt(&v, &encryption, &tt[i].key.unwrap())?;
                                                debug!("FINISH decrypt bool");
                                                Some(format!("{}", res))
                                            },
                                            &NativeType::D128 => {
                                                let res = d128::decrypt(&v, &encryption, &tt[i].key.unwrap())?;
                                                Some(format!("{}", res))
                                            },
                                            &NativeType::F64 => {
                                                let res = f64::decrypt(&v, &encryption, &tt[i].key.unwrap())?;
                                                Some(format!("{}", res))
                                            },
                                            &NativeType::DATE => {

                                                let res = DateTime::decrypt(&v, &encryption, &tt[i].key.unwrap())?;
                                                Some(res.date().format("%Y-%m-%d").to_string())
                                            },
                                            &NativeType::DATETIME(ref fsp) => {
                                                let res = DateTime::decrypt(&v, &encryption, &tt[i].key.unwrap())?;
                                                let fmt = match fsp {
                                                    &0 => "%Y-%m-%d %H:%M:%S",
                                                    &1 => "%Y-%m-%d %H:%M:%S%.1f",
                                                    &2 => "%Y-%m-%d %H:%M:%S%.2f",
                                                    &3 => "%Y-%m-%d %H:%M:%S%.3f",
                                                    &4 => "%Y-%m-%d %H:%M:%S%.4f",
                                                    &5 => "%Y-%m-%d %H:%M:%S%.5f",
                                                    &6 => "%Y-%m-%d %H:%M:%S%.6f",
                                                    _ => return Err(ZeroError::EncryptionError {
                                                        message: format!("Invalid fractional second precision {}", fsp).into(),
                                                        code: "1064".into()
                                                    }.into())
                                                };
                                                Some(res.format(fmt).to_string())

                                            }
                                            native_type @ _ => panic!("Native type {:?} not implemented", native_type)
                                        }
                                    },
                                    None => None
                                }

                            }
                        };

                        match value {
                            Some(s) => w.write_lenenc_bytes(s.as_bytes()),
                            None => w.write_byte(0xfb)
                        }

                    }

                    // write the new packet
                    w.build();
                    let new_packet = Packet { bytes: w.payload };
                    Ok(Action::Mutate(new_packet))
                } else {
                    // no decryption required, so just forward packet
                    Ok(Action::Forward)
                }
            },
            // no decryption required, so just forward packet
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

    /// reads a single byte
    pub fn read_byte(&mut self) -> Option<u8> {
        if self.pos < self.payload.len() {
            let b = self.payload[self.pos];
            self.pos += 1;
            Some(b)
        } else {
            None
        }
    }

        /// reads a length-encoded string
    pub fn read_lenenc_string(&mut self) -> Option<String> {
        match self.read_lenenc_bytes() {
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

    pub fn read_lenenc_bytes(&mut self) -> Option<Vec<u8>> {
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

//    pub fn read_slice(&mut self, n: usize) -> &[u8] {
//        let v = &self.payload[self.pos..self.pos+n];
//        self.pos += n;
//        v
//    }
}

struct MySQLPacketWriter {
    payload: Vec<u8>
}

impl MySQLPacketWriter {


    fn new(sequence_id: u8) -> Self {
        MySQLPacketWriter {
            payload: vec![0x00, 0x00, 0x00, sequence_id],
        }
    }

    fn write_lenenc_bytes(&mut self, b: &[u8]) {
        let l = b.len();

        // write the length of the data using variable-length encoding
        if l < 0xfc {
            // single byte to represent length
            self.payload.push(l as u8);
        } else {
            // two bytes to represent length
            //TODO: add support for 3 and 4 byte lengths!!
            self.payload.push(0xfc);
            self.payload.write_u16::<LittleEndian>(l as u16);
        }

        // now write the actual data
        self.payload.extend_from_slice(b);
    }

    fn write_byte(&mut self, b: u8) {
        self.payload.push(b);
    }

    fn write_bytes(&mut self, b: &[u8]) {
        self.payload.extend_from_slice(b);
    }

    /// calculates the payload length and writes it to the first three bytes of the header
    fn build(&mut self) {
        //TODO: could re-implement this struct/impl to avoid being so expensive here
        let l = self.payload.len() - 4;
        let mut header : Vec<u8> = Vec::with_capacity(4);
        // write the payload length to the header
        header.write_u32::<LittleEndian>(l as u32).unwrap();
        // length is a 3-byte little-endian integer, so the fourth byte must always be zero
        assert!(0x00 == header[3]);
        // copy the header into the payload
        self.payload[0] = header[0];
        self.payload[1] = header[1];
        self.payload[2] = header[2];
        //TODO: would be nice to transfer ownership of payload to the packet
        //Packet { bytes: self.payload.clone() }
    }

}

trait MySQLEncoder {
    fn encode(&self, w: &mut MySQLPacketWriter);
}



impl MySQLEncoder for u64 {
    fn encode(&self, w: &mut MySQLPacketWriter) {
        w.payload.write_u64::<LittleEndian>(*self);
    }
}

impl MySQLEncoder for String {
    fn encode(&self, w: &mut MySQLPacketWriter) {
        w.write_lenenc_bytes(self.as_bytes());
    }
}

impl MySQLEncoder for Vec<u8> {
    fn encode(&self, w: &mut MySQLPacketWriter) {
        w.write_lenenc_bytes(&self);
    }
}

fn copy(r: &mut MySQLPacketParser, w: &mut MySQLPacketWriter, n: usize) {
    w.write_bytes(&r.payload[r.pos..r.pos+n]);
    r.skip(n);
}
