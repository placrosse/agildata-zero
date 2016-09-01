#!/bin/sh
#
# Script for testing AgilData Zero with Jenkins

sudo service mysql start

# Drop test databases
# Copy test database info into MySQL
# Query MySQL and run diffs against the values
# If any diffs occur, indicate a build error, and report back the differences

sudo service mysql stop
