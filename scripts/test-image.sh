#!/bin/sh
# Container image smoke test (feature 008, contracts/release-artifacts.md "Container image").
# Runs in CI on the freshly built image, before anything is pushed.
#
# Usage: sh scripts/test-image.sh <image>
set -u

image=${1:?usage: test-image.sh <image>}
step=setup
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

fail() {
    echo "FAIL [$step]: $*"
    exit 1
}

step=version
docker run --rm "$image" --version | grep -q '^live-qso-map ' || fail "--version does not run"

step=user
uid=$(docker run --rm --entrypoint id "$image" -u) || fail "cannot run id"
[ "$uid" = 10001 ] || fail "runs as uid $uid, wanted 10001"

step=example
docker run --rm --entrypoint test "$image" -f /usr/share/doc/live-qso-map/config.example.toml \
    || fail "the example is not in /usr/share/doc/live-qso-map"

step=mounted-config
cat > "$work/config.toml" <<'TOML'
http_port = 18080

[country_data]
path = "/var/lib/live-qso-map/x.dat"
TOML
chmod 644 "$work/config.toml"
out=$(docker run --rm -v "$work/config.toml:/etc/live-qso-map/config.toml:ro" "$image" \
    --save-config -) || fail "--save-config with a mounted file failed"
echo "$out" | grep -q '^http_port = 18080$' || fail "the mounted file was not read"
echo "$out" | grep -q '^path = "/var/lib/live-qso-map/x.dat"$' \
    || fail "the mounted file's country_data.path was overridden"

step=default-data-path
out=$(docker run --rm "$image" --save-config -) || fail "--save-config without a file failed"
echo "$out" | grep -q '^path = "/var/lib/live-qso-map/cty.dat"$' \
    || fail "the data path is not /var/lib/live-qso-map/cty.dat"

echo "test-image: all checks passed"
