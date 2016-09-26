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
    StmtPrepareResponse,
    /// Expecting column definitions for column and parameters as part of a COM_STMT_PREPARE response
    StmtPrepareFieldPacket(u16, Box<PStmt>, AtomicU32, AtomicU32),
    /// Expecting a COM_STMT_EXECUTE response
    StmtExecuteResponse,
    /// Expecting 1 or more field definitions as part of a COM_QUERY response
    StmtExecuteFieldPacket(usize),
    // Binary result row
    StmtExecuteResultRow,
    /// Instructs the packet handler to ignore all further result rows (due to an earlier error)
    IgnoreFurtherResults,
    /// Forward any further packets
    ForwardAll,
    /// Expect an OK or ERR packet
    OkErrResponse,
}

#[derive(Debug,PartialEq,Clone)]
struct PStmt {
    param_types: Vec<u8>,
    column_types: Vec<u8>,
//    ast: Box<ASTNode>
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
                Ok(PacketType::ComQuery) => self.process_com_query(p),
                Ok(PacketType::ComStmtPrepare) => self.process_com_stmt_prepare(p),
                Ok(PacketType::ComStmtExecute) => {
                    print_packet_chars("ComStmtExecute", &p.bytes);
                    let stmt_id = read_u16_le(&p.bytes[5..9]);

                    debug!("stmt_id = {}", stmt_id);

                    match self.stmt_map.get(&stmt_id) {
                        Some(ref pstmt) => {
                            debug!("Executing with {:?}", pstmt);

                        },
                        None => {
                            debug!("No statement in map for id {}", stmt_id);
                        }
                    }


                    self.state = HandlerState::StmtExecuteResponse;
                    Action::Forward
                },
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
            HandlerState::ComQueryFieldPacket(ref mut n) => if n.load(Ordering::SeqCst) == 1 {
                (Some(HandlerState::ExpectResultRow), Action::Forward)
            } else {
                n.fetch_sub(1, Ordering::SeqCst);
                (None, Action::Forward)
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
            HandlerState::StmtPrepareResponse => {
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

                let mut pstmt = PStmt {
                    param_types: Vec::with_capacity(num_params as usize),
                    column_types: Vec::with_capacity(num_columns as usize),
                };

                debug!("StmtPrepareResponse: stmt_id={}, num_columns={}, num_params={}", stmt_id, num_columns, num_params);
                (Some(HandlerState::StmtPrepareFieldPacket(
                    stmt_id,
                    Box::new(pstmt),
                    AtomicU32::new(num_params as u32),
                    AtomicU32::new(num_columns as u32),
                )), Action::Forward)
            },
            HandlerState::StmtPrepareFieldPacket(stmt_id, ref mut pstmt, ref mut num_params, ref mut num_columns) => {

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
            HandlerState::StmtExecuteResponse => {
                print_packet_chars("StmtExecuteResponse", &p.bytes);
                match p.bytes[4] {
                    0x00 | 0xfe | 0xff => (Some(HandlerState::ExpectClientRequest), Action::Forward),
                    _ => {
                        let mut r = MySQLPacketParser::new(&p.bytes);
                        let col_count = r.read_len();
                        (Some(HandlerState::StmtExecuteFieldPacket(col_count)), Action::Forward)
                    },
                }
            },
            HandlerState::StmtExecuteFieldPacket(mut col_count) => {
                print_packet_chars("StmtExecuteFieldPacket", &p.bytes);
                if col_count > 1 {
                    col_count = col_count - 1;
                    (None, Action::Forward)
                } else {
                    (Some(HandlerState::StmtExecuteResultRow), Action::Forward)
                }
            },
            HandlerState::StmtExecuteResultRow => {
                print_packet_chars("StmtExecuteResultRow", &p.bytes);
                match p.bytes[4] {
                    0xfe | 0xff => (Some(HandlerState::ExpectClientRequest), Action::Forward),
                    0x00 => (None, Action::Forward),
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
                println!("Unsupported state {:?}", self.state);
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

#[allow(dead_code)]
pub fn print_packet_chars(msg: &'static str, buf: &[u8]) {
    debug!("{} {:?}", msg, &buf);
//    print!("{}: packet_type={} [", msg, buf[4]);
//    for i in 0..buf.len() {
//        print!("{} ", buf[i] as char);
//    }
//    println!("]");
}

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
                        // write packed with new query
                        let slice: &[u8] = sql.as_bytes();
                        let mut packet: Vec<u8> = Vec::with_capacity(slice.len() + 4);
                        packet.write_u32::<LittleEndian>((slice.len() + 1) as u32).unwrap();
                        assert!(0x00 == packet[3]);
                        packet.push(0x03); // COM_QUERY request packet type
                        packet.extend_from_slice(slice);

                        self.tt = Some(p.projection.clone());

                        Action::Mutate(Packet { bytes: packet })
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
        self.state = HandlerState::StmtPrepareResponse;
        let query = parse_string(&p.bytes[5..]);

        Action::Forward
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

                debug!("Received row: tt={:?}", tt);

                let mut r = MySQLPacketParser::new(&p.bytes);
                let mut wtr: Vec<u8> = vec![];

                for i in 0..tt.len() {
                    debug!("decrypt element {} : {:?}", i, &tt[i]);

                    let value = match &tt[i].encryption {
                        &EncryptionType::NA => r.read_lenenc_string(),
                        encryption @ _ => match &tt[i].data_type {
                            &NativeType::U64 => {
                                let res = try!(u64::decrypt(&r.read_bytes().unwrap(), &encryption, &tt[i].key.unwrap()));
                                Some(format!("{}", res))
                            },
                            &NativeType::I64 => {
                                let res = try!(i64::decrypt(&r.read_bytes().unwrap(), &encryption, &tt[i].key.unwrap()));
                                Some(format!("{}", res))
                            },
                            &NativeType::Varchar(_) | &NativeType::Char(_) => { // TODO enforce length
                                let res = try!(String::decrypt(&r.read_bytes().unwrap(), &encryption, &tt[i].key.unwrap()));
                                Some(res)
                            },
                            &NativeType::BOOL => {
                                debug!("try decrypt bool");
                                let res = bool::decrypt(&r.read_bytes().unwrap(),  &encryption, &tt[i].key.unwrap())?;
                                debug!("FINISH decrypt bool");
                                Some(format!("{}", res))
                            },
                            &NativeType::D128 => {
                                let res = d128::decrypt(&r.read_bytes().unwrap(),  &encryption, &tt[i].key.unwrap())?;
                                Some(format!("{}", res))
                            },
                            &NativeType::F64 => {
                                let res = f64::decrypt(&r.read_bytes().unwrap(),  &encryption, &tt[i].key.unwrap())?;
                                Some(format!("{}", res))
                            },
                            &NativeType::DATE => {

                                let res = DateTime::decrypt(&r.read_bytes().unwrap(),  &encryption, &tt[i].key.unwrap())?;
                                Some(res.date().format("%Y-%m-%d").to_string())
                            },
                            &NativeType::DATETIME(ref fsp) => {
                                let res = DateTime::decrypt(&r.read_bytes().unwrap(),  &encryption, &tt[i].key.unwrap())?;
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
