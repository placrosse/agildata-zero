use std::net;
use std::io::{Read, Write, Cursor};
use std::collections::HashMap;
use std::error::Error;
use byteorder::*;
use query::{Tokenizer, Parser, Writer, SQLWriter, ASTNode};
use query::dialects::mysqlsql::*;
use query::dialects::ansisql::*;

use query::planner::{Planner, TupleType, Element, HasTupleType, RelVisitor, Rel};
use super::writers::*;

use mio::{self, TryRead, TryWrite};
use mio::tcp::*;

use bytes::Take;

use config::{Config, TConfig, ColumnConfig};

use encrypt::{Decrypt, NativeType, EncryptionType};

use super::encrypt_visitor::EncryptVisitor;
use super::server::Proxy;
use super::server::State;

#[derive(Debug)]
pub struct MySQLPacket {
    pub bytes: Vec<u8>
}

impl MySQLPacket {

    pub fn new(buf: Vec<u8>) -> Self {
        MySQLPacket { bytes: buf }
    }

    pub fn parse_packet_length(header: &[u8]) -> usize {
        (((header[2] as u32) << 16) |
            ((header[1] as u32) << 8) |
            header[0] as u32) as usize
    }

    pub fn sequence_id(&self) -> u8 {
        self.bytes[3]
    }

    pub fn packet_type(&self) -> u8 {
        if self.bytes.len() > 4 {
            self.bytes[4]
        } else {
            0
        }
    }

}

pub struct MySQLPacketParser<'a> {
    payload: &'a [u8],
    pos: usize
}

impl<'a> MySQLPacketParser<'a> {

    pub fn new(packet: &'a MySQLPacket) -> Self {
        MySQLPacketParser { payload: &packet.bytes, pos: 4 }
    }

    pub fn skip(&mut self, n: usize) {
//        println!("Skipping {} bytes:", n);
//        print_packet_bytes(&self.payload[self.pos..self.pos+n]);
        self.pos += n;
    }

