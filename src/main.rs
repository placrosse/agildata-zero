#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]
#![cfg_attr(feature="clippy", deny(warnings))]
#![feature(inclusive_range_syntax, question_mark, box_syntax, box_patterns, integer_atomics)]

extern crate argparse;
use argparse::{ArgumentParser, Store, StoreTrue};

#[macro_use]
extern crate log;
extern crate log4rs;

extern crate byteorder;
extern crate mysql_proxy;
#[macro_use]
extern crate futures;
#[macro_use]
extern crate tokio_core;

extern crate bytes;

extern crate mysql;

extern crate chrono;

#[macro_use]
extern crate decimal;

use std::str;
use std::rc::Rc;
use std::process;

mod encrypt;
mod config;
mod proxy;
mod error;
mod query;

pub const APP_NAME: &'static str = "AgilData Zero Gateway";
pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug)]
pub struct Opts {
    pub ver: bool,
    pub cfg: String,
    pub log_cfg: String,
}

fn main() {
    // env::set_var("RUST_BACKTRACE", "1");

    let mut opt = Opts {
        ver: false,
        cfg: String::from("zero-config.xml"),
        log_cfg: String::from("log.toml"),
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
        ap.refer(&mut opt.log_cfg)
            .add_option(&["-L", "--logconfig"], Store,
            "path to logging configuration file defaults to ./log.toml");
        ap.parse_args_or_exit();
    }

    if opt.ver {
        println!("{}", dsc);
        process::exit(0);
    }

    if log4rs::init_file(&opt.log_cfg, Default::default()).is_err() {
        println!("Unable to open logging configuration file: {}", opt.log_cfg);
        process::exit(1);
    }

    info!("{}", dsc);

    let config = config::parse_config(&opt.cfg);
    let config = Rc::new(config);
    let provider = proxy::schema_provider::MySQLBackedSchemaProvider::new(config.clone());
    let stmt_cache = proxy::statement_cache::StatementCache::new();
    proxy::server::Proxy::run(config, Rc::new(provider), Rc::new(stmt_cache));

}
