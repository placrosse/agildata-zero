extern crate mio;
extern crate bytes;
extern crate byteorder;
use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian, LittleEndian};

use mio::{TryRead, TryWrite};
use mio::tcp::*;
use std::io::{Read, Write};
use mio::util::Slab;
use bytes::{Buf, Take};
use std::mem;
use std::io::Cursor;

const SERVER: mio::Token = mio::Token(0);
const MAX_LINE: usize = 128;

struct Pong {
    server: TcpListener,
    connections: Slab<Connection>,
}

impl Pong {
    fn new(server: TcpListener) -> Pong {
        // Token `0` is reserved for the server socket. Tokens 1+ are used for
        // client connections. The slab is initialized to return Tokens
        // starting at 1.
        let slab = Slab::new_starting_at(mio::Token(1), 1024);

        Pong {
            server: server,
            connections: slab,
        }
    }
}

impl mio::Handler for Pong {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self, event_loop: &mut mio::EventLoop<Pong>, token: mio::Token, events: mio::EventSet) {
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

#[derive(Debug)]
struct Connection {
    socket: TcpStream,
    token: mio::Token,
    state: State,
    remote: std::net::TcpStream,
    authenticating: bool
}

impl Connection {
    fn new(socket: TcpStream, token: mio::Token) -> Connection {
        println!("Creating remote connection...");
        // let ip  = std::net::Ipv4Addr::new(127,0,0,1);
        // let saddr = std::net::SocketAddr::new(std::net::IpAddr::V4(ip), 3306);
        // let mut tcps = TcpStream::connect(&saddr).unwrap();

        let mut header = vec![0_u8; 3];
        let mut realtcps = std::net::TcpStream::connect("127.0.0.1:3306").unwrap();
        realtcps.read(&mut header).unwrap();
        println!("Header read is {:?}", header);

        let packet_len: u32 =
            ((header[2] as u32) << 16) |
            ((header[1] as u32) << 8) |
            header[0] as u32;

        let mut vec = vec![0_u8; (packet_len + 1) as usize ];
        let mut payload = vec.as_mut_slice();
        realtcps.read(&mut payload);

        let mut response: Vec<u8> = Vec::new();
        response.extend_from_slice(&header);
        response.extend_from_slice(&payload);

        let buf = Cursor::new(response);

        println!("Created new connection in Writing state");

        Connection {
            socket: socket,
            token: token,
            state: State::Writing(Take::new(buf, (packet_len+4) as usize)),
            remote: realtcps,
            authenticating: true
        }
    }

    fn ready(&mut self, event_loop: &mut mio::EventLoop<Pong>, events: mio::EventSet) {
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

    fn read(&mut self, event_loop: &mut mio::EventLoop<Pong>) {

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
                //println!("bytes read {:#04x}", buf);

                if self.authenticating {

                    let mut i: usize = 32;
                    // skip these 32 bytes
                    // 2 bytes: client mask. Example: 8d a2
    	            // 2 bytes: extended client capabilities. Example: 00 00
    	            // 4 bytes: Max packet size (4 byte int).
                    // 1 byte: Character set e.g. 08
                    // 23 bytes: Empty 23 null bytes

                    // username (null-terminated)
                    while buf[i] != 0x00 {
                        i += 1;
                    }
                    i += 1;
                    //println!("username = {}", &buf[32..i]);

                    let password_len = buf[i] as usize;
                    i += password_len;

                    // let mut rdr = Cursor::new(buf[packet_len .. packet_len+]
                    // let username_len = rdr.read_u16::<BigEndian>().unwrap()

                    println!("login packet len = {}", i);

                    self.mysql_send(&buf[0..i]);

                    self.authenticating = false;

                } else {

                    // do we have the complete request packet yet?
                    if buf.len() > 3 {

                        let packet_len: u32 =
                            ((buf[2] as u32) << 16) |
                            ((buf[1] as u32) << 8) |
                            buf[0] as u32;
                        println!("incoming command packet_len = {}", packet_len);
                        println!("Buf len {}", buf.len());

                        if buf.len() >= (packet_len+4) as usize {
                            self.mysql_send(&buf[0 .. (packet_len+4) as usize]);

                        } else {
                            println!("do not have full packet!");
                        }

                    } else {
                        println!("do not have full header!");
                    }
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
        self.remote.write(request);
        self.remote.flush();

        println!("Reading from MySQL...");
        let mut rBuf: Vec<u8> = Vec::new();
        let mut pLen: usize = 0;
        loop {
            println!("Entering remote read loop..");
            let mut h = [0_u8; 3];
            self.remote.read(&mut h).unwrap();

            let h_len: u32 =
                ((h[2] as u32) << 16) |
                ((h[1] as u32) << 8) |
                h[0] as u32;

            let mut pVec: Vec<u8> = vec![0_u8; (h_len + 1) as usize];
            let mut p = pVec.as_mut_slice();

            println!("DEBUG hlen={:?}, pLen = {:?}",h_len, p.len());
            pLen += self.remote.read(&mut p).unwrap();

            println!("First of payload is {}", p[0]);
            rBuf.extend_from_slice(&h);
            rBuf.extend_from_slice(p);
            if p[0] == 0x00 || p[0] == 0xfe || p[0] == 0xff {
                break;
            }
        }

        println!("Setting state to writing..");
        // let s = self.remote.read_to_end(&mut rBuf).unwrap();

        let curs = Cursor::new(rBuf);

        // Transition the state to `Writing`, limiting the buffer to the
        // new line (inclusive).
        self.state = State::Writing(Take::new(curs, pLen));

        println!("Set state to Writing");
        //TODO: remove bytes from buffer
        //TODO: do blocking read of mysql response packets

        // state is transitioned from `Reading` to `Writing`.
        //self.state.try_transition_to_writing();
    }

    fn write(&mut self, event_loop: &mut mio::EventLoop<Pong>) {

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

    fn reregister(&self, event_loop: &mut mio::EventLoop<Pong>) {
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

fn main() {
    let address = "0.0.0.0:6567".parse().unwrap();
    let server = TcpListener::bind(&address).unwrap();

    let mut event_loop = mio::EventLoop::new().unwrap();
    event_loop.register(&server, SERVER).unwrap();

    let mut pong = Pong::new(server);

    println!("running pingpong server; port=6567");
    event_loop.run(&mut pong).unwrap();
}

fn drain_to(vec: &mut Vec<u8>, count: usize) {
    // A very inefficient implementation. A better implementation could be
    // built using `Vec::drain()`, but the API is currently unstable.
    for _ in 0..count {
        vec.remove(0);
    }
}
