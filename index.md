---
layout: default
title: AgilData Zero
---

# AgilData Zero - Zero-Knowledge Encryption for MySQL!

AgilData Zero is an encryption gateway for MySQL that encrypts sensitive data on the way into the database, and decrypts result sets on the way back out.

Even if the database server is compromised and an unauthorized user is able to login and run SQL queries, no sensitive data is revealed. This contrasts greatly with the traditional approach that databases take, where data is only encrypted in-transit and at-rest, but is available in plain text if someone can login and run queries.

# Project Status

This is a work-in-progress and not suitable for production use yet. The main limitations currently are:

- Subset of MySQL syntax supported (just enough to run TPC-C benchmarks)
- Depends on rust-crypto which has not been verified as secure yet
- Query planner only handles subset of validation required to ensure no unencrypted data can leak to the database server

# Security versus Functionality

One of the challenges with storing encrypted data in a database is that it reduces the databases ability to operate on that data. For example, if data is encrypted with AES-256 then it changes the sort order of that data, so it is no longer possible for the database to perform range queries or sort that data with an `ORDER BY` clause. Weaker forms of encryption exist that can preserve the sort order, but order-preserving encryption is known to leak knowledge about the data, as demonstrated in [cite that paper here]. 

AgilData Zero takes a pragmatic approach to the problem by supporting encyrption schemes that allow for some basic operations to be performed by the database. AgilData Zero also validates queries and fails any queries that attempt to perform an operation on encrypted data, rather than just returning the wrong results.

# Supported Encryption Algorithms

AgilData Zero currently supports the following types of encryption:

## AES-256 with column-specific IV 

A single initialization-vector (IV) and key is used to encrypt all values in the column. This type of encryption supports equality checks, meaning that simple predicates of the form `WHERE ssn = ?` can still be performed efficiently by the database. AgilData encrypts the query parameters so that the database never sees unencrypted data.

If two columns share the same key and IV then `JOIN` operations can also be performed natively by the database.

## AES-256 with unique IV per encrypted value

This form of encryption uses a unique IV per row. This is a stronger form of encryption but adds additional restrictions on functionality. Equality checks can no longer be performed.

# Architecture

AgilData is implemented in the Rust programming language since this ensures that the product is not susceptible to exploits that rely on buffer overflow / overrun errors. Also, Rust can be run on bare metal for additional security.

# Performance

Performance will naturally depend greatly on the specific application and the encryption schemes chosen, but the general overhead of parsing and planning queries (excluding encryption) adds approximately 10% overhead compared to running an applicaton directly against MySQL.

# Roadmap

We use github issues to track the roadmap for this product. Some of major themes are:

- Add query engine in the gateway to allow for increased functionality against strongly encrypted data
- Add support for caching unencrypted index data in the gateway to support efficient range queries and sort operations
- Improving coverage of MySQL/MariaDB SQL syntax
- Develop tools to make recommendations for encryption schemes based on current query access patterns

# Running AgilData Zero

We recommend using rustc 1.13.0-nightly.

Simply modify zero-config.xml to specify the MySQL connection details and specify which columns should be encrypted. 

```
cargo run --release
```










