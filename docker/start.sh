#!/bin/bash
set -eo pipefail

cd /agildata-zero

# this is a hack until agildata-zero can load host from an environmental variable
MY_IP_ADDRESS="$(hostname -i)"
sed -i -e "s/127.0.0.1/$MY_IP_ADDRESS/" zero-config.xml
if [ -n "$MYSQL_PORT_3306_TCP_ADDR" ]; then
  sed -i -e "s/127.0.0.1/$MYSQL_PORT_3306_TCP_ADDR/" zero-config.xml
fi

./agildata-zero --config zero-config.xml --logconfig log.toml
