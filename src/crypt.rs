extern crate parking_lot;
use self::parking_lot::Mutex;

extern crate regex;
use self::regex::Regex;

extern crate notify;
use self::notify::{RecommendedWatcher, Error, Watcher};

use chrono::*;

use std::io::prelude::*;
use std::fs::File;
use std::ops::Deref;
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

pub fn watch() {
    info!("encryption key monitor is up");
    let (tx, rx) = channel();
    let rw: Result<RecommendedWatcher, Error> = Watcher::new(tx);
    match rw {
        Ok(mut w) => {
            ld_keys();
            w.watch(FILE).expect(FILE);

            while !::chk_stop() {
                match rx.recv() {
                    Ok(e) => info!("key file notification {:?}", e),
                    Err(e) => error!("problem watching key file: {:?}", e),
                }
                ld_keys();
                ::x_sleep();
            }
        },
        Err(e) => panic!("{:?}", e),
    }
}

extern crate crypto;
use self::crypto::aes::KeySize;
use self::crypto::aes_gcm::AesGcm;
use self::crypto::aead::{AeadEncryptor, AeadDecryptor};

use std::iter::repeat;

fn mk_nonce(n0: u8, n1: u8) -> [u8; 12] {
    let now = UTC::now();
    let mut ts = now.timestamp();
    let mut tn = now.nanosecond();
    let mut nonce = [0u8; 12];

    trace!("mk_nonce: n0={:?}, n1={:?}, now={:?}, ts={:?} tn={:?} ts_hex={:#018x} tn_hex={:#010x}",
            n0, n1, now, ts, tn, ts, tn);

    for i in 0..6 { nonce[i] = (ts & 0xff) as u8; ts >>= 8; }
    nonce[6] = n0;
    nonce[7] = n1;
    for i in 0..4 { nonce[i + 8] = (tn & 0xff) as u8; tn >>= 8; }

    trace!("mk_nonce: nonce={:?}", nonce);

    nonce
}

pub fn encrypt(n0: u8, n1:u8, buf: &[u8]) -> Option<Vec<u8>> {
    let key: [u8; 32] = KEYS[n0 as usize].lock().deref().clone();
    debug!("encrypt: found key={:?} for n0={:?}", key, n0);
    if key == [0u8; 32] { return None; }

    let nonce = mk_nonce(n0, n1);
    assert!(nonce.len() == 12);

    let mut cipher = AesGcm::new(KeySize::KeySize256, &key, &nonce, &[]);

    let mut tag = [0u8; 16];
    let mut out: Vec<u8> = repeat(0).take(buf.len()).collect();
    cipher.encrypt(&buf, &mut out, &mut tag);
    trace!("encrypt: inp={:?} out={:?} tag={:?}", buf, out, tag);

    let mut bs = Vec::with_capacity(12 + out.len() + 16);
    for b in nonce.iter() { bs.push(*b); }
    bs.append(&mut out);
    for b in tag.iter() { bs.push(*b); }
    Some(bs)
}

pub fn decrypt(buf: &[u8]) -> Option<Vec<u8>> {
    if buf.len() < 36 { return None; }  // min size w/nonce & k & tag

    let n0 = buf[6];
    let key: [u8; 32] = KEYS[n0 as usize].lock().deref().clone();
    debug!("decrypt: found key={:?} for n0={:?}", key, n0);
    if key == [0u8; 32] { return None; }

    let iv: &[u8] = &buf[0..12];
    let mut decipher = AesGcm::new(KeySize::KeySize256, &key, &iv, &[]);
    let inp = &buf[12..buf.len() - 16];
    let tag = &buf[buf.len() - 16..];
    let mut out: Vec<u8> = repeat(0).take(buf.len() - 28).collect();
    decipher.decrypt(&inp, &mut out, &tag);
    trace!("decrypt: inp={:?} out={:?}", inp, out);
    Some(out)
}
