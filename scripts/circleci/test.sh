#!/bin/bash
#
# Script for testing AgilData Zero with CircleCI
#
# For the record, this script is very chatty.  This is so we can verify
# each step of the way as the build runs.

set -e

source /home/ubuntu/.cargo/env

RUST_BUILD="nightly-2016-09-12"
AGILDATA_TEST_DB="zero"
MYSQL_USER="agiluser"
MYSQL_PASS="password123"

TESTS=(test1 test_data_types test_uncontrolled)

# Set up the Rust version to be overridden to the required build.
rustup override set $RUST_BUILD

# Run a full test
cargo test

# Start server in the background, storing the PID of the app
echo "Generated binaries:"
ls -al target/debug
echo

# Launch AgilData Zero
echo
echo "Launching AgilData-Zero proxy..."
target/debug/agildata-zero --config src/test/test-zero-config.xml &
echo
echo "Waiting for AgilData-Zero proxy to initialize."
sleep 5
AGILDATA_ZERO_PID=$!
echo
echo "AgilData Zero launched: Process ID=$AGILDATA_ZERO_PID"

echo "Create database (if not exist): $AGILDATA_TEST_DB via MySQL on 127.0.0.1 port 3306"
mysql --host=127.0.0.1 --port=3306 -u$MYSQL_USER -p$MYSQL_PASS -e "CREATE DATABASE IF NOT EXISTS $AGILDATA_TEST_DB CHARACTER SET UTF8"

# PS to make sure the process is running
ps -aux | grep $AGILDATA_ZERO_PID | grep -v grep
sleep 5

# Clear out previous run results
rm -f scripts/test/test*-output.sql

# Copy test database info into MySQL
echo
for test_script in "${TESTS[@]}"
do
  if [ -f "scripts/test/${test_script}.sql" ]; then
    echo "Running test script: ${test_script}.sql against MySQL on 127.0.0.1 port 3307"
    mysql --host=127.0.0.1 --port=3307 -u$MYSQL_USER -p$MYSQL_PASS -D $AGILDATA_TEST_DB < scripts/test/${test_script}.sql > scripts/test/${test_script}-output.sql
    echo "Comparing output from ${test_script}.sql against expected output."
    output=$(diff scripts/test/${test_script}-expected.sql scripts/test/${test_script}-output.sql)
    if [ "${output}" != "" ]; then
      echo "Output from ${test_script}.sql does not match expected output; integration test fails!"
      echo
      echo "--- DIFF OUTPUT: ---"
      echo "${output}"
      echo "--- END DIFF OUTPUT ---"
      exit 2
    fi
  else
    echo "Skipping eval of test script ${test_script}.sql: file not found (or test does not exist.)"
  fi
done

echo

# Drop Database
echo
echo "Dropping database: $AGILDATA_TEST_DB"
mysql --host=127.0.0.1 --port=3306 -u$MYSQL_USER -p$MYSQL_PASS -e "DROP DATABASE $AGILDATA_TEST_DB"

# Stop AgilData Zero
echo
echo "Stopping AgilData Zero: $AGILDATA_ZERO_PID"
kill $AGILDATA_ZERO_PID
