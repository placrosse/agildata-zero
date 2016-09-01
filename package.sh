#!/bin/bash
export app=agildata-zero
export exe=target/release/$app

cargo test --color always
rc=$?; if [[ $rc != 0 ]]; then
    exit
fi
echo $app tests completed
cargo clean
echo $app clean build starting
cargo build --release --color always
rc=$?; if [[ $rc != 0 ]]; then
    exit
fi
echo $app release build completed
rm -Rf dist
mkdir dist
cp ./doc/README.md dist/
cp example-zero-config.xml dist/
cp $exe dist/
tar -vczf $app.tar.gz -C dist .
echo $app.tar.gz packaging completed
