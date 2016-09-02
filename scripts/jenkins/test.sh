#!/bin/sh
#
# Script for testing AgilData Zero with Jenkins
#
# For the record, this script is very chatty.  This is so we can verify
# each step of the way as the build runs.

RUST_BUILD="nightly-2016-08-03"
AGILDATA_TEST_DB="itest$BUILD_NUMBER"

MYSQL_USER="agiluser"
MYSQL_PASS="password123"

# Set the version of Rust to use
echo "Switching to Rust build: $RUST_BUILD"
rustup override set $RUST_BUILD

# Clear out binaries already built
echo "Clearing out already built binaries."
rm -rf target

# Build Zero
echo "Building AgilData Zero"
cargo build

# Start server in the background, storing the PID of the app
echo "Generated binaries:"
ls -al target/debug

# Launch AgilData Zero
echo "Launching AgilData Zero."
target/debug/agildata-zero 2>&1 >/dev/null & 
AGILDATA_ZERO_PID=$!
echo "AgilData Zero launched: Process ID=$AGILDATA_ZERO_PID"

# PS to make sure the process is running
ps -aux | grep $AGILDATA_ZERO_PID | grep -v grep

# Create Database
echo "Creating database: $AGILDATA_TEST_DB"
mysql --host=127.0.0.1 --port=3306 -u$MYSQL_USER -p$MYSQL_PASS -e "CREATE DATABASE $AGILDATA_TEST_DB CHARACTER SET UTF8"

# Copy test database info into MySQL
mysql --host=127.0.0.1 --port=3307 -u$MYSQL_USER -p$MYSQL_PASS -D $AGILDATA_TEST_DB < scripts/test/test1.sql

# Query MySQL and run diffs against the values
# If any diffs occur, indicate a build error, and report back the differences

# Drop Database
echo "Dropping database: $AGILDATA_TEST_DB"
mysql --host=127.0.0.1 --port=3306 -u$MYSQL_USER -p$MYSQL_PASS -e "DROP DATABASE $AGILDATA_TEST_DB"

# Stop AgilData Zero
echo "Stopping AgilData Zero: $AGILDATA_ZERO_PID"
kill $AGILDATA_ZERO_PID
