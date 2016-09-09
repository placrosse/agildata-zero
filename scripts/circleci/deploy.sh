#!/bin/bash
#
# Script to handle deployment preparation
# This script only runs in the CircleCI environment.

cd $HOME/agildata-zero
rm -rf dist
mkdir dist
mkdir dist/bin
mkdir dist/doc
mkdir dist/conf
cp target/debug/agildata-zero dist/bin
cp zero-config.xml dist/conf
cp -r target/doc dist/doc
cp doc/README.md dist
tar -czf agildata-zero-dist.tar.gz dist

echo "Deployment archive completed."
ls -al agildata-zero-dist.tar.gz
