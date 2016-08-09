#![feature(recover, std_panic, panic_handler)]

extern crate mio;
extern crate bytes;
extern crate byteorder;
use byteorder::*;

use mio::{TryRead, TryWrite};
use mio::tcp::*;
use std::io::{Read, Write};
use mio::util::Slab;
use bytes::{Buf, Take};
use std::mem;
use std::io::Cursor;

extern crate sql_parser;
use sql_parser::sql_parser::*;
use sql_parser::sql_writer;

const SERVER: mio::Token = mio::Token(0);

#[derive(Debug)]
struct MySQLPacket {
    header: Vec<u8>,
    payload: Vec<u8>
}

impl MySQLPacket {

    fn sequence_id(&self) -> u8 {
        self.header[3]
    }

    fn packet_type(&self) -> u8 {
        match self.payload.len() {
            0 => 0,
            _ => self.payload[0]
        }
    }

}

fn parse_string(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("Invalid UTF-8")
}

struct MySQLPacketReader {
    payload: Vec<u8>,
    pos: usize
}

impl MySQLPacketReader {

    fn read_lenenc_str(&mut self) -> String {
        println!("read_lenenc_str BEGIN pos={}", self.pos);
        //TODO: HACK: assume single byte for length for now
        let n = self.payload[self.pos] as usize;
        self.pos += 1;

        println!("read_lenenc_str str_len={}", n);

        let s = parse_string(&self.payload[self.pos..self.pos+n]);
        self.pos += n;
        s

    }

    // fn read_bytes(len: usize) -> [u8] {
    //     panic!("not implemented");
    // }

}

fn read_packet_length(header: &[u8]) -> usize {
    (((header[2] as u32) << 16) |
    ((header[1] as u32) << 8) |
    header[0] as u32) as usize
}

pub struct Proxy {
    server: TcpListener,
    connections: Slab<Connection>,
}

impl Proxy {

    pub fn run(bind_host: String, bind_port: u16) {

        let bind_addr = format!("{}:{}", bind_host, bind_port);

        let address = &bind_addr.parse().unwrap();
        let server = TcpListener::bind(&address).unwrap();

        let mut event_loop = mio::EventLoop::new().unwrap();
        event_loop.register(&server, SERVER).unwrap();

        // Token `0` is reserved for the server socket. Tokens 1+ are used for
        // client connections. The slab is initialized to return Tokens
        // starting at 1.
        let slab = Slab::new_starting_at(mio::Token(1), 1024);

        let mut proxy = Proxy {
            server: server,
            connections: slab
        };

        println!("running MySQLProxy server on host {} port {}", bind_host, bind_port);
        event_loop.run(&mut proxy).unwrap();
    }
}

impl mio::Handler for Proxy {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self, event_loop: &mut mio::EventLoop<Proxy>, token: mio::Token, events: mio::EventSet) {
        match token {
            SERVER => {
                // Only receive readable events
                assert!(events.is_readable());

                println!("the server socket is ready to accept a connection");
                match self.server.accept() {
                    Ok(Some(socket)) => {
                        println!("accepted a new client socket");

                        // This will fail when the connection cap is reached
                        let token = self.connections
                            .insert_with(|token| Connection::new(socket, token))
                            .unwrap();

                        // Register the connection with the event loop.
                        event_loop.register_opt(
                            &self.connections[token].socket,
                            token,
                            mio::EventSet::readable() | mio::EventSet::writable(),
                            //mio::PollOpt::edge() | mio::PollOpt::oneshot()
                            mio::PollOpt::level()
                        ).unwrap();
                    }
                    Ok(None) => {
                        println!("the server socket wasn't actually ready");
                    }
                    Err(e) => {
                        println!("encountered error while accepting connection; err={:?}", e);
                        event_loop.shutdown();
                    }
                }
            }
            _ => {
                self.connections[token].ready(event_loop, events);

                // If handling the event resulted in a closed socket, then
                // remove the socket from the Slab. This will result in all
                // resources being freed.
                if self.connections[token].is_closed() {
                    let _ = self.connections.remove(token);
                }
            }
        }
    }
}

trait MySQLConnection {
    fn read_packet(&mut self) -> Result<MySQLPacket, &'static str>;
}

impl MySQLConnection for std::net::TcpStream {

    fn read_packet(&mut self) -> Result<MySQLPacket, &'static str> {

        println!("read_packet() BEGIN");

        // read header
        let mut header_vec = vec![0_u8; 4];
        match self.read(&mut header_vec) {
            Ok(0) => Ok(MySQLPacket { header: vec![], payload: vec![] }),
            Ok(n) => {
                assert!(n==4);

                let payload_len = read_packet_length(&header_vec);

                // read payload
                let mut payload_vec = vec![0_u8; payload_len];
                assert!(payload_len == self.read(&mut payload_vec).unwrap());

                println!("read_packet() END");

                Ok(MySQLPacket { header: header_vec, payload: payload_vec })
            },
            Err(_) => Err("oops")
        }
    }
}


