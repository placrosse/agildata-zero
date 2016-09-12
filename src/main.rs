#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]
#![cfg_attr(feature="clippy", deny(warnings))]
#![feature(inclusive_range_syntax, question_mark, box_syntax, box_patterns, integer_atomics)]

extern crate argparse;
use argparse::{ArgumentParser, Store, StoreTrue};

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate byteorder;
extern crate mysql_proxy;
#[macro_use]
extern crate futures;
#[macro_use]
extern crate tokio_core;

extern crate bytes;

extern crate mysql;

use std::env;
use std::str;
use std::rc::Rc;

mod encrypt;
mod config;
mod protocol;
mod proxy;

mod query;

pub const APP_NAME: &'static str = "AgilData Zero Gateway";
pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug)]
pub struct Opts {
    pub ver: bool,
    pub cfg: String,
}

fn main() {
    env::set_var("RUST_BACKTRACE", "1");
    drop(env_logger::init());

    let mut opt = Opts {
        ver: false,
        cfg: String::from("zero-config.xml"),
    };

    let dsc = format!("{} version {}", APP_NAME, VERSION);
    {
        let mut ap = ArgumentParser::new();
        ap.set_description(&dsc);
        ap.refer(&mut opt.ver)
            .add_option(&["-V", "--version"], StoreTrue,
            "show version number and exit");
        ap.refer(&mut opt.cfg)
            .add_option(&["-C", "--config"], Store,
            "path to configuration file defaults to ./zero-config.xml");
        ap.parse_args_or_exit();
    }

    if opt.ver {
        println!("{}", dsc);
    } else {
        let config = config::parse_config(&opt.cfg);
        let config = Rc::new(config);
        let provider = proxy::schema_provider::MySQLBackedSchemaProvider::new(config.clone());
        proxy::server::Proxy::run(config, Rc::new(provider));
    }
}
