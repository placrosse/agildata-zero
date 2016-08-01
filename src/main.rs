#![feature(custom_derive, const_fn, inclusive_range_syntax, question_mark, box_syntax,
           collections_bound, btree_range, stmt_expr_attributes, integer_atomics, plugin)]

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;
extern crate log4rs;

extern crate argparse;
use argparse::{ArgumentParser, Store, StoreTrue};

// extern crate rand;

extern crate chrono;
use chrono::*;

use std::thread::{sleep, spawn};
use std::{env, panic};
use std::path::Path;
use std::fs::remove_file;
use std::sync::atomic::{Ordering, AtomicBool, ATOMIC_BOOL_INIT};

static STOP: AtomicBool = ATOMIC_BOOL_INIT;
fn ask_stop() { STOP.store(true, Ordering::SeqCst) }
fn chk_stop() -> bool {
    // TODO FIXME XXX remove below for final build - to time-limit use of betas
    { assert!(UTC::now() < UTC.ymd(2017, 1, 1).and_hms(4, 0, 0), "CALL SUPPORT: LIMIT FAILURE"); }
    // TODO FIXME XXX remove above for final build
    STOP.load(Ordering::SeqCst)
}

fn x_sleep() { sleep(Duration::seconds(15).to_std().unwrap()); }

pub const APP_NAME: &'static str = "AgilData Babel Proxy";
pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug)]
pub struct Opts {
    pub host: String,
    pub port: u16,
}

#[allow(deprecated)]
fn main() {
    env::set_var("RUST_BACKTRACE", "1");
    let _ = log4rs::init_file("babel.toml", Default::default());

    let app_ver: &str = &format!("{} v. {}", APP_NAME, VERSION);

    let mut opt = Opts {
        host: String::from(""),
        port: 0,
    };

    {
        let mut ap = ArgumentParser::new();
        ap.set_description(&app_ver);
        ap.refer(&mut opt.host)
            .required()
            .add_option(&["--host"], Store,
            "host name for MySQL server");
        ap.refer(&mut opt.port)
            .add_option(&["--port"], Store,
            "port number for MySQL server");
        ap.parse_args_or_exit();
    }

    let ph = panic::take_hook();
    panic::set_hook(Box::new(move |pi| {
        ask_stop();
        x_sleep();
        ph(pi);
    }));

    // {
        // {
            // let opt = opt.clone();
            // load(opt);  pre-start tasks to run to completion here
        // }

        // let fs: Vec<fn(Opts)> = vec!(funcA, funcB, funcC, funcD);
        // for f in fs {
        //     let opt = opt.clone();
        //     let _ = spawn(move || { f(opt); });  // one thread per independent task
        //     x_sleep();
        // }
    // }

    info!("{} is up", app_ver);

    while !chk_stop() { x_sleep(); }

    x_sleep();
    info!("{} shutting down", app_ver);
    x_sleep();
    info!("{} is down", app_ver);
}
