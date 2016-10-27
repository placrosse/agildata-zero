use rustc_serialize::Decodable;
use rustc_serialize::Decoder;
use toml;
use std::env;

use std::collections::HashMap;

#[derive(Debug, PartialEq, RustcDecodable)]
struct Config0 {
    client: ClientConfig0,
}

#[derive(Debug, PartialEq, RustcDecodable)]
struct ClientConfig0 {
    host: OptResolvedString0,
    port: OptResolvedString0
}

#[derive(RustcDecodable)]
struct OptResolvedString0 {
    value: Option<String>
}

