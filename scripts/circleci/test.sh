#!/bin/sh
#
# Script for testing AgilData Zero with Jenkins
#
# For the record, this script is very chatty.  This is so we can verify
# each step of the way as the build runs.

RUST_BUILD="nightly-2016-08-03"
AGILDATA_TEST_DB="zero"
MYSQL_USER="agiluser"
MYSQL_PASS="password123"

# # Clear out binaries already built
# echo "Clearing out already built binaries."
# rm -rf target
# 
# # Build Zero
# echo "Building AgilData Zero"
# cargo build

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

# Clear out previous run results
rm -f scripts/test/test*-output.sql

# Copy test database info into MySQL
echo "Running test script: test1.sql"
mysql --host=127.0.0.1 --port=3307 -u$MYSQL_USER -p$MYSQL_PASS -D $AGILDATA_TEST_DB < scripts/test/test1.sql > scripts/test/test1-output.sql

echo
echo "Comparing output from test1.sql against expected output."
diff scripts/test/expected1.sql scripts/test/test1-output.sql
echo

# echo "Comparing output from test1.sql against expected1.sql (Any output indicates an error.)"
# diff scripts/test/expected1.sql scripts/test/output1.sql

# Drop Database
echo "Dropping database: $AGILDATA_TEST_DB"
mysql --host=127.0.0.1 --port=3306 -u$MYSQL_USER -p$MYSQL_PASS -e "DROP DATABASE $AGILDATA_TEST_DB"

# Stop AgilData Zero
echo "Stopping AgilData Zero: $AGILDATA_ZERO_PID"
kill $AGILDATA_ZERO_PID
