use mio::{self};
use mio::tcp::*;

use mio::util::Slab;
use bytes::{Buf, Take};
use std::mem;
use std::io::Cursor;
use std::str::FromStr;

const SERVER: mio::Token = mio::Token(0);

use config::{Config, TConfig};

use super::mysql::MySQLConnectionHandler;
use super::schema_provider::MySQLBackedSchemaProvider;

pub struct Proxy<'a> {
    server: TcpListener,
    connections: Slab<MySQLConnectionHandler<'a>>,
    config: &'a Config,
    provider: &'a MySQLBackedSchemaProvider<'a>
}

impl<'a> Proxy<'a> {

    pub fn run(config: &Config, provider: &MySQLBackedSchemaProvider) {

        let bind_host = config.get_client_config().props.get("host").unwrap().clone();
        let bind_port = u16::from_str(config.get_client_config().props.get("port").unwrap()).unwrap();
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
            connections: slab,
            config: config,
            provider: provider
        };

        println!("running MySQLProxy server on host {} port {}", bind_host, bind_port);
        event_loop.run(&mut proxy).unwrap();
    }
}

impl<'a> mio::Handler for Proxy<'a> {
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
                        let config = self.config;
                        let provider = self.provider;
                        let token = self.connections
                            .insert_with(|token| MySQLConnectionHandler::new(socket, token, config, provider))
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
                // new data is available to read from a client connection
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
pub enum State {
    Reading(Vec<u8>),
    Writing(Take<Cursor<Vec<u8>>>),
    Closed,
}

impl State {
    // fn mut_read_buf(&mut self) -> &mut Vec<u8> {
    //     match *self {
    //         State::Reading(ref mut buf) => buf,
    //         _ => panic!("connection not in reading state"),
    //     }
    // }

    pub fn read_buf(&self) -> &[u8] {
        match *self {
            State::Reading(ref buf) => buf,
            _ => panic!("connection not in reading state"),
        }
    }

    pub fn write_buf(&self) -> &Take<Cursor<Vec<u8>>> {
        match *self {
            State::Writing(ref buf) => buf,
            _ => panic!("connection not in writing state"),
        }
    }

    pub fn mut_write_buf(&mut self) -> &mut Take<Cursor<Vec<u8>>> {
        match *self {
            State::Writing(ref mut buf) => buf,
            _ => panic!("connection not in writing state"),
        }
    }

    // Looks for a new line, if there is one the state is transitioned to
    // writing
    pub fn try_transition_to_writing(&mut self) {
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
    pub fn try_transition_to_reading(&mut self) {
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

    pub fn event_set(&self) -> mio::EventSet {
        match *self {
            State::Reading(..) => mio::EventSet::readable(),
            State::Writing(..) => mio::EventSet::writable(),
            _ => mio::EventSet::none(),
        }
    }

    pub fn unwrap_read_buf(self) -> Vec<u8> {
        match self {
            State::Reading(buf) => buf,
            _ => panic!("connection not in reading state"),
        }
    }

    pub fn unwrap_write_buf(self) -> Take<Cursor<Vec<u8>>> {
        match self {
            State::Writing(buf) => buf,
            _ => panic!("connection not in writing state"),
        }
    }
}


pub fn drain_to(vec: &mut Vec<u8>, count: usize) {
    // A very inefficient implementation. A better implementation could be
    // built using `Vec::drain()`, but the API is currently unstable.
    for _ in 0..count {
        vec.remove(0);
    }
}
