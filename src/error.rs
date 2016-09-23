use std::error;
use std::fmt;
use std::io;
use std::result;
use std::sync;
use std::error::Error;

#[derive(Debug, Clone)]
pub enum ZeroError {

//    IoError(io::Error),
    EncryptionError {message: String, code: String},
    DecryptionError {message: String, code: String},
    ParseError{message: String, code: String},
    SchemaError{message: String, code: String},
}

impl ZeroError{
    pub fn is_read_error(&self) -> bool {
        match self {
            &ZeroError::DecryptionError{..} => true,
            &ZeroError::SchemaError{..} => true,
            _=> false,
        }
    }
}

impl Error for ZeroError {
    fn description(&self) -> &str {
        match *self {
//            ZeroError::IoError(_) => "I/O Error",
            ZeroError::EncryptionError{..} => "Encryption error",
            ZeroError::DecryptionError{..} => "Decryption error",
            ZeroError::ParseError{..} => "Parse error",
            ZeroError::SchemaError{..} => "Parse error"
        }
    }


    fn cause(&self) -> Option<&Error> {
        match *self {
//            ZeroError::IoError(ref err) => Some(err),
            _ => None
        }
    }


}

impl fmt::Display for ZeroError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ZeroError::EncryptionError{ref message, ref code} =>  {write!(f, "{}", message)},
            ZeroError::DecryptionError{ref message, ref code} =>  {write!(f, "{}", message)},
            ZeroError::ParseError{ref message, ref code}  =>  {write!(f, "{}", message)},
            ZeroError::SchemaError{ref message, ref code}  =>  {write!(f, "{}", message)},
//            ZeroError::IoError(ref err) => write!(f, "IOError {{ {} }}", err),
        }
    }
}