    /// read the length of a length-encoded field
    pub fn read_len(&mut self) -> usize {
        let n = self.payload[self.pos] as usize;
        self.pos += 1;

        match n {
            //NOTE: depending on context, 0xfb could mean null and 0xff could mean error
            0xfc | 0xfd | 0xfe => panic!("no support yet for length >= 251"),
            _ => {
                //println!("read_len() returning {}", n);
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

#[allow(dead_code)]
fn print_packet_chars(buf: &[u8]) {
    print!("[");
    for i in 0..buf.len() {
        print!("{} ", buf[i] as char);
    }
    println!("]");
}

#[allow(dead_code)]
fn print_packet_bytes(buf: &[u8]) {
    print!("[");
    for i in 0..buf.len() {
        if i%8==0 { println!(""); }
        print!("{:#04x} ",buf[i]);
    }
    println!("]");
}

fn parse_string(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("Invalid UTF-8")
}

pub trait MySQLConnection {
    fn read_packet(&mut self) -> Result<MySQLPacket, &'static str>;
}

impl MySQLConnection for net::TcpStream {

    fn read_packet(&mut self) -> Result<MySQLPacket, &'static str> {
        // read header
        let mut header_vec = vec![0_u8; 4];
        match self.read(&mut header_vec) {
            Ok(0) => Ok(MySQLPacket { bytes: vec![] }),
            Ok(n) => {
                assert!(n==4);

                let payload_len = MySQLPacket::parse_packet_length(&header_vec);

                // read payload
                let mut payload_vec = vec![0_u8; payload_len];
                assert!(payload_len == self.read(&mut payload_vec).unwrap());
                header_vec.extend_from_slice(&payload_vec);

                Ok(MySQLPacket { bytes: header_vec })
            },
            Err(_) => Err("oops")
        }
    }
}

#[derive(Debug)]
struct ColumnMetaData {
    schema: String,
    table_name: String,
    column_name: String
}

#[derive(Debug)]
enum ConnectionPhase {
    Handshake, Query
}

#[derive(Debug)]
pub struct MySQLConnectionHandler<'a> {
    pub socket: TcpStream, // this is the socket from the client
    token: mio::Token,
    state: State,
    phase: ConnectionPhase,
    remote: net::TcpStream, // this is the connection to the remote mysql server
    schema: Option<String>, // the current schema
    config: &'a Config
    //authenticating: bool
}

impl<'a> MySQLConnectionHandler <'a> {

    pub fn new(socket: TcpStream, token: mio::Token, config: &Config) -> MySQLConnectionHandler {
        println!("Creating remote connection...");

        let conn = config.get_connection_config();
        let conn_host = conn.props.get("host").unwrap();
        let default_port = &String::from("3306");
        let conn_port = conn.props.get("port").unwrap_or(default_port);

        let conn_addr = format!("{}:{}",conn_host,conn_port);

        // connect to real MySQL
        let mut mysql = net::TcpStream::connect(conn_addr.as_str()).unwrap();

        // read header
        let auth_packet = mysql.read_packet().unwrap();
        let len = auth_packet.bytes.len();

        let buf = Cursor::new(auth_packet.bytes);

        println!("Created new connection in Writing state");

        MySQLConnectionHandler {
            socket: socket,
            token: token,
            state: State::Writing(Take::new(buf, len)),
            phase: ConnectionPhase::Handshake,
            remote: mysql,
            schema: None,
            config: &config
            // authenticating: true
        }
    }

    pub fn ready(&mut self, event_loop: &mut mio::EventLoop<Proxy>, events: mio::EventSet) {
        match self.state {
            State::Reading(..) => {
                assert!(events.is_readable(), "unexpected events; events={:?}", events);
                self.read(event_loop)
            }
            State::Writing(..) => {
                assert!(events.is_writable(), "unexpected events; events={:?}", events);
                self.write(event_loop)
            }
            _ => unimplemented!(),
        }
    }

    /// process a single mysql packet from the client
    pub fn read(&mut self, event_loop: &mut mio::EventLoop<Proxy>){
        println!("Reading from client");

        let mut buf = Vec::with_capacity(1024);
        match self.socket.try_read_buf(&mut buf) {
            Ok(Some(0)) => {
                self.state = State::Closed;
            },
            Ok(Some(_)) => {
                // do we have enough bytes to read the packet len?
                if buf.len() > 3 {
                    // do we have the full packet?
                    let packet_len = MySQLPacket::parse_packet_length(&buf);
                    if buf.len() >= packet_len+4 {
                        match self.phase {
                            ConnectionPhase::Handshake => {
                                self.process_handshake_response(&buf, packet_len);
                                self.phase = ConnectionPhase::Query;
                            },
                            ConnectionPhase::Query => {
                                let packet_type = buf[4];
                                match packet_type {
                                    0x02 => self.process_init_db(&buf, packet_len),
                                    0x03 => {
                                        let res = self.process_query(&buf, packet_len);
                                        match res {
                                            Err(e) => {
                                                self.send_error(&String::from("42000"), &e.to_string());
                                             },
                                            Ok(()) => {}
                                        }
                                    },
                                    _ => {
                                        let res = self.mysql_process_query(&buf[0..packet_len + 4], None);
                                        match res {
                                            Err(e) => {
                                                self.send_error(&String::from("42000"), &e.to_string());
                                            },
                                            Ok(()) => {}

                                        }
                                    },
                                }
                            },
                        }
                    } else {
                        println!("do not have full packet!");
                    }

                } else {
                    println!("do not have full header!");
                }

                // Re-register the socket with the event loop. The current
                // state is used to determine whether we are currently reading
                // or writing.
                println!("Reregistering");
                self.reregister(event_loop);
            },
            Ok(None) => {
                self.reregister(event_loop);
            },
            Err(e) => {
                panic!("got an error trying to read; err={:?}", e);
            },
        }
    }

    fn process_handshake_response(&mut self, buf: &Vec<u8>, packet_len: usize) {
        // see https://dev.mysql.com/doc/internals/en/connection-phase-packets.html#packet-Protocol::HandshakeResponse
        //NOTE: this code makes assumptions about what 'capabilities' are active

        //print_packet_bytes(&buf);

        let p = MySQLPacket::new(buf.clone());
        let mut r = MySQLPacketParser::new(&p);
        r.skip(4); // capability flags, CLIENT_PROTOCOL_41 always set
        r.skip(4); // max-packet size
        r.skip(1); // character set
        r.skip(23); // reserved
        let username = r.read_c_string().unwrap(); // username
        println!("user: {}", username);
        let auth_response = r.read_bytes().unwrap(); // auth-response
        println!("auth_response: {:?}", auth_response);

        if let Some(schema) = r.read_c_string() {
            println!("HANDSHAKE: schema={}", schema);
            self.schema = Some(schema);
        }

        // pass along to MySQL
        self.mysql_process_query(&buf[0..packet_len + 4], None);
    }

    fn process_init_db(&mut self, buf: &Vec<u8>, packet_len: usize) {
        let schema = parse_string(&buf[5 as usize .. (packet_len+4) as usize]);
        println!("COM_INIT_DB: {}", schema);
        self.schema = Some(schema);
        self.mysql_process_query(&buf[0 .. packet_len+4], None);
    }

    fn process_query(&mut self, buf: &Vec<u8>, packet_len: usize) -> Result<(), Box<Error>> {
        println!("0x03");

        let query = parse_string(&buf[5 as usize .. (packet_len+4) as usize]);
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
                        None
                    }
                }
            },
            Err(e) => {
                println!("Failed to tokenize with: {}", e);
                None
            }
        };

        let plan = try!(self.plan(&parsed));

        // reqwrite query
        if parsed.is_some() {

            let value_map: HashMap<u32, Result<Vec<u8>, Box<Error>>> = HashMap::new();
            let mut encrypt_vis = EncryptVisitor{valuemap: value_map};

            // Visit and conditionally encrypt (if there was a plan)
            match plan {
                Some(ref p) => {
                    try!(encrypt_vis.visit_rel(p));
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
            let mut wtr: Vec<u8> = vec![];
            wtr.write_u32::<LittleEndian>((slice.len() + 1) as u32).unwrap();
            assert!(0x00 == wtr[3]);
            wtr.push(0x03); // packet type for COM_Query
            wtr.extend_from_slice(slice);

            match plan {
                None => {
                    try!(self.mysql_process_query(&wtr, None));
                    return Ok(());
                },
                Some(p) => {
                    let tt = p.tt();
                    try!(self.mysql_process_query(&wtr, Some(tt)));
                    return Ok(());
                }
            }

        } else {
            try!(self.mysql_process_query(&buf[0 .. packet_len+4], None));
            return Ok(());
        }
    }

    fn plan(&self, parsed: &Option<ASTNode>) -> Result<Option<Rel>, Box<Error>> {
        match parsed {
            &None => Ok(None),
            &Some(ref sql) => {
                let mut foo = match self.schema {
                    Some(ref s) => Some(s),
                    None => None
                };
                let planner = Planner::new(foo, &self.config);
                planner.sql_to_rel(sql)
            }
        }
    }

    fn mysql_process_query<'b>(&'b mut self, request: &'b [u8], tt: Option<&TupleType>) -> Result<(), Box<Error>> {
        println!("Sending packet to mysql");
        self.remote.write(request).unwrap();
        self.remote.flush().unwrap();

        println!("Reading from MySQL...");
        let mut write_buf: Vec<u8> = Vec::new();

        let packet = self.remote.read_packet().unwrap();
        let packet_type = packet.packet_type();

        println!("response packet type: {}", packet_type);

        match packet_type {
            // break on receiving OK_Packet, Err_Packet, or EOF_Packet
            0x00 | 0xfe | 0xff => {
                println!("Got OK/ERR/EOF packet");
                write_buf.extend_from_slice(&packet.bytes);
            },
            0xfb => panic!("not implemented"),
            0x03 => {
                println!("Got COM_QUERY packet");
                write_buf.extend_from_slice(&packet.bytes);

                match tt {
                    Some(t) => {
                        let field_count = t.elements.len() as u32;
                        for i in 0 .. field_count {
                            let field_packet = self.remote.read_packet().unwrap();
                            println!("column meta data packet type {}", field_packet.bytes[4]);
                            write_buf.extend_from_slice(&field_packet.bytes);
                        }
                        try!(self.process_result_set(&mut write_buf, tt));
                    },
                    None => {
                        loop {
                            let row_packet = self.remote.read_packet().unwrap();
                            println!("COM_QUERY handled packet type {}", row_packet.bytes[4]);

                            write_buf.extend_from_slice(&row_packet.bytes);
                            // break on receiving Err_Packet, or EOF_Packet
                            match row_packet.packet_type() {
                                0xfe | 0xff => break,
                                _ => {}
                            }
                        }
                    }
                }

            },
            _ => {

                println!("Got field_count packet");

                // first packet is field count
                write_buf.extend_from_slice(&packet.bytes);

                //TODO: this assumes < 251 fields in result set
                let field_count = packet.bytes[4] as u32;

                println!("Result set has {} columns", field_count);

                // read one result set meta data packet per column and append to write buffer
                for i in 0 .. field_count {
                    let field_packet = self.remote.read_packet().unwrap();
                    println!("column meta data packet type {}", field_packet.bytes[4]);
                    write_buf.extend_from_slice(&field_packet.bytes);
                }

                try!(self.process_result_set(&mut write_buf, tt));

            }
        }

        println!("Setting state to writing..");
        let buf_len = write_buf.len();
        let curs = Cursor::new(write_buf);
        self.state = State::Writing(Take::new(curs, buf_len));
        Ok(())
    }

    fn process_result_set(&mut self, write_buf: &mut Vec<u8>, tt: Option<&TupleType>)  -> Result<(), Box<Error>> {
        // process row packets until ERR or EOF
        loop {
            let row_packet = self.remote.read_packet().unwrap();
            match row_packet.packet_type() {
                // break on receiving Err_Packet, or EOF_Packet
                0xfe | 0xff => {
                    println!("End of result rows");
                    write_buf.extend_from_slice(&row_packet.bytes);
                    return Ok(());
                },
                _ => {
                    try!(self.process_result_row(&row_packet, write_buf, tt));
                }
            }
        }

    }

