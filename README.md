# AgilData Zero - Zero-Knowledge Encryption for MySQL

[![Build Status](https://travis-ci.org/AgilData/agildata-zero.svg?branch=master)](https://travis-ci.org/AgilData/agildata-zero)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

AgilData Zero is an encryption gateway for MySQL that encrypts sensitive data on the way into the database, and decrypts result sets on the way back out.

Even if the database server is compromised and an unauthorized user is able to login and run SQL queries, no sensitive data is revealed. This contrasts greatly with the traditional approach that databases take, where data is only encrypted in-transit and at-rest, but is available in plain text if someone can login and run queries.

# Project Status

AgilData Zero is currently a proof-of-concept project. The main limitations currently are:

- Subset of MySQL syntax supported (just enough to run [TPC-C](https://github.com/AgilData/tpcc) benchmarks)
- Depends on [rust-crypto](https://github.com/DaGenix/rust-crypto) which is not recommended for production use
- Query planner only handles subset of validation required to ensure no unencrypted data can leak to the database server

# Documentation

Full documentation is available at https://agildata.github.io/agildata-zero/








