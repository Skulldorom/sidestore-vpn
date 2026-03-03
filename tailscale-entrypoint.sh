#!/bin/sh
set -e

/sidestore-vpn "$@" &
exec /usr/local/bin/containerboot
