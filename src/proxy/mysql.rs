use std::net;
use std::io::{Read, Write, Cursor};
use std::collections::HashMap;

use byteorder::*;

use parser::sql_parser::{AnsiSQLParser, SQLExpr};
use parser::sql_writer::*;
use super::writers::*;

use mio::{self, TryRead, TryWrite};
use mio::tcp::*;

use bytes::{Take};

use config::{Config, TConfig, ColumnConfig};

use encrypt::{Decrypt, NativeType, EncryptionType};

use super::encryption_visitor::EncryptionVisitor;
use super::server::Proxy;
use super::server::State;

#[derive(Debug)]
pub struct MySQLPacket {
    pub bytes: Vec<u8>
}

impl MySQLPacket {

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

    /// read the length of a length-encoded field
    pub fn read_len(&mut self) -> usize {
        let n = self.payload[self.pos] as usize;
        self.pos += 1;

        match n {
            //NOTE: depending on context, 0xfb could mean null and 0xff could mean error
            0xfc | 0xfd | 0xfe => panic!("no support yet for length >= 251"),
            _ => n
        }
    }

    pub fn read_string(&mut self) -> Option<String> {
        match self.read_bytes() {
            Some(s) => Some(String::from_utf8(s.to_vec()).expect("Invalid UTF-8")),
            None => None
        }
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
pub struct MySQLConnectionHandler<'a> {
    pub socket: TcpStream, // this is the socket from the client
    token: mio::Token,
    state: State,
    remote: net::TcpStream, // this is the connection to the remote mysql server
    schema: Option<String>, // the current schema
    config: &'a Config
    //authenticating: bool
}

impl<'a> MySQLConnectionHandler <'a> {

