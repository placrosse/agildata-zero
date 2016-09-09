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
cp zero-config.xml agildata-zero/conf
cp -r target/doc agildata-zero/doc
cp doc/README.md agildata-zero
tar -czf agildata-zero-dist.tar.gz agildata-zero

echo "Deployment archive completed."
ls -al agildata-zero-dist.tar.gz
