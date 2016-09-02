#!/bin/bash
export app=agildata-zero
export exe=target/release/$app

rustup override set nightly-2016-08-31

cargo test --color always
rc=$?; if [[ $rc != 0 ]]; then
    exit 1
fi
echo $app tests completed
cargo clean
echo $app clean build starting
cargo build --release --color always
rc=$?; if [[ $rc != 0 ]]; then
    exit 1
fi
echo $app release build completed
rm -Rf dist
mkdir dist
cp ./doc/README.md dist/
cp zero-config.xml dist/
cp $exe dist/
tar -vczf $app.tar.gz -C dist .
echo $app.tar.gz packaging completed
