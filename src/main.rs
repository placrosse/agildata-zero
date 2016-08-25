#![feature(custom_derive, const_fn, inclusive_range_syntax, question_mark,
           box_syntax, box_patterns, stmt_expr_attributes, plugin, integer_atomics)]

#[macro_use]
extern crate lazy_static;

extern crate parking_lot;

extern crate byteorder;

extern crate mio;
extern crate bytes;
extern crate chrono;

use std::env;

mod encrypt;

mod config;

mod protocol;

mod proxy;

mod parser;

pub const APP_NAME: &'static str = "AgilData Babel Proxy";
pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[allow(deprecated)]
fn main() {
    env::set_var("RUST_BACKTRACE", "1");

    let config_path = "example-babel-config.xml";
    let config = config::parse_config(config_path);

    proxy::server::Proxy::run(&config);

}
