#![deny(warnings)]#![feature(inclusive_range_syntax, question_mark, box_syntax, box_patterns, integer_atomics)]

pub const APP_NAME: &'static str = "AgilData Zero Gateway";
pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate byteorder;
extern crate mio;
extern crate bytes;

use std::env;
use std::str;

mod encrypt;
mod config;
mod protocol;
mod proxy;

mod query;
fn main() {
    env::set_var("RUST_BACKTRACE", "1");
    drop(env_logger::init());

    let config_path = "example-zero-config.xml";
    let config = config::parse_config(config_path);
    proxy::server::Proxy::run(&config);
}
