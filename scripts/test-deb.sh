#!/bin/sh
# Debian package life-cycle test (feature 008, contracts/debian-package.md "Life-cycle test").
# Runs as root in a Debian or Ubuntu container with adduser and systemd installed; systemd is
# not running, so only offline queries (systemctl is-enabled) are used.
#
# Usage: sh scripts/test-deb.sh <path-to-amd64-deb>
set -u

deb=${1:?usage: test-deb.sh <path-to-deb>}
step=setup

fail() {
    echo "FAIL [$step]: $*"
    exit 1
}

# expect_mode <path> <octal mode> <owner> <group>
expect_mode() {
    got=$(stat -c '%a %U %G' "$1" 2>/dev/null) || fail "$1 is missing"
    [ "$got" = "$2 $3 $4" ] || fail "$1 is '$got', wanted '$2 $3 $4'"
}

expect_not_enabled() {
    state=$(systemctl is-enabled live-qso-map.service 2>/dev/null || true)
    [ "$state" != "enabled" ] || fail "the service is enabled"
}

install_deb() {
    dpkg -i --force-confold "$deb" >/dev/null || fail "dpkg -i failed"
}

step=install
install_deb
expect_mode /usr/bin/live-qso-map 755 root root
expect_mode /etc/live-qso-map/config.toml 640 root live-qso-map
expect_mode /var/lib/live-qso-map 750 live-qso-map live-qso-map
entry=$(getent passwd live-qso-map) || fail "no live-qso-map account"
[ "$(echo "$entry" | cut -d: -f7)" = /usr/sbin/nologin ] || fail "shell is not nologin: $entry"
[ "$(echo "$entry" | cut -d: -f5)" = "live-qso-map service" ] || fail "GECOS marker missing: $entry"
[ "$(echo "$entry" | cut -d: -f6)" = /var/lib/live-qso-map ] || fail "home is wrong: $entry"
expect_not_enabled
live-qso-map --version | grep -q '^live-qso-map ' || fail "--version does not run"

step=upgrade
echo "# test-deb edit" >> /etc/live-qso-map/config.toml
install_deb
grep -q '^# test-deb edit$' /etc/live-qso-map/config.toml || fail "the edited conffile was replaced"
expect_mode /etc/live-qso-map/config.toml 640 root live-qso-map
expect_not_enabled

step=save-over-conffile
live-qso-map --config /etc/live-qso-map/config.toml \
    --save-config /etc/live-qso-map/config.toml 2>/dev/null || fail "--save-config failed"
expect_mode /etc/live-qso-map/config.toml 640 root live-qso-map

step=remove
dpkg -r live-qso-map >/dev/null || fail "dpkg -r failed"
[ ! -e /usr/bin/live-qso-map ] || fail "the binary is still installed"
[ -d /etc/live-qso-map ] || fail "/etc/live-qso-map was removed"
[ -d /var/lib/live-qso-map ] || fail "/var/lib/live-qso-map was removed"
getent passwd live-qso-map >/dev/null || fail "the account was removed"
expect_not_enabled

step=purge
install_deb
dpkg -P live-qso-map >/dev/null || fail "dpkg -P failed"
[ ! -e /etc/live-qso-map ] || fail "/etc/live-qso-map remains"
[ ! -e /var/lib/live-qso-map ] || fail "/var/lib/live-qso-map remains"
! getent passwd live-qso-map >/dev/null || fail "the account remains"
! getent group live-qso-map >/dev/null || fail "the group remains"

step=foreign-account
adduser --system --group --no-create-home --home /nonexistent live-qso-map >/dev/null 2>&1 \
    || fail "cannot create the foreign account"
usermod -c "someone else" live-qso-map
install_deb
dpkg -P live-qso-map >/dev/null || fail "dpkg -P failed"
getent passwd live-qso-map >/dev/null || fail "purge deleted an account it did not create"
deluser --system --quiet live-qso-map >/dev/null 2>&1 || true

echo "test-deb: all life-cycle checks passed"