    pub fn new(socket: TcpStream, token: mio::Token, config: &Config) -> MySQLConnectionHandler {
        println!("Creating remote connection...");
        // let ip  = std::net::Ipv4Addr::new(127,0,0,1);
        // let saddr = std::net::SocketAddr::new(std::net::IpAddr::V4(ip), 3306);
        // let mut tcps = TcpStream::connect(&saddr).unwrap();

        // connect to real MySQL
        let mut mysql = net::TcpStream::connect("127.0.0.1:3306").unwrap();

        // read header
        let auth_packet = mysql.read_packet().unwrap();
        let len = auth_packet.bytes.len();

        let buf = Cursor::new(auth_packet.bytes);

        println!("Created new connection in Writing state");

        MySQLConnectionHandler {
            socket: socket,
            token: token,
            state: State::Writing(Take::new(buf, len)),
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
    pub fn read(&mut self, event_loop: &mut mio::EventLoop<Proxy>) {

        println!("Reading from client");

        let mut buf = Vec::with_capacity(1024);
        match self.socket.try_read_buf(&mut buf) {
            Ok(Some(0)) => {
                self.state = State::Closed;
            }
            Ok(Some(_)) => {

                // this is very verbose debug logging, please leave disabled when you push to master
                /*
                println!("read {} bytes", n);
                print!("Bytes read [");
                for i in 0..buf.len() {
                    print!("{} ",buf[i] as char);
                }
                println!("]");

                println!("Bytes read:");
                for i in 0..buf.len() {
                    if i%8==0 { println!(""); }
                    print!("{:#04x} ",buf[i]);
                }
                */

                // do we have enough bytes to read the packet len?
                if buf.len() > 3 {

                    let packet_len = MySQLPacket::parse_packet_length(&buf);

                    println!("incoming packet_len = {}", packet_len);
                    println!("Buf len {}", buf.len());

                    // do we have the full packet?
                    if buf.len() >= packet_len+4 {
                        let packet_type = buf[4];
                        match packet_type {
                            0x02 => self.process_init_db(&buf, packet_len),
                            0x03 => self.process_query(&buf, packet_len),
                            _ => self.mysql_send(&buf[0 .. packet_len+4])
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
                self.reregister(event_loop);
            }
            Ok(None) => {
                self.reregister(event_loop);
            }
            Err(e) => {
                panic!("got an error trying to read; err={:?}", e);
            }
        }
    }

    fn process_init_db(&mut self, buf: &Vec<u8>, packet_len: usize) {
        let schema = parse_string(&buf[5 as usize .. (packet_len+4) as usize]);
        println!("COM_INIT_DB: {}", schema);
        self.schema = Some(schema);
        self.mysql_send(&buf[0 .. packet_len+4]);
    }

    fn process_query(&mut self, buf: &Vec<u8>, packet_len: usize) {
        println!("0x03");

        let query = parse_string(&buf[5 as usize .. (packet_len+4) as usize]);
        println!("COM_QUERY : {}", query);

        // parse query
        let parser = AnsiSQLParser {};
        let result = parser.parse(&query);

        let parsed: Option<SQLExpr> = match result {
            Ok(p) => {
                match p {
                    SQLExpr::SQLUse(box SQLExpr::SQLIdentifier(ref schema)) => {
                        self.schema = Some(schema.clone())
                    },
                    _ => {}
                };
                Some(p)
            },
            Err(e) => {
                println!("Failed to parse due to {:?}", e);
                None
            }
        };

        // visit and conditionally encrypt query


        // reqwrite query
        if parsed.is_some() {

            let value_map: HashMap<u32, Option<Vec<u8>>> = HashMap::new();
            let mut encrypt_vis = EncryptionVisitor {
                config: self.config,
                valuemap: value_map
            };
            match parsed {
                Some(ref expr) => super::encryption_visitor::walk(&mut encrypt_vis, expr),
                None => {}
            }
            // encryption_visitor::walk(&mut encrypt_vis, &parsed.unwrap());


            let lit_writer = LiteralReplacingWriter{literals: &encrypt_vis.get_value_map()};
            let translator = CreateTranslatingWriter {
                config: &self.config,
                schema: &String::from("zero") // TODO proxy should know its connection schema...
            };

            let writer = SQLWriter::new(vec![
                                        &lit_writer,
                                        &translator
                                    ]);

            let rewritten = writer.write(&parsed.unwrap()).unwrap();

            println!("REWRITTEN {}", rewritten);

            // write packed with new query
            //let n_buf: Vec<u8> = Vec::new();
            let slice: &[u8] = rewritten.as_bytes();

            let mut wtr: Vec<u8> = vec![];
            wtr.write_u32::<LittleEndian>((slice.len() + 1) as u32).unwrap();
            assert!(0x00 == wtr[3]);
            wtr.push(0x03); // packet type for COM_Query
            wtr.extend_from_slice(slice);

            println!("SENDING {:?}", wtr);
            self.mysql_send(&wtr);

        } else {
            let send = &buf[0 .. packet_len+4];
            println!("SENDING:");
            for i in 0..send.len() {
                if i%8==0 { println!(""); }
                print!("{:#04x} ",send[i]);
            }
            //println!("SENDING {:?}", send);
            self.mysql_send(&buf[0 .. packet_len+4]);
        }
    }

    fn mysql_send(&mut self, request: &[u8]) {
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

                loop {
                    let row_packet = self.remote.read_packet().unwrap();
                    match row_packet.packet_type() {
                        // break on receiving Err_Packet, or EOF_Packet
                        0xfe | 0xff => {

                            println!("End of result rows");
                            write_buf.extend_from_slice(&row_packet.bytes);
                            break
                        },
                        _ => {
                            write_buf.extend_from_slice(&row_packet.bytes);
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

                let mut column_meta: Vec<ColumnMetaData> = vec![];

                for i in 0 .. field_count {

                    let field_packet = self.remote.read_packet().unwrap();
                    write_buf.extend_from_slice(&field_packet.bytes);

                    let mut r = MySQLPacketParser::new(&field_packet);

                    //TODO: assumes these values can never be NULL
                    let catalog = r.read_string().unwrap();
                    let schema = r.read_string().unwrap();
                    let table = r.read_string().unwrap();
                    let org_table = r.read_string().unwrap();
                    let name = r.read_string().unwrap();
                    let org_name = r.read_string().unwrap();

                    println!("ALL catalog {}, schema {}, table {}, org_table {}, name {}, org_name {}",
                             catalog, schema, table, org_table, name, org_name);

                    let md = ColumnMetaData {
                        schema: schema,
                        table_name: table,
                        column_name: name
                    };

                    println!("column {} = {:?}", i, md);


                    column_meta.push(md);

                }

                //TODO: expect EOF packet in some versions of MySQL
                // let eof_packet = self.remote.read_packet().unwrap();
                // println!("eof_packet type = {}", eof_packet.packet_type());
                //
                // //assert!(0xfe == eof_packet.packet_type());
                //
                // write_buf.extend_from_slice(&eof_packet.header);
                // write_buf.extend_from_slice(&eof_packet.payload);

                // process row packets until ERR or EOF
                loop {
                    let row_packet = self.remote.read_packet().unwrap();
                    match row_packet.packet_type() {
                        // break on receiving Err_Packet, or EOF_Packet
                        0xfe | 0xff => {

                            println!("End of result rows");
                            write_buf.extend_from_slice(&row_packet.bytes);
                            break
                        },

                        _ => {
                            println!("Received row");

                            //TODO: if this result set does not contain any encrypted values
                            // then we can just write the packet straight to the client and
                            // skip all of this processing

                            //TODO do decryption here if required
                            let mut r = MySQLPacketParser::new(&row_packet);

                            let mut wtr: Vec<u8> = vec![];

                            for i in 0 .. field_count {
                                // let is_encrypted = false;

                                //println!("Value {} is {:?}", i, orig_value);

                                let column_config = self.config.get_column_config(
                                    &(column_meta[i as usize].schema),
                                    &(column_meta[i as usize].table_name),
                                    &(column_meta[i as usize].column_name));

                                println!("config is {:?}", column_config);

                                let value = match column_config {

                                    None => r.read_string(),
                                    Some(cc) => match cc {
                                        &ColumnConfig {ref encryption, ref native_type, ..} => {
                                            match native_type {
                                                &NativeType::U64 => {
                                                    match encryption {
                                                        &EncryptionType::NA => r.read_string(),
                                                        _ => Some(format!("{}", u64::decrypt(&r.read_bytes().unwrap(), encryption)))
                                                    }
                                                },
                                                &NativeType::Varchar(_) => {
                                                    match encryption {
                                                        &EncryptionType::NA => r.read_string(),
                                                        _ => Some(String::decrypt(&r.read_bytes().unwrap(), encryption))
                                                    }
                                                }
                                                _ => panic!("Native type {:?} not implemented", native_type)
                                            }
                                        }
                                    }

                                    /*match cc.encryption {
                                        EncryptionType::AES => orig_value,
                                        _ => orig_value
                                    }*/
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

//                            println!("Modified Header: {:?}", &new_header);
//                            println!("Modified Payload: {:?}", &wtr);

                            write_buf.extend_from_slice(&new_header);
                            write_buf.extend_from_slice(&wtr);

                        }
                    }
                }
            }
        }

        println!("Setting state to writing..");
        // let s = self.remote.read_to_end(&mut rBuf).unwrap();

        let buf_len = write_buf.len();
        let curs = Cursor::new(write_buf);

        // Transition the state to `Writing`, limiting the buffer to the
        // new line (inclusive).
        self.state = State::Writing(Take::new(curs, buf_len));

        println!("Set state to Writing");
        //TODO: remove bytes from buffer
        //TODO: do blocking read of mysql response packets

        // state is transitioned from `Reading` to `Writing`.
        //self.state.try_transition_to_writing();
    }

    pub fn write(&mut self, event_loop: &mut mio::EventLoop<Proxy>) {

        println!("Writing to client");

        // TODO: handle error
        match self.socket.try_write_buf(self.state.mut_write_buf()) {
            Ok(Some(_)) => {
                // If the entire line has been written, transition back to the
                // reading state
                self.state.try_transition_to_reading();

                // Re-register the socket with the event loop.
                self.reregister(event_loop);
            }
            Ok(None) => {
                // The socket wasn't actually ready, re-register the socket
                // with the event loop
                self.reregister(event_loop);
            }
            Err(e) => {
                panic!("got an error trying to write; err={:?}", e);
            }
        }
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
