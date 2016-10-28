#!/bin/bash
#
# Script to handle deployment preparation
# This script only runs in the CircleCI environment.

cd $HOME/agildata-zero
rm -rf agildata-zero
mkdir agildata-zero
mkdir agildata-zero/bin
mkdir agildata-zero/doc
mkdir agildata-zero/conf
cp target/debug/agildata-zero agildata-zero/bin
cp zero-config.toml agildata-zero/conf
cp -r target/doc agildata-zero
cp doc/README.md agildata-zero
tar -cvjf agildata-zero-dist.tar.bz2 agildata-zero

echo "Deployment archive completed."
ls -al agildata-zero-dist.tar.bz2
