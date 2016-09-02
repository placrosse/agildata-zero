#!/bin/bash
export app=agildata-zero
export exe=target/release/$app
export ver=$(grep version Cargo.toml | cut -d'"' -f2)
export out=$app-$ver.tgz

rustup override set nightly-2016-08-31

cargo clean
cargo test --color=always
rc=$?; if [[ $rc != 0 ]]; then
    exit 1
fi
echo $app tests completed
cargo clean
echo $app clean build starting
# cargo build --features "clippy" --release --color=always
cargo build --release --color=always
rc=$?; if [[ $rc != 0 ]]; then
    exit 1
fi
echo $app release build completed
rm $app-*.tgz
rm -Rf dist
mkdir dist
cp ./doc/README.md dist/
cp zero-config.xml dist/
cp $exe dist/
tar -vczf $out -C dist .
echo $out packaging completed
