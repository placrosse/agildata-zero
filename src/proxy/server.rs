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

    pub fn run(config: &Config, provider: &MySQLBackedSchemaProvider) {

        //env_logger::init().unwrap();

        // determine address for the proxy to bind to
        let bind_addr = env::args().nth(1).unwrap_or("127.0.0.1:3307".to_string());
        let bind_addr = bind_addr.parse::<SocketAddr>().unwrap();

        // determine address of the MySQL instance we are proxying for
        let mysql_addr = env::args().nth(2).unwrap_or("127.0.0.1:3306".to_string());
        let mysql_addr = mysql_addr.parse::<SocketAddr>().unwrap();

        // Create the tokio event loop that will drive this server
        let mut l = Core::new().unwrap();

        // Get a reference to the reactor event loop
        let handle = l.handle();

        // Create a TCP listener which will listen for incoming connections
        let socket = TcpListener::bind(&bind_addr, &l.handle()).unwrap();
        println!("Listening on: {}", bind_addr);

        // for each incoming connection
        let done = socket.incoming().for_each(move |(socket, _)| {

            // create a future to serve requests
            let future = TcpStream::connect(&mysql_addr, &handle).and_then(move |mysql| {
                Ok((socket, mysql))
            }).and_then(move |(client, server)| {
                Pipe::new(Rc::new(client), Rc::new(server), ZeroHandler {})
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

struct ZeroHandler {}

impl PacketHandler for ZeroHandler {

    fn handle_request(&self, p: &Packet) -> Action {
        Action::Forward
    }

    fn handle_response(&self, p: &Packet) -> Action {
        Action::Forward
    }
}