#[derive(Debug)]
struct Connection {
    socket: TcpStream,
    token: mio::Token,
    state: State,
    remote: std::net::TcpStream,
    //authenticating: bool
}

impl Connection {
    fn new(socket: TcpStream, token: mio::Token) -> Connection {
        println!("Creating remote connection...");
        // let ip  = std::net::Ipv4Addr::new(127,0,0,1);
        // let saddr = std::net::SocketAddr::new(std::net::IpAddr::V4(ip), 3306);
        // let mut tcps = TcpStream::connect(&saddr).unwrap();

        // connect to real MySQL
        let mut realtcps = std::net::TcpStream::connect("127.0.0.1:3306").unwrap();

        // read header
        let auth_packet = realtcps.read_packet().unwrap();

        let mut response: Vec<u8> = Vec::new();
        response.extend_from_slice(&auth_packet.header);
        response.extend_from_slice(&auth_packet.payload);

        let buf = Cursor::new(response);

        println!("Created new connection in Writing state");

        Connection {
            socket: socket,
            token: token,
            state: State::Writing(Take::new(buf, auth_packet.payload.len()+4)),
            remote: realtcps,
            // authenticating: true
        }
    }

    fn ready(&mut self, event_loop: &mut mio::EventLoop<Proxy>, events: mio::EventSet) {
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

    fn read(&mut self, event_loop: &mut mio::EventLoop<Proxy>) {

        println!("Reading from client");

        let mut buf = Vec::with_capacity(1024);
        match self.socket.try_read_buf(&mut buf) {
            Ok(Some(0)) => {
                self.state = State::Closed;
            }
            Ok(Some(n)) => {
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

                // do we have the complete request packet yet?
                if buf.len() > 3 {

                    let packet_len = read_packet_length(&buf);

                    println!("incoming packet_len = {}", packet_len);
                    println!("Buf len {}", buf.len());

                    if buf.len() >= packet_len+4 {
                        match buf[4] {
                            0x03 => {
                                println!("0x03");

                                let query = parse_string(&buf[5 as usize .. (packet_len+4) as usize]);
                                println!("QUERY : {:?}", query);

                                // mutate with builder

                                // parse query
                                let parser = AnsiSQLParser {};
                                let result = parser.parse(&query);

                                let parsed: Option<SQLExpr> = match result {
                                    Ok(p) => Some(p),
                                    Err(e) => {
                                        println!("Failed to parse due to {:?}", e);
                                        None
                                    }
                                };

                                // visit and conditionally encrypt query

                                // reqwrite query
                                if parsed.is_some() {
                                    let rewritten = sql_writer::write(parsed.unwrap());
                                    println!("REWRITTEN {:?}", rewritten);

                                    // write packed with new query
                                    //let n_buf: Vec<u8> = Vec::new();
                                    let slice: &[u8] = rewritten.as_bytes();

                                    let mut wtr: Vec<u8> = vec![];
                                    wtr.write_u32::<LittleEndian>(slice.len() as u32).unwrap();
                                    assert!(0x00 == wtr.pop().unwrap());
                                    wtr.push(0x03); // packet type for COM_Query
                                    wtr.extend_from_slice(slice);

                                    self.mysql_send(&wtr);

                                } else {
                                    self.mysql_send(&buf[0 .. packet_len+4]);
                                }

                            },
                            _ => {
                                self.mysql_send(&buf[0 .. packet_len+4]);
                            }
                        }
                        // self.mysql_send(&buf[0 .. packet_len+4]);

                        //self.authenticating = false;

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
                write_buf.extend_from_slice(&packet.header);
                write_buf.extend_from_slice(&packet.payload);
            },
            0xfb => panic!("not implemented"),
            _ => {

                println!("Got field_count packet");

                // first packet is field count
                write_buf.extend_from_slice(&packet.header);
                write_buf.extend_from_slice(&packet.payload);

                //TODO: this assumes < 251 fields in result set
                let field_count = packet.payload[0] as u32;

                println!("Result set has {} columns", field_count);

                for i in 0 .. field_count {

                    let field_packet = self.remote.read_packet().unwrap();
                    write_buf.extend_from_slice(&field_packet.header);
                    write_buf.extend_from_slice(&field_packet.payload);

                    let mut r = MySQLPacketReader { payload: field_packet.payload, pos: 0 };

                    let catalog = r.read_lenenc_str();
                    let schema = r.read_lenenc_str();
                    let table = r.read_lenenc_str();
                    let org_table = r.read_lenenc_str();
                    let name = r.read_lenenc_str();
                    let org_name = r.read_lenenc_str();

                    println!("column {}: table={}, column={}", i, table, name);

                }

                // expect EOF packet
                let eof_packet = self.remote.read_packet().unwrap();
                println!("eof_packet type = {}", eof_packet.packet_type());

                //assert!(0xfe == eof_packet.packet_type());

                write_buf.extend_from_slice(&eof_packet.header);
                write_buf.extend_from_slice(&eof_packet.payload);

                // process row packets until ERR or EOF
                loop {
                    let row_packet = self.remote.read_packet().unwrap();
                    match row_packet.packet_type() {
                        // break on receiving Err_Packet, or EOF_Packet
                        0xfe | 0xff => {

                            println!("End of result rows");
                            write_buf.extend_from_slice(&row_packet.header);
                            write_buf.extend_from_slice(&row_packet.payload);
                            break
                        },

                        _ => {
                            println!("Received row");

                            //TODO do decryption here if required

                            write_buf.extend_from_slice(&row_packet.header);
                            write_buf.extend_from_slice(&row_packet.payload);
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

    fn write(&mut self, event_loop: &mut mio::EventLoop<Proxy>) {

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

    fn reregister(&self, event_loop: &mut mio::EventLoop<Proxy>) {
        event_loop.reregister(&self.socket, self.token, self.state.event_set(), mio::PollOpt::oneshot())
            .unwrap();
    }

    fn is_closed(&self) -> bool {
        match self.state {
            State::Closed => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
enum State {
    Reading(Vec<u8>),
    Writing(Take<Cursor<Vec<u8>>>),
    Closed,
}

impl State {
    fn mut_read_buf(&mut self) -> &mut Vec<u8> {
        match *self {
            State::Reading(ref mut buf) => buf,
            _ => panic!("connection not in reading state"),
        }
    }

    fn read_buf(&self) -> &[u8] {
        match *self {
            State::Reading(ref buf) => buf,
            _ => panic!("connection not in reading state"),
        }
    }

    fn write_buf(&self) -> &Take<Cursor<Vec<u8>>> {
        match *self {
            State::Writing(ref buf) => buf,
            _ => panic!("connection not in writing state"),
        }
    }

    fn mut_write_buf(&mut self) -> &mut Take<Cursor<Vec<u8>>> {
        match *self {
            State::Writing(ref mut buf) => buf,
            _ => panic!("connection not in writing state"),
        }
    }

    // Looks for a new line, if there is one the state is transitioned to
    // writing
    fn try_transition_to_writing(&mut self) {
        if let Some(pos) = self.read_buf().iter().position(|b| *b == b'\n') {
            // First, remove the current read buffer, replacing it with an
            // empty Vec<u8>.
            let buf = mem::replace(self, State::Closed)
                .unwrap_read_buf();

            // Wrap in `Cursor`, this allows Vec<u8> to act as a readable
            // buffer
            let buf = Cursor::new(buf);

            // Transition the state to `Writing`, limiting the buffer to the
            // new line (inclusive).
            *self = State::Writing(Take::new(buf, pos + 1));
        }
    }

    // If the buffer being written back to the client has been consumed, switch
    // back to the reading state. However, there already might be another line
    // in the read buffer, so `try_transition_to_writing` is called as a final
    // step.
    fn try_transition_to_reading(&mut self) {
        if !self.write_buf().has_remaining() {
            let cursor = mem::replace(self, State::Closed)
                .unwrap_write_buf()
                .into_inner();

            let pos = cursor.position();
            let mut buf = cursor.into_inner();

            // Drop all data that has been written to the client
            drain_to(&mut buf, pos as usize);

            *self = State::Reading(buf);

            // Check for any new lines that have already been read.
            self.try_transition_to_writing();
        }
    }

    fn event_set(&self) -> mio::EventSet {
        match *self {
            State::Reading(..) => mio::EventSet::readable(),
            State::Writing(..) => mio::EventSet::writable(),
            _ => mio::EventSet::none(),
        }
    }

    fn unwrap_read_buf(self) -> Vec<u8> {
        match self {
            State::Reading(buf) => buf,
            _ => panic!("connection not in reading state"),
        }
    }

    fn unwrap_write_buf(self) -> Take<Cursor<Vec<u8>>> {
        match self {
            State::Writing(buf) => buf,
            _ => panic!("connection not in writing state"),
        }
    }
}


fn drain_to(vec: &mut Vec<u8>, count: usize) {
    // A very inefficient implementation. A better implementation could be
    // built using `Vec::drain()`, but the API is currently unstable.
    for _ in 0..count {
        vec.remove(0);
    }
}
