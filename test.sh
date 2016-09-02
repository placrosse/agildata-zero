#!/bin/bash
rustup override set nightly-2016-08-31

cargo clean
cargo test
