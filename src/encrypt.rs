extern crate crypto;
use self::crypto::aes::KeySize;
use self::crypto::aes_gcm::AesGcm;
use self::crypto::aead::{AeadEncryptor, AeadDecryptor};
use std::iter::repeat;
use error::ZeroError;
use byteorder::{WriteBytesExt,ReadBytesExt,BigEndian};
use std::io::Cursor;

use chrono::{DateTime, TimeZone, NaiveDateTime};
use chrono::offset::utc::UTC;

use decimal::d128;
use std::str::from_utf8_unchecked;

#[derive(Debug, PartialEq, Clone)]
pub enum EncryptionType {
	AES,
	OPE,
	NA,
}

#[derive(Debug, PartialEq, Clone)]
pub enum NativeType {
	U64,
	I64,
	Char(u32), // fixed
	Varchar(u32), // variable
	F64,
	D128,
	BOOL,
	DATETIME(u32), // fsp
	DATE,

	// These are representative native types, but not supported as encrypted columns
	TIMESTAMP(u32),
	TIME(u32),
	YEAR(u32),
	FIXEDBINARY(u32),
	VARBINARY(u32), // Max of 65535
	LONGBLOB(u64),
	LONGTEXT(u64)
}

pub trait Encrypt {

	fn encrypt(self, scheme: &EncryptionType, key: &[u8; 32]) -> Result<Vec<u8>, Box<ZeroError>>;

}

pub trait Decrypt {
    type DecType;

	fn decrypt(value: &[u8], scheme: &EncryptionType, key: &[u8; 32]) -> Result<Self::DecType, Box<ZeroError>>;
}

impl Decrypt for bool {
	type DecType = bool;

	fn decrypt(value: &[u8], scheme: &EncryptionType, key: &[u8; 32]) -> Result<bool, Box<ZeroError>> {
		match scheme {
			&EncryptionType::AES => {
				let decrypted = decrypt(key, value)?;
				match Cursor::new(decrypted).read_u8().unwrap() {
					0 => Ok(false),
					1 => Ok(true),
					e => Err(ZeroError::DecryptionError{message: format!("Cannot decrypt u8 {} to boolean", e), code: "123".into()}.into())
				}
			},
			_ => Err(ZeroError::DecryptionError{message: format!("Decryption not supported {:?}", scheme), code: "123".into()}.into())
		}
	}
}

impl Encrypt for bool {

	fn encrypt(self, scheme: &EncryptionType, key: &[u8; 32]) -> Result<Vec<u8>, Box<ZeroError>> {

		match scheme {
			&EncryptionType::AES => {
				let mut buf: Vec<u8> = Vec::new();
				buf.write_u8(self as u8).unwrap();

				encrypt(key, &buf)
			},
			_ => Err(ZeroError::EncryptionError{message: format!("Encryption not supported {:?}", scheme), code: "123".into()}.into())
		}

	}
}

impl Decrypt for u64 {
    type DecType = u64;

	fn decrypt(value: &[u8], scheme: &EncryptionType, key: &[u8; 32]) -> Result<u64, Box<ZeroError>> {
		match scheme {
			&EncryptionType::AES => {
				let decrypted = decrypt(key, value)?;
                Ok(Cursor::new(decrypted).read_u64::<BigEndian>().unwrap())
			},
			_ => Err(ZeroError::DecryptionError{message: format!("Decryption not supported {:?}", scheme), code: "123".into()}.into())
		}
	}
}

impl Encrypt for u64 {

	fn encrypt(self, scheme: &EncryptionType, key: &[u8; 32]) -> Result<Vec<u8>, Box<ZeroError>> {

		match scheme {
			&EncryptionType::AES => {
				let mut buf: Vec<u8> = Vec::new();
				buf.write_u64::<BigEndian>(self).unwrap();

				encrypt(key, &buf)
			},
			_ => Err(ZeroError::EncryptionError{message: format!("Encryption not supported {:?}", scheme), code: "123".into()}.into())
		}

	}
}

impl Decrypt for i64 {
	type DecType = i64;

	fn decrypt(value: &[u8], scheme: &EncryptionType, key: &[u8; 32]) -> Result<i64, Box<ZeroError>> {
		match scheme {
			&EncryptionType::AES => {
				let decrypted = decrypt(key, value)?;
				Ok(Cursor::new(decrypted).read_i64::<BigEndian>().unwrap())
			},
			_ => Err(ZeroError::DecryptionError{message: format!("Decryption not supported {:?}", scheme), code: "123".into()}.into())
		}
	}
}

