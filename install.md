---
layout: default
title: AgilData Zero - Installation Instructions
---

# Installation

WORK IN PROGRESS

## Linux

A tarball is available that is compatible with all Linux distributions.

Find the latest release from https://github.com/AgilData/agildata-zero/releases

```
wget https://github.com/AgilData/agildata-zero/releases/download/v0.1.0-test3/agildata-zero-v0.1.0-test3-x86_64-unknown-linux-musl.tar.gz
tar xzf agildata-zero-v0.1.0-test3-x86_64-unknown-linux-musl.tar.gz
```

## Docker

To run agildata-zero in docker alongside a mysql instance first pull and start a mysql box with the following commands:

```
docker pull mysql
docker run --name mysql-server -e MYSQL_USER=agiluser -e MYSQL_PASSWORD=password123 -e MYSQL_DATABASE=zero -e MYSQL_ROOT_PASSWORD=password123 -d mysql:latest
```

Now that the mysql container is up and running we will pull the latest agildata:zero

```
docker pull agildata/zero:latest
docker run --name agildata --link mysql-server:mysql agildata/zero:latest
```

You can now test the agildata-zero container by connecting directly with a mysql client or with a container

```
mysql -h$(docker inspect --format '{{ .NetworkSettings.IPAddress }}' agildata1) -P 3307 -u agiluser -ppassword123
# OR with another container
docker run -it --link agildata1:mysql --rm mysql sh -c 'exec mysql -h"$MYSQL_PORT_3307_TCP_ADDR" -P"$MYSQL_PORT_3307_TCP_PORT" -uagiluser -ppassword123'
```

