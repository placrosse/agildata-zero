---
layout: default
title: AgilData Zero - Installation Instructions
active: install
---

# Installation

## Linux

A tarball is available that is compatible with all Linux distributions.

Find the latest release from [https://github.com/AgilData/agildata-zero/releases](https://github.com/AgilData/agildata-zero/releases)

``` bash
wget https://github.com/AgilData/agildata-zero/releases/download/v0.1.0-test3/agildata-zero-v0.1.0-test3-x86_64-unknown-linux-musl.tar.gz
tar xzf agildata-zero-v0.1.0-test3-x86_64-unknown-linux-musl.tar.gz
```

## Docker on Mac

NOTE: these instructions are for the new Docker (1.12 or later) that does not use VirtualBox.

To run agildata-zero in docker alongside a mysql instance first pull and start a mysql box with the following commands:

``` bash
docker pull mysql:5.7
docker run --name mysql-server -e MYSQL_USER=myuser -e MYSQL_PASSWORD=mypassword -e MYSQL_DATABASE=zero -e MYSQL_ROOT_PASSWORD=password -p 3306:3306 -d mysql:5.7
```

You can now test the mysql container by connecting directly with a mysql client

```bash
mysql -u myuser -pmypassword -h 127.0.0.1 -P 3306
```

Now that the mysql container is up and running we will pull the latest agildata:zero

``` bash
docker pull agildata/zero:latest
docker run --name agildata --link mysql-server:mysql -p 3307:3307 -d agildata/zero:latest
```

You can now test the agildata-zero container by connecting directly with a mysql client

```bash
mysql -u myuser -pmypassword -h 127.0.0.1 -P 3307
```

## Docker on Linux

To run agildata-zero in docker alongside a mysql instance first pull and start a mysql box with the following commands:

``` bash
docker pull mysql:5.7
docker run --name mysql-server -e MYSQL_USER=myuser -e MYSQL_PASSWORD=mypassword -e MYSQL_DATABASE=zero -e MYSQL_ROOT_PASSWORD=password -d mysql:5.7
```

Now that the mysql container is up and running we will pull the latest agildata:zero

``` bash
docker pull agildata/zero:latest
docker run --name agildata --link mysql-server:mysql agildata/zero:latest
```

You can now test the agildata-zero container by connecting directly with a mysql client or with a container

```bash
{% raw %}
mysql -h$(docker inspect --format '{{ .NetworkSettings.IPAddress }}' agildata) -P 3307 -u myuser -pmypassword
{% endraw %}
# OR with another container
docker run -it --link agildata:mysql --rm mysql sh -c 'exec mysql -h"$MYSQL_PORT_3307_TCP_ADDR" -P"$MYSQL_PORT_3307_TCP_PORT" -umyuser -pmypassword'
```

# Docker with custom configuration
The agildata/zero docker image comes prepackaged with a lightweight default zero-config.xml. 

In many cases, extending or overriding components of this configuration file will be necessary when launching in Docker. 

The AgilData Zero executable supports extending or overriding properties with config fragments that exist in the /etc/zero.d directory. 

One example can be to override a certain element, such as connection:

```xml
<!-- overrides connection properties in the default config -->
<zero-config>
    <connection>
        <property name="host" value="176.120.90.168" />
        <property name="user" value="somecustomuser" />
        <property name="password" value="s0m3cust0mP@$$w0rd" />
    </connection>
</zero-config>
```

Another can be to extend other configs, such as adding new schema configuration:

```xml
<!-- overrides connection properties in the default config -->
<zero-config>
    <schema name="newschema">
        <table name="newtable">
            <column name="id" type="INTEGER" encryption="none"/>
            <column name="a" type="VARCHAR(50)" encryption="AES" iv="..." key="..."/>
            <column name="b" type="VARCHAR(50)" encryption="AES_GCM" key="..."/>
        </table>
    </schema>
</zero-config>
```

Any number of config fragments can be added to the `/etc/zero.d` directory, though do note that configs will be loaded by filename in library order and thus those loaded last can override components of those loaded before.

To use such config fragments with a docker run execution, use a run syntax similar to the below, mounting your local config fragment files into the `/etc/zero.d/` directory in the container:

```bash
{% raw %}
docker run --name agildata --link mysql-server:mysql -v /path/to/my/configs:/etc/zero.d agildata/zero:latest
{% endraw %}
```