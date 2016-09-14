#!/bin/bash
#
# Documentation step for AgilData Zero

source /home/ubuntu/.cargo/env
rustup override set nightly-2016-09-12

# Generate documentation
echo
echo "Generating AgilData Zero auto-doc"
cargo doc

