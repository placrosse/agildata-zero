extern crate crypto;
use self::crypto::aes::KeySize;
use self::crypto::aes_gcm::AesGcm;
use self::crypto::aead::{AeadEncryptor, AeadDecryptor};
use std::error::Error;
use std::iter::repeat;

use byteorder::{WriteBytesExt,ReadBytesExt,BigEndian};
use std::io::Cursor;

#[derive(Debug, PartialEq, Clone)]
pub enum EncryptionType {
	AES,
	OPE,
	NA,
}



#[derive(Debug, PartialEq, Clone)]
pub enum NativeType {
	U64,
	Varchar(u32),
	F64,
}

pub trait Encrypt {
	fn encrypt(self, scheme: &EncryptionType) -> Result<Vec<u8>, Box<Error>>;
}

pub trait Decrypt {
    type DecType;
	fn decrypt(value: &[u8], scheme: &EncryptionType) -> Result<Self::DecType,  Box<Error>>;
}

impl Decrypt for u64 {
    type DecType = u64;
	fn decrypt(value: &[u8], scheme: &EncryptionType) ->  Result<u64,  Box<Error>> {
		match scheme {
			&EncryptionType::AES => {
				let mut decrypted = try!(decrypt(&get_key(), value));
                Ok(Cursor::new(decrypted).read_u64::<BigEndian>().unwrap())
			},
			&EncryptionType::NA => panic!("This should be handled outside this method for now..."),
			_ => panic!("Not implemented")
		}
	}
}

impl Decrypt for String {
    type DecType = String;
	fn decrypt(value: &[u8], scheme: &EncryptionType) ->  Result<String,  Box<Error>>{
        match scheme {
			&EncryptionType::AES => {
				let mut decrypted = try!(decrypt(&get_key(), value));
				Ok(String::from_utf8(decrypted).expect("Invalid UTF-8"))

			},
			&EncryptionType::NA => panic!("This should be handled outside this method for now..."),
			_ => panic!("Not implemented")
		}
	}
}

impl Encrypt for u64 {
	fn encrypt(self, scheme: &EncryptionType) -> Result<Vec<u8>, Box<Error>> {

		match scheme {
			&EncryptionType::AES => {
				let mut buf: Vec<u8> = Vec::new();
				buf.write_u64::<BigEndian>(self).unwrap();

				encrypt(&get_key(), &buf)
			},
			&EncryptionType::NA => panic!("This should be handled outside this method for now..."),
			_ => panic!("Not implemented")
		}

	}
}

impl Encrypt for String {
	fn encrypt(self, scheme: &EncryptionType) -> Result<Vec<u8>, Box<Error>> {
		match scheme {
			&EncryptionType::AES => {
				let buf = self.as_bytes();
				println!("Buf length = {}", buf.len());
				let e = encrypt(&get_key(), &buf).unwrap();
				println!("Encrypted length = {}", e.len());
				Ok(e)
			},
			&EncryptionType::NA => panic!("This should be handled outside this method for now..."),
			_ => panic!("Not implemented")
		}
	}
}

fn get_key() -> [u8; 32] {
	let hex = "44E6884D78AA18FA690917F84145AA4415FC3CD560915C7AE346673B1FDA5985";
	let mut k = [0_u8; 32];
	let mut m = 0;
	let mut b = 0;

	for (j, v) in hex.bytes().enumerate() {
		b <<= 4;
		match v {
			b'a'...b'f' => b |= v - b'a' + 10,
			b'A'...b'F' => b |= v - b'A' + 10,
			b'0'...b'9' => b |= v - b'0',
			_ => panic!("get_key.hex"),
		}
		m += 1;
		if m == 2 {
			m = 0;
			k[(j / 2) as usize] = b;
		}
	}

	k
}

pub fn encrypt(key: &[u8], buf: &[u8]) -> Result<Vec<u8>, Box<Error>> {
    // let key: [u8; 32] = KEYS[n0 as usize].lock().deref().clone();
    // debug!("encrypt: found key={:?} for n0={:?}", key, n0);
    // if key == [0u8; 32] { return None; }


    let nonce = [0_u8;12];
    let mut cipher = AesGcm::new(KeySize::KeySize256, key, &nonce, &[]);

    let mut tag = [0u8; 16];
    let mut out: Vec<u8> = repeat(0).take(buf.len()).collect();
    cipher.encrypt(&buf, &mut out, &mut tag);
    println!("encrypt: inp={:?} out={:?} tag={:?}", buf, out, tag);

    let mut bs = Vec::with_capacity(12 + out.len() + 16);
    for b in nonce.iter() { bs.push(*b); }
    bs.append(&mut out);
    for b in tag.iter() { bs.push(*b); }
    Ok(bs)
}

pub fn decrypt(key: &[u8], buf: &[u8]) -> Result<Vec<u8>, Box<Error>> {
    if buf.len() < 12 {
        println!("ERROR: Buffer Length too short, are you trying to decrypt non-encrypted data?");
        return Err("Failed decrypting data".into());
    }
    let iv: &[u8] = &buf[0..12];
    let mut decipher = AesGcm::new(KeySize::KeySize256, &key, &iv, &[]);
    let inp = &buf[12..buf.len() - 16];
    let tag = &buf[buf.len() - 16..];
    let mut out: Vec<u8> = repeat(0).take(buf.len() - 28).collect();
    if decipher.decrypt(&inp, &mut out, &tag){
        println!("decrypt: inp={:?} out={:?}", inp, out);
        Ok(out)
    } else{

        Err("Failed to Decrypt".into())
    }
}
