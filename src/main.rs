#![feature(custom_derive, const_fn, inclusive_range_syntax, question_mark, box_syntax,
           stmt_expr_attributes, plugin)]

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;
extern crate log4rs;

extern crate chrono;
use chrono::*;

use std::thread::{sleep, spawn};
use std::{env, panic};
use std::sync::atomic::{Ordering, AtomicBool, ATOMIC_BOOL_INIT};

mod crypt;
use crypt::*;

extern crate config;
use config::*;

extern crate mysql_protocol;
use mysql_protocol::*;

extern crate mysql_proxy;
use mysql_proxy::*;

extern crate sql_parser;
use sql_parser::*;

static STOP: AtomicBool = ATOMIC_BOOL_INIT;
fn ask_stop() { STOP.store(true, Ordering::SeqCst) }
fn chk_stop() -> bool {
    // TODO FIXME XXX remove below for final build - to time-limit use of betas
    { assert!(UTC::now() < UTC.ymd(2017, 1, 1).and_hms(4, 0, 0), "CALL SUPPORT: LIMIT FAILURE"); }
    // TODO FIXME XXX remove above for final build
    STOP.load(Ordering::SeqCst)
}

fn x_sleep() { sleep(Duration::seconds(1).to_std().unwrap()); }

pub const APP_NAME: &'static str = "AgilData Babel Proxy";
pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[allow(deprecated)]
fn main() {
    env::set_var("RUST_BACKTRACE", "1");
    let _ = log4rs::init_file("babel.toml", Default::default());

    let app_ver: &str = &format!("{} v. {}", APP_NAME, VERSION);

    let ph = panic::take_hook();
    panic::set_hook(Box::new(move |pi| {
        ask_stop();
        x_sleep();
        ph(pi);
    }));

    {
        // {
            // let opt = opt.clone();
            // load(opt);  pre-start tasks to run to completion here
        // }

        let fs: Vec<fn()> = vec!(watch);
        for f in fs {
            let _ = spawn(move || { f(); });  // one thread per independent task
            x_sleep();
        }
    }

    info!("{} is up", app_ver);

    // create proxy
    Proxy::run("0.0.0.0", 6567);

    while !chk_stop() { x_sleep(); }

    x_sleep();
    info!("{} shutting down", app_ver);
    x_sleep();
    info!("{} is down", app_ver);
}
