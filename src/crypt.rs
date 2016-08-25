extern crate parking_lot;
use self::parking_lot::Mutex;

extern crate regex;
use self::regex::Regex;

extern crate notify;
use self::notify::{RecommendedWatcher, Error, Watcher};


use std::io::prelude::*;
use std::fs::File;
use std::sync::mpsc::channel;

// pub enum EncScheme {
//
// }
//
// trait Encryptable {
//     fn encrypt(&self, scheme: EncScheme) -> Option<Vec<u8>>;
//     fn decrypt(&self, scheme: EncScheme) -> Option<Vec<u8>>;
// }
//
// impl Encryptable for String { …. }
//
// if let Some(scheme) = get_enc_scheme(table, col) {
//
// let new_value = value.encrypt(scheme);
//
// }
//


const FILE: &'static str = "./babel.key";

lazy_static! {
    static ref KEYS: Vec<Mutex<[u8; 32]>> = {
        let mut ks = Vec::with_capacity(256);
        for _ in 0..256 { ks.push(Mutex::new([0u8; 32])); }
        ks
    };
}

fn ld_keys() {
    warn!("READING ENCRYPTION KEYS from {:?}", FILE);
    for i in 0..256 { let mut k = KEYS[i].lock(); for j in 0..32 { k[j] = 0u8; } }

    let mut f = File::open(FILE).expect("unable to open key file for AES encrypt/decrypt");
    let mut s = String::new();
    let _ = f.read_to_string(&mut s).expect("unable to read key file for AES encrypt/decrypt");

    let re = Regex::new(r"(?m)^\s*([:digit:]{1,3})\s*=\s*([:xdigit:]{64})\s*$").unwrap();
    for c in re.captures_iter(&s) {
        let i: u8 = c.at(1).unwrap().parse().unwrap();
        let hex = c.at(2).unwrap();
        let mut k = KEYS[i as usize].lock();
        let mut m = 0;
        let mut b = 0;

        for (j, v) in hex.bytes().enumerate() {
            b <<= 4;
            match v {
                b'a'...b'f' => b |= v - b'a' + 10,
                b'A'...b'F' => b |= v - b'A' + 10,
                b'0'...b'9' => b |= v - b'0',
                _ => panic!("ld_keys.hex"),
            }
            m += 1;
            if m == 2 {
                m = 0;
                k[(j / 2) as usize] = b;
            }
        }
    }
}

// pub fn watch() {
//     info!("encryption key monitor is up");
//     let (tx, rx) = channel();
//     let rw: Result<RecommendedWatcher, Error> = Watcher::new(tx);
//     match rw {
//         Ok(mut w) => {
//             ld_keys();
//             w.watch(FILE).expect(FILE);
//
//             while !::chk_stop() {
//                 match rx.recv() {
//                     Ok(e) => info!("key file notification {:?}", e),
//                     Err(e) => error!("problem watching key file: {:?}", e),
//                 }
//                 ld_keys();
//                 ::x_sleep();
//             }
//         },
//         Err(e) => panic!("{:?}", e),
//     }
// }
