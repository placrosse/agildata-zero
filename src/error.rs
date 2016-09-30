use std::fmt;
use std::error::Error;

#[derive(Debug, Clone)]
pub enum ZeroError {
    EncryptionError {message: String, code: String},
    DecryptionError {message: String, code: String},
    ParseError{message: String, code: String},
    SchemaError{message: String, code: String},
}

impl Error for ZeroError {
    fn description(&self) -> &str {
        match *self {
            ZeroError::EncryptionError{..} => "Encryption error",
            ZeroError::DecryptionError{..} => "Decryption error",
            ZeroError::ParseError{..} => "Parse error",
            ZeroError::SchemaError{..} => "Parse error"
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            _ => None
        }
    }

}

impl fmt::Display for ZeroError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ZeroError::EncryptionError{ref message, ref code} => {write!(f, "[{}] {}", code, message)},
            ZeroError::DecryptionError{ref message, ref code} => {write!(f, "[{}] {}", code, message)},
            ZeroError::ParseError{ref message, ref code}      => {write!(f, "[{}] {}", code, message)},
            ZeroError::SchemaError{ref message, ref code}     => {write!(f, "[{}] {}", code, message)},
        }
    }
}

