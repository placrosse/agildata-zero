#!/bin/bash
#
# Script for testing AgilData Zero with Jenkins
#
# For the record, this script is very chatty.  This is so we can verify
# each step of the way as the build runs.

RUST_BUILD="nightly-2016-08-03"
AGILDATA_TEST_DB="zero"
MYSQL_USER="agiluser"
MYSQL_PASS="password123"

TESTS=(test1 test2)

# Start server in the background, storing the PID of the app
echo "Generated binaries:"
ls -al target/debug
echo

# Launch AgilData Zero
target/debug/agildata-zero & 
AGILDATA_ZERO_PID=$!
echo "AgilData Zero launched: Process ID=$AGILDATA_ZERO_PID"

# PS to make sure the process is running
ps -aux | grep $AGILDATA_ZERO_PID | grep -v grep

# Clear out previous run results
rm -f scripts/test/test*-output.sql

# Copy test database info into MySQL
echo
for test_script in "${TESTS[@]}"
do
  if [ -f "scripts/test/${test_script}.sql" ]; then
    echo "Running test script: ${test_script}.sql"
    mysql --host=127.0.0.1 --port=3307 -u$MYSQL_USER -p$MYSQL_PASS -D $AGILDATA_TEST_DB < scripts/test/${test_script}.sql > scripts/test/${test_script}-output.sql
    echo "Comparing output from ${test_script}.sql against expected output."
    output=$(diff scripts/test/${test_script}-expected.sql scripts/test/${test_script}-output.sql)
    if [ "${output}" != "" ]; then
      echo "Output from ${test_script}.sql does not match expected output; integration test fails!"
      echo
      echo "--- DIFF OUTPUT: ---"
      echo "${output}"
      echo "--- END DIFF OUTPUT ---"
      echo
      exit 2
    fi
  else
    echo "Skipping eval of test script ${test_script}.sql: file not found (or test does not exist.)"
  fi
done

echo

# Drop Database
echo "Dropping database: $AGILDATA_TEST_DB"
mysql --host=127.0.0.1 --port=3306 -u$MYSQL_USER -p$MYSQL_PASS -e "DROP DATABASE $AGILDATA_TEST_DB"

# Stop AgilData Zero
echo "Stopping AgilData Zero: $AGILDATA_ZERO_PID"
kill $AGILDATA_ZERO_PID
