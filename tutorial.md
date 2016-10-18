---
layout: default
title: AgilData Zero - Installation Instructions
active: tutorial
---

# AgilData Zero Tutorial

In this tutorial we will walk through creating a schema with some encrypted columns and demonstrate how AgilData Zero works.

To follow along with this tutorial you will need access to a MySQL instance and have permissions to create databases and tables. This MySQL instance can be local or remote.

## 1. Installing AgilData Zero

Use the [installation instructions](install.html) to install a binary release or compile the code from source.

## 2. Configure AgilData Zero

Edit the provided `zero-config.xml` and modify the MySQL connection details:

``` xml
<connection>
	<property name="host" value="127.0.0.1"/>
	<property name="schema" value="myschema"/>
	<property name="user" value="myuser"/>
	<property name="password" value="mypassword"/>
</connection>
```

By default, Zero binds to port 3307 on 127.0.0.1 but this can also be modified:

```xml
<client>
	<property name="host" value="127.0.0.1" />
	<property name="port" value="3307" />
</client>
```

## 3. Test connectivity

It should now be possible to run the gateway and connect to it. To run the gateway, simple run the executable:

```
./agildata-zero
```

Next, use the MySQL console to connect to the proxy.

```
mysql -h 127.0.0.1 -P 3307 -u username -p
```

If this fails, try connecting to MySQL directly to confirm that the gateway is configured with the correct connection details.

## 4. Create a schema with encrypted columns

When creating new tables, AgilData Zero will intercept the `CREATE TABLE` statements and modify them according to the encryption scheme specified in `zero-config.xml`.

The configuration file that ships with AgilData Zero contains an encryption configuration for a `zero` schema and specifies the encryption to use for the tables `user`, `user_purchase` and `item`.

NOTE: AgilData Zero does not yet support `CREATE DATABASE` so you'll need to run `CREATE DATABASE zero` directly against MySQL until [Issue #77](https://github.com/AgilData/agildata-zero/issues/77) is resolved.

For each column where encryption is required it is necessary to include the column in this configuration section and specify the encryption scheme to be used (currently only `AES` is supported) along with the initialization vector (iv) and key. It is not necessary to provide configuration information for columns in the table that will not be encrypted.

Here is the encryption schema for our tutorial database, ommitting the keys and initialization vectors.

```xml
<schema name="zero">
	<table name="user">
		<column name="id" type="INTEGER" encryption="none" pkOrdinal="0"/>
		<column name="first_name" type="VARCHAR(50)" encryption="AES" iv="..." key="..."/>
		<column name="last_name" type="VARCHAR(50)" encryption="AES" iv="..." key="..."/>
		<column name="ssn" type="VARCHAR(10)" encryption="AES" iv="..." key="..."/>
		<column name="age" type="INTEGER" encryption="AES" iv="..." key="..."/>
		<column name="sex" type="VARCHAR(1)" encryption="AES" iv="..." key="..."/>
	</table>
	<table name="user_purchase">
		<column name="id" type="INTEGER" encryption="none" pkOrdinal="0"/>
		<column name="user_id" type="INTEGER" encryption="none"/>
		<column name="item_code" type="INTEGER" encryption="AES" iv="..." key="..."/>
		<column name="amount" type="DOUBLE" encryption="AES" iv="..." key="..."/>
	</table>
	<table name="item">
		<column name="item_code" type="INTEGER" encryption="AES" iv="..." key="..."/>
		<column name="item_name" type="VARCHAR(50)" encryption="AES" iv="..." key="..."/>
        <column name="description" type="VARCHAR(50)" encryption="none"/>
	</table>
</schema>
```

With the encryption schema defined we can go ahead and create tables) making sure we are connected via the AgilData Zero gateway rather than connecting directly to MySQL).

```sql
CREATE TABLE user (
id INTEGER NOT NULL,
first_name VARCHAR(50),
last_name VARCHAR(50),
ssn VARCHAR(10),
age INTEGER,
sex VARCHAR(1),
PRIMARY KEY (id)
);

CREATE TABLE user_purchase (
id INTEGER NOT NULL,
user_id INTEGER NOT NULL,
item_code INTEGER NOT NULL,
amount INTEGER NOT NULL,
PRIMARY KEY (id)
);

CREATE TABLE item (
item_code INTEGER NOT NULL,
item_name VARCHAR(50),
description VARCHAR(50),
PRIMARY KEY (item_code)
);
```

With the tables created, let's go ahead and insert some data and select it back out:

```sql
INSERT INTO user (id, first_name, last_name, ssn, age, sex) VALUES (1, 'Janice', 'Joplin', '1234567890', 27, 'F');
SELECT * FROM user;
```

This should produce the following output.

```sql
mysql> INSERT INTO user (id, first_name, last_name, ssn, age, sex) VALUES (1, 'Janice', 'Joplin', '1234567890', 27, 'F');
Query OK, 1 row affected (0.00 sec)

mysql> SELECT * FROM user;
+----+------------+-----------+------------+------+------+
| id | first_name | last_name | ssn        | age  | sex  |
+----+------------+-----------+------------+------+------+
|  1 | Janice     | Joplin    | 1234567890 |   27 | F    |
+----+------------+-----------+------------+------+------+
1 row in set (0.00 sec)
```

If everything has been configured correctly, the data stored in MySQL is actually encrypted. To see if this is the case we can connect directly to MySQL and query the data there.

```
mysql -h 127.0.0.1 -P 3307 -u username -p
```

```sql
mysql> SELECT * FROM user;
+----+------------------------------------+------------------------------------+----------------------------------------+--------------------------------------+-------------------------------+
| id | first_name                         | last_name                          | ssn                                    | age                                  | sex                           |
+----+------------------------------------+------------------------------------+----------------------------------------+--------------------------------------+-------------------------------+
|  1 | ????????Cơ??                       | ?.ty??GR???џǥ???K{ɧ^??k?6????      | ?.ty??GR??!?=?-]??)?&                  | ??X                                  | ?.ty??GR??g?@T?v???ƭ?\r?o     |
+----+------------------------------------+------------------------------------+----------------------------------------+--------------------------------------+-------------------------------+
1 row in set (0.00 sec)
```

This demonstrates how AgilData Zero is encrypting data being inserted into the table and automatically decrypting the data being returned in result sets.

## 5. Functionality for encrypted columns

### 5.1 Equality

When AES is used with a shared IV for all values in a column then it is possible to use equality predicates in queries. For example, the following query is supported:

``` sql
SELECT * FROM user WHERE first_name = 'Janice' and last_name = 'Joplin'
```

The gateway will rewrite the query, replacing the literal values with encrypted values, ensuring that the database never sees the clear text values.


### 5.2 Joins

If two columns share the same encryption key and IV then they can can be used in an equi-join.

### 5.3 Unsupported Operations

Any attempt at using an unsupported operation on an encrypted column should result in the gateway rejecting the query and returning an error to the client.
