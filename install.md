---
layout: default
title: AgilData Zero - Installation Instructions
active: install
---

# Installation

WORK IN PROGRESS

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
$ mysql -u myuser -pmypassword -h 127.0.0.1 -P 3306
```

Now that the mysql container is up and running we will pull the latest agildata:zero

``` bash
docker pull agildata/zero:latest
docker run --name agildata --link mysql-server:mysql -p 3307:3307 -d agildata/zero:latest
```

You can now test the agildata-zero container by connecting directly with a mysql client

```bash
$ mysql -u myuser -pmypassword -h 127.0.0.1 -P 3307
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
