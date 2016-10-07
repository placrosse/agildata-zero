// Copyright 2016 AgilData
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http:// www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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

