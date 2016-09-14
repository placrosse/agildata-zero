#!/bin/bash
#
# Build script for AgilData Zero in Circle CI

source /home/ubuntu/.cargo/env
rustup default nightly-2016-09-12
rustup override set nightly-2016-09-12

# Build the project
echo
echo "Building AgilData Zero."
cargo build