impl Encrypt for i64 {

	fn encrypt(self, scheme: &EncryptionType, key: &[u8; 32]) -> Result<Vec<u8>, Box<ZeroError>> {

		match scheme {
			&EncryptionType::AES => {

				let mut buf: Vec<u8> = Vec::new();
				buf.write_i64::<BigEndian>(self).unwrap();

				encrypt(key, &buf)
			},
			_ => Err(ZeroError::EncryptionError{message: format!("Encryption not supported {:?}", scheme), code: "123".into()}.into())

		}

	}
}

impl Decrypt for f64 {
	type DecType = f64;

	fn decrypt(value: &[u8], scheme: &EncryptionType, key: &[u8; 32]) -> Result<f64, Box<ZeroError>> {
		match scheme {
			&EncryptionType::AES => {
				let decrypted = decrypt(key, value)?;
				Ok(Cursor::new(decrypted).read_f64::<BigEndian>().unwrap())
			},
			_ => Err(ZeroError::DecryptionError{message: format!("Decryption not supported {:?}", scheme), code: "123".into()}.into())
		}
	}
}

impl Encrypt for f64 {

	fn encrypt(self, scheme: &EncryptionType, key: &[u8; 32]) -> Result<Vec<u8>, Box<ZeroError>> {

		match scheme {
			&EncryptionType::AES => {
				let mut buf: Vec<u8> = Vec::new();
				buf.write_f64::<BigEndian>(self).unwrap();

				encrypt(key, &buf)
			},
			_ => Err(ZeroError::EncryptionError{message: format!("Encryption not supported {:?}", scheme), code: "123".into()}.into())

		}

	}
}

impl Decrypt for d128 {
	type DecType = d128;

	fn decrypt(value: &[u8], scheme: &EncryptionType, key: &[u8; 32]) -> Result<d128, Box<ZeroError>> {
		match scheme {
			&EncryptionType::AES => {
				let decrypted = decrypt(key, value)?;

				let hex_str = decrypted.iter().rev()
					.map(|b| format!("{:02x}", b))
					.collect::<Vec<String>>()
					.connect("");

				Ok(d128::from_hex(&hex_str))
			},
			_ => Err(ZeroError::DecryptionError{message: format!("Decryption not supported {:?}", scheme), code: "123".into()}.into())
		}
	}
}

impl Encrypt for d128 {


	fn encrypt(self, scheme: &EncryptionType, key: &[u8; 32]) -> Result<Vec<u8>, Box<ZeroError>> {
		match scheme {
			&EncryptionType::AES => {

				// decimal does not expose underlying bytes.
				// get hex string, convert to bytes and encrypt
				let hex = format!("{:x}", self);

				unsafe {
					let mut bytes: Vec<u8> = Vec::new();
					for octet in hex.as_bytes().chunks(2).rev() {
						bytes.push( match u8::from_str_radix(from_utf8_unchecked(octet), 16) {
							Ok(b) => b,
							Err(e) => return Err(
								ZeroError::EncryptionError{
									message: format!("Failed to encrypt {} due to : {}", self, e),
									code: "123".into()
								}.into()
							)
						})
					}

					encrypt(key, &bytes)
				}

			},
			_ => Err(ZeroError::EncryptionError{message: format!("Decryption not supported {:?}", scheme), code: "123".into()}.into())

		}

	}
}

impl Decrypt for String {
    type DecType = String;

	fn decrypt(value: &[u8], scheme: &EncryptionType, key: &[u8; 32]) -> Result<String, Box<ZeroError>>{
        match scheme {
			&EncryptionType::AES => {
				let decrypted = decrypt(key, value)?;
				Ok(String::from_utf8(decrypted).expect("Invalid UTF-8"))

			},
			_ => Err(ZeroError::DecryptionError{message: format!("Decryption not supported {:?}", scheme), code: "123".into()}.into())
		}
	}
}

impl Encrypt for String {
	fn encrypt(self, scheme: &EncryptionType, key: &[u8; 32]) -> Result<Vec<u8>, Box<ZeroError>> {
		match scheme {
			&EncryptionType::AES => {
				let buf = self.as_bytes();
				debug!("Buf length = {}", buf.len());
				let e = encrypt(key, &buf).unwrap();
				debug!("Encrypted length = {}", e.len());
				Ok(e)
			},
			_ => Err(ZeroError::EncryptionError{message: format!("Decryption not supported {:?}", scheme), code: "123".into()}.into())

		}
	}
}

