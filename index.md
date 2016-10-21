---
layout: default
title: AgilData Zero
active: index
---

# AgilData Zero

## Zero-Knowledge Encryption for MySQL

AgilData Zero is an encryption gateway for MySQL that encrypts sensitive data on the way into the database, and decrypts result sets on the way back out.

Even if the database server is compromised and an unauthorized user is able to login and run SQL queries, no sensitive data is revealed. This contrasts greatly with the traditional approach that databases take, where data is only encrypted in-transit and at-rest, but is available in plain text if someone can login and run queries.

# Project Status

AgilData Zero is currently a proof-of-concept. The main limitations currently are:

- Subset of MySQL syntax supported (just enough to run TPC-C benchmarks)
- Depends on rust-crypto which has not been verified as secure yet
- Query planner only handles subset of validation required to ensure no unencrypted data can leak to the database server
- Encryption keys are stored in clear text in the encryption gateway configuration file


# Security versus Functionality

One of the challenges with storing encrypted data in a database is that it reduces the databases ability to operate on that data. For example, if data is encrypted with AES-256 then it changes the sort order of that data, so it is no longer possible for the database to perform range queries or sort that data with an `ORDER BY` clause. Weaker forms of encryption exist that can preserve the sort order, but order-preserving encryption is known to leak knowledge about the data.

AgilData Zero takes a pragmatic approach to the problem by supporting encyrption schemes that allow for some basic operations to be performed by the database. AgilData Zero also validates queries and fails any queries that attempt to perform an operation on encrypted data, rather than just returning the wrong results.

# Supported Encryption Algorithms

AgilData Zero currently supports the following types of encryption:

## Clear text

- No encryption and preserves all database functionality

## AES-256 with unique initialization vector per column

- This is a form of deterministic encryption where encrypting the same input value multiple times always results in the same encrypted data
- Supports equality operations, allowing the database to filter (WHERE ssn = ?) 
- If two columns share the same initialization vector and key then they can be joined
- Not suitable for low-cardinality data since this encryption is deterministic e.g. for a gender column storing M or F,  there would only be two encrypted values

## AES-256 with unique initialization vector per value

- Non-deterministic encryption. Encrypting the same value multiple times results in a different encrypted value each time.
- More secure than using a fixed IV but no support for equality
- Database can include column in projection but cannot operate on the data

# Architecture

AgilData is implemented in the Rust programming language since this ensures that the product is not susceptible to exploits that rely on buffer overflow / overrun errors. Also, Rust can be run on bare metal for additional security.

# Performance

Performance will naturally depend greatly on the specific application and the encryption schemes chosen, but the general overhead of parsing and planning queries (excluding encryption) adds approximately 10% overhead compared to running an applicaton directly against MySQL.

# Roadmap

We use [github issues](https://github.com/AgilData/agildata-zero/issues) to track the roadmap for this product. Some of major themes are:

- Add query engine in the gateway to allow for increased functionality against strongly encrypted data
- Add support for caching unencrypted index data in the gateway to support efficient range queries and sort operations
- Improving coverage of MySQLSQL syntax
- Develop tools to make recommendations for encryption schemes based on current query access patterns
- Replace flat file XML configuration with key management systems, encrypted in-database configuration metadata, and UI configuration tools, to name a few
