#!/bin/sh
#
# Script for testing AgilData Zero with Jenkins

# Build Zero
# Start server in the background, storing the PID of the app
# Drop test databases
# Copy test database info into MySQL
# Query MySQL and run diffs against the values
# If any diffs occur, indicate a build error, and report back the differences
