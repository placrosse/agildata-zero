---
layout: default
title: AgilData Zero - Installation Instructions
---

# AgilData Zero Tutorial

## THIS IS A WORK IN PROGRESS ##

In this tutorial we will walk through creating a schema with some encrypted columns and demonstrate how AgilData Zero works.

To follow along with this tutorial you will need access to a MySQL instance and have permissions to create databases and tables. This MySQL instance can be local or remote.

## 1. Installing AgilData Zero

Use the [installation instructions](install.html) to install a binary release or compile the code from source.

## 2. Configure AgilData Zero

Edit the provided `zero-config.xml` and modify the MySQL connection details:

```xml
<connection>
	<property name="dbms" value="mysql"/>
	<property name="host" value="127.0.0.1"/>
	<property name="schema" value="tpcc"/>
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
mysql -h 127.0.0.1 -P 3307
```

If this fails, try connecting to MySQL directly to confirm that the gateway is configured with the correct connection details.

## 4. Create a schema with encrypted columns

When creating new tables, AgilData Zero will intercept the `CREATE TABLE` statements and modify them according to the encryption scheme specified in `zero-config.xml`.
