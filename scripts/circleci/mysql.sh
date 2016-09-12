#!/bin/bash
#
# This script starts up MySQL in the CircleCI environment using Docker.

MYSQL_DATABASE="zero"
MYSQL_USER="agiluser"
MYSQL_PASS="password123"
MYSQL_PORT="3306"
MYSQL_VERSION="5.7.12"

echo
echo "Starting Docker for MySQL version $MYSQL_VERSION"
docker run --detach --name mysql --publish $MYSQL_PORT:$MYSQL_PORT --env MYSQL_ALLOW_EMPTY_PASSWORD='yes' --env MYSQL_DATABASE=$MYSQL_DATABASE --env MYSQL_USER=$MYSQL_USER --env MYSQL_PASSWORD=$MYSQL_PASS mysql:$MYSQL_VERSION

echo
echo "Sleeping 10 seconds to allow MySQL docker image to stabilize."
sleep 10
