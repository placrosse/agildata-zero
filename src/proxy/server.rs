use mysql_proxy::*;

use futures::{Future};
use futures::stream::Stream;
use tokio_core::net::{TcpStream, TcpListener};
use tokio_core::reactor::{Core};

use bytes::{Buf, Take};
use std::mem;
use std::net::{SocketAddr};
use std::io::Cursor;
use std::str::FromStr;
use std::env;
use std::rc::Rc;
use config::{Config, TConfig};

use super::schema_provider::MySQLBackedSchemaProvider;

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
        let default_port = &String::from("3306");
        let conn_port = conn.props.get("port").unwrap_or(default_port);
        let conn_addr = format!("{}:{}",conn_host,conn_port);
        let bind_addr = conn_addr.parse::<SocketAddr>().unwrap();

        // determine address of the MySQL instance we are proxying for
        let conn = temp.get_connection_config();
        let conn_host = conn.props.get("host").unwrap();
        let default_port = &String::from("3306");
        let conn_port = conn.props.get("port").unwrap_or(default_port);
        let conn_addr = format!("{}:{}",conn_host,conn_port);
        let mysql_addr = conn_addr.parse::<SocketAddr>().unwrap();

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
            //TODO: process request
            Action::Forward
        }
    }

    fn handle_response(&mut self, p: &Packet) -> Action {
        Action::Forward
    }
}