impl Decrypt for DateTime<UTC> {
	type DecType = DateTime<UTC>;

	fn decrypt(value: &[u8], scheme: &EncryptionType, key: &[u8; 32]) -> Result<DateTime<UTC>, Box<ZeroError>>{
		match scheme {
			&EncryptionType::AES => {

				let decrypted = decrypt(key, value)?;
				let mut curs = Cursor::new(decrypted);

				let timestamp = curs.read_i64::<BigEndian>().unwrap();

				// fractional seconds stored alongside timestamp
				let fs = curs.read_u32::<BigEndian>().unwrap();

				Ok(UTC.timestamp(timestamp, fs))

			},
			_ => Err(ZeroError::DecryptionError{message: format!("Decryption not supported {:?}", scheme), code: "123".into()}.into())
		}
	}
}

impl<Tz: TimeZone> Encrypt for DateTime<Tz> {

	fn encrypt(self, scheme: &EncryptionType, key: &[u8; 32]) -> Result<Vec<u8>, Box<ZeroError>> {

		match scheme {
			&EncryptionType::AES => {

				let mut buf: Vec<u8> = Vec::new();
				// sgtore fractional seconds alongside timestamp
				buf.write_i64::<BigEndian>(self.timestamp()).unwrap();
				buf.write_u32::<BigEndian>(self.timestamp_subsec_nanos()).unwrap();

				encrypt(key, &buf)

			},
			_ => Err(ZeroError::EncryptionError{message: format!("Encryption not supported {:?}", scheme), code: "123".into()}.into())

		}

	}
}

