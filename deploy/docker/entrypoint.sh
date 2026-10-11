#!/bin/sh
# Container entry point (feature 008, contracts/release-artifacts.md "Container image").
# A mounted /etc/live-qso-map/config.toml is used as is, including its country_data.path;
# without one, downloaded data goes to the /var/lib/live-qso-map volume.
set -e

if [ -f /etc/live-qso-map/config.toml ]; then
    exec /usr/bin/live-qso-map --config /etc/live-qso-map/config.toml "$@"
fi
exec /usr/bin/live-qso-map --cty-path /var/lib/live-qso-map/cty.dat "$@"
