#!/bin/bash
#
# Build script for AgilData Zero in Circle CI

source /home/ubuntu/.cargo/env
rustup default nightly-2016-08-03
rustup override set nightly-2016-08-03

# Build the project
echo
echo "Building AgilData Zero."
cargo build