pub fn hex_key(hex: &str) -> [u8; 32] {
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


pub fn encrypt(key: &[u8], buf: &[u8]) -> Result<Vec<u8>, Box<ZeroError>> {
    let nonce = [0_u8;12];
    let mut cipher = AesGcm::new(KeySize::KeySize256, key, &nonce, &[]);

    let mut tag = [0u8; 16];
    let mut out: Vec<u8> = repeat(0).take(buf.len()).collect();
    cipher.encrypt(&buf, &mut out, &mut tag);
    debug!("encrypt: inp={:?} out={:?} tag={:?}", buf, out, tag);

    let mut bs = Vec::with_capacity(12 + out.len() + 16);
    for b in nonce.iter() { bs.push(*b); }
    bs.append(&mut out);
    for b in tag.iter() { bs.push(*b); }
    Ok(bs)
}

pub fn decrypt(key: &[u8], buf: &[u8]) -> Result<Vec<u8>, Box<ZeroError>> {
    if buf.len() < 12 {
        error!("ERROR: Buffer Length too short, are you trying to decrypt non-encrypted data?");
        return Err(ZeroError::DecryptionError{message: "Failed decrypting data".into(), code: "123".into()}.into())
    }
    let iv: &[u8] = &buf[0..12];
    let mut decipher = AesGcm::new(KeySize::KeySize256, &key, &iv, &[]);
    let inp = &buf[12..buf.len() - 16];
    let tag = &buf[buf.len() - 16..];
    let mut out: Vec<u8> = repeat(0).take(buf.len() - 28).collect();
    if decipher.decrypt(&inp, &mut out, &tag){
        debug!("decrypt: inp={:?} out={:?}", inp, out);
        Ok(out)
    } else{
        Err(ZeroError::DecryptionError{ message: "Failed decrypting data".into(), code: "123".into()}.into())
    }
}

#[cfg(test)]
mod test {

	use super::*;
	use chrono::*;

	use decimal::*;
	use std::str::FromStr;

	#[test]
	fn test_encrypt_u64() {
		let value = 12345_u64;
		let key = hex_key("44E6884D78AA18FA690917F84145AA4415FC3CD560915C7AE346673B1FDA5985");
		let enc = EncryptionType::AES;
		let encrypted = value.encrypt(&enc, &key).unwrap();

		let decrypted = u64::decrypt(&encrypted, &enc, &key).unwrap();

		assert_eq!(decrypted, value);
	}

	#[test]
	fn test_encrypt_i64() {
		let value = -12345_i64;
		let key = hex_key("44E6884D78AA18FA690917F84145AA4415FC3CD560915C7AE346673B1FDA5985");
		let enc = EncryptionType::AES;
		let encrypted = value.encrypt(&enc, &key).unwrap();

		let decrypted = i64::decrypt(&encrypted, &enc, &key).unwrap();

		assert_eq!(decrypted, value);
	}

	#[test]
	fn test_encrypt_string() {
		let value = String::from("Ima a sensitive string...");
		let key = hex_key("44E6884D78AA18FA690917F84145AA4415FC3CD560915C7AE346673B1FDA5985");
		let enc = EncryptionType::AES;
		let encrypted = value.clone().encrypt(&enc, &key).unwrap();

		let decrypted = String::decrypt(&encrypted, &enc, &key).unwrap();

		assert_eq!(decrypted, value.clone());
	}

	#[test]
	fn test_encrypt_f64() {
		let value = 12345.6789_f64;
		let key = hex_key("44E6884D78AA18FA690917F84145AA4415FC3CD560915C7AE346673B1FDA5985");
		let enc = EncryptionType::AES;
		let encrypted = value.encrypt(&enc, &key).unwrap();

		let decrypted = f64::decrypt(&encrypted, &enc, &key).unwrap();

		assert_eq!(decrypted, value);
	}

	#[test]
	fn test_encrypt_datetime() {
		let value = String::from("2014-11-28 21:00:09");
		let key = hex_key("44E6884D78AA18FA690917F84145AA4415FC3CD560915C7AE346673B1FDA5985");
		let enc = EncryptionType::AES;
		let datetime = UTC.datetime_from_str(&value, "%Y-%m-%d %H:%M:%S").unwrap();

		let encrypted = datetime.encrypt(&enc, &key).unwrap();
		let decrypted = DateTime::decrypt(&encrypted, &enc, &key).unwrap();
		assert_eq!(decrypted, datetime);

		let rewritten = decrypted.format("%Y-%m-%d %H:%M:%S").to_string();

		assert_eq!(rewritten, value);
	}

	#[test]
	fn test_encrypt_datetime_fsp() {
		let value = String::from("2014-11-28 21:00:09.778");
		let key = hex_key("44E6884D78AA18FA690917F84145AA4415FC3CD560915C7AE346673B1FDA5985");
		let enc = EncryptionType::AES;
		let datetime = UTC.datetime_from_str(&value, "%Y-%m-%d %H:%M:%S%.f").unwrap();

		let encrypted = datetime.encrypt(&enc, &key).unwrap();
		let decrypted = DateTime::decrypt(&encrypted, &enc, &key).unwrap();
		assert_eq!(decrypted, datetime);

		let rewritten = decrypted.format("%Y-%m-%d %H:%M:%S%.3f").to_string();

		assert_eq!(rewritten, value);
	}

	#[test]
	fn test_encrypt_date() {
		let value = String::from("2014-11-28");
		let key = hex_key("44E6884D78AA18FA690917F84145AA4415FC3CD560915C7AE346673B1FDA5985");
		let enc = EncryptionType::AES;
		let datetime = UTC.datetime_from_str(&format!("{} 00:00:00",&value), "%Y-%m-%d %H:%M:%S").unwrap();

		let encrypted = datetime.encrypt(&enc, &key).unwrap();
		let decrypted = DateTime::decrypt(&encrypted, &enc, &key).unwrap();

		assert_eq!(decrypted, datetime);

		let rewritten = decrypted.date().format("%Y-%m-%d").to_string();

		assert_eq!(rewritten, value);
	}

	#[test]
	fn test_encrypt_bool() {
		let value = true;
		let key = hex_key("44E6884D78AA18FA690917F84145AA4415FC3CD560915C7AE346673B1FDA5985");
		let enc = EncryptionType::AES;

		let encrypted = value.encrypt(&enc, &key).unwrap();
		let decrypted = bool::decrypt(&encrypted, &enc, &key).unwrap();
		assert_eq!(decrypted, value);
	}

	#[test]
	fn test_encrypt_decimal() {
		let src_string = String::from("10.2345");
		let value = d128::from_str(&src_string).unwrap();
		let key = hex_key("44E6884D78AA18FA690917F84145AA4415FC3CD560915C7AE346673B1FDA5985");
		let enc = EncryptionType::AES;

		let encrypted = value.encrypt(&enc, &key).unwrap();
		let decrypted = d128::decrypt(&encrypted, &enc, &key).unwrap();

		assert_eq!(decrypted, value);
		assert_eq!(decrypted.to_string(), src_string);

	}

}
