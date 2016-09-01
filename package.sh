#!/bin/bash
export app=agildata-zero
export exe=target/release/$app

cargo clean
cargo build --release
rc=$?; if [[ $rc == 0 ]]; then
    echo $app build completed
    rm -Rf dist
    mkdir dist
    cp ./doc/README.md dist/
    cp example-zero-config.xml dist/
    cp $exe dist/
    tar -vczf $app.tar.gz -C dist .
    echo $app.tar.gz packaging completed
fi