//    fn read_result_set_meta(&mut self, field_count: u32, write_buf: &mut Vec<u8>) -> Result<Vec<ColumnMetaData>, String> {
//
//        let mut column_meta: Vec<ColumnMetaData> = vec![];
//
//        for i in 0 .. field_count {
//
//            let field_packet = self.remote.read_packet().unwrap();
//            write_buf.extend_from_slice(&field_packet.bytes);
//
//            let mut r = MySQLPacketParser::new(&field_packet);
//
//            //TODO: assumes these values can never be NULL
//            let catalog = r.read_lenenc_string().unwrap();
//            let schema = r.read_lenenc_string().unwrap();
//            let table = r.read_lenenc_string().unwrap();
//            let org_table = r.read_lenenc_string().unwrap();
//            let name = r.read_lenenc_string().unwrap();
//            let org_name = r.read_lenenc_string().unwrap();
//
//            println!("ALL catalog {}, schema {}, table {}, org_table {}, name {}, org_name {}",
//             catalog, schema, table, org_table, name, org_name);
//
//            let md = ColumnMetaData {
//                schema: schema,
//                table_name: table,
//                column_name: name
//            };
//
//            println!("column {} = {:?}", i, md);
//
//            column_meta.push(md);
//        }
//
//        Ok(column_meta)
//    }

    fn process_result_row(&mut self,
                          row_packet: &MySQLPacket,
                          write_buf: &mut Vec<u8>,
                          tt: Option<&TupleType>) -> Result<(), Box<Error>> {

        println!("Received row");

        if tt.is_some() {

            let mut r = MySQLPacketParser::new(&row_packet);

            let mut wtr: Vec<u8> = vec![];

            for i in 0.. tt.unwrap().elements.len() {
                let value = match tt {
                    Some(t) => {
                        match &t.elements[i].encryption {
                            &EncryptionType::NA => r.read_lenenc_string(),
                            encryption @ _ => match &t.elements[i].data_type {
                                &NativeType::U64 => {
                                    let res = try!(u64::decrypt(&r.read_bytes().unwrap(), &encryption));
                                    Some(format!("{}", res))},
                                &NativeType::Varchar(_) => {
                                    let res = try!(String::decrypt(&r.read_bytes().unwrap(), &encryption));
                                    Some(res)
                                },
                                native_type @ _ => panic!("Native type {:?} not implemented", native_type)
                            }
                        }
                    },

                    None => r.read_lenenc_string()
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

            let mut new_header: Vec<u8> = vec![];
            let sequence_id = row_packet.sequence_id();
            new_header.write_u32::<LittleEndian>(wtr.len() as u32).unwrap();
            new_header.pop();
            new_header.push(sequence_id);
            write_buf.extend_from_slice(&new_header);
            write_buf.extend_from_slice(&wtr);
        } else {
            // no need to decrypt, just pass the bytes along
            write_buf.extend_from_slice(&row_packet.bytes);
        }

        Ok(())
    }

    pub fn write(&mut self, event_loop: &mut mio::EventLoop<Proxy>) {

        println!("Writing to client");

        // TODO: handle error
        match self.socket.try_write_buf(self.state.mut_write_buf()) {
            Ok(Some(_)) => {
                // If the entire line has been written, transition back to the
                // reading state
                println!("Transitioning to reading");
                self.state.try_transition_to_reading();

                // Re-register the socket with the event loop.
                self.reregister(event_loop);
            }
            Ok(None) => {
                println!("Just Reregistering");
                // The socket wasn't actually ready, re-register the socket
                // with the event loop
                self.reregister(event_loop);
            }
            Err(e) => {
                panic!("got an error trying to write; err={:?}", e);
            }
        }
    }

    pub fn send_error(&mut self, state: &str, msg: &String) {
        let mut err_header: Vec<u8> = vec![];
        let mut err_wtr: Vec<u8> = vec![];

        err_wtr.push(0xff);  //Header, shows its an error
        err_wtr.write_u16::<LittleEndian>(1064 as u16).unwrap(); //ERROR CODE

        err_wtr.extend_from_slice("#".as_bytes()); //sql_state_marker
        err_wtr.extend_from_slice(state.as_bytes()); //SQL STATE
        err_wtr.extend_from_slice(msg.as_bytes());

        err_header.write_u32::<LittleEndian>(err_wtr.len() as u32).unwrap();
        err_header.pop();
        err_header.push(1);

        let mut write_buf: Vec<u8> = Vec::new();
        write_buf.extend_from_slice(&err_header);
        write_buf.extend_from_slice(&err_wtr);

        let buf_len = write_buf.len();
        let curs = Cursor::new(write_buf);
        self.state = State::Writing(Take::new(curs, buf_len));
    }

    pub fn reregister(&self, event_loop: &mut mio::EventLoop<Proxy>) {
        event_loop.reregister(&self.socket, self.token, self.state.event_set(), mio::PollOpt::oneshot())
            .unwrap();
    }

    pub fn is_closed(&self) -> bool {
        match self.state {
            State::Closed => true,
            _ => false,
        }
    }
}
