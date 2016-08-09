#[derive(Debug)]
pub enum EncryptionType {
	AES,
	AES_SALT,
	OPE,
	NA,
}

pub trait Encrypt {
	fn encrypt(self, scheme: &EncryptionType) -> Vec<u8>;
}

impl Encrypt for u64 {
	fn encrypt(self, scheme: &EncryptionType) -> Vec<u8> {
		panic!("Not implemented")
	}
}
