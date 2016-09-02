#!/bin/sh
#
# Script for testing AgilData Zero with Jenkins

# Set the version of Rust to use
rustup override set nightly-2016-08-03

# Build Zero
cargo build

# Start server in the background, storing the PID of the app
echo "Generated binaries:"
ls -al target/debug

# Launch AgilData Zero
echo "Launching AgilData Zero."
target/debug/agildata-zero &
AGILDATA_ZERO_PID=$!

# PS to make sure the process is running
ps -ax | grep $AGILDATA_ZERO_PID

# Drop test databases

# Copy test database info into MySQL
# Query MySQL and run diffs against the values
# If any diffs occur, indicate a build error, and report back the differences

# Stop AgilData Zero
echo "Stopping AgilData Zero: $AGILDATA_ZERO_PID"
kill $AGILDATA_ZERO_PID
