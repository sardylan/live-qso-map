#!/bin/sh
# Tests scripts/check-release-files.sh (feature 015, FR-012). Runs in the test-tools gate.
# Packages are built with dpkg-deb where it exists (the Debian CI image); elsewhere a stub
# dpkg-deb reads the same file tree from a plain tar, so the test also runs on macOS.
#
# Usage: sh scripts/test-check-release-files.sh
set -u

here=$(cd "$(dirname "$0")" && pwd)
script="$here/check-release-files.sh"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

example="$work/config.example.toml"
printf '# example\n# key = "<value>"\n' > "$example"
printf '# another example\n' > "$work/other.toml"
printf '#!/bin/sh\n' > "$work/live-qso-map"

if ! command -v dpkg-deb >/dev/null 2>&1; then
    mkdir "$work/bin"
    cat > "$work/bin/dpkg-deb" <<'SH'
#!/bin/sh
[ "$1" = --fsys-tarfile ] && exec cat "$2"
exit 2
SH
    chmod 755 "$work/bin/dpkg-deb"
    PATH="$work/bin:$PATH"
    stub=1
else
    stub=0
fi

# tarball <out> <example or empty>
tarball() {
    rm -rf "$work/t" && mkdir "$work/t"
    cp "$work/live-qso-map" "$work/t/"
    set -- "$1" "${2:-}"
    if [ -n "$2" ]; then
        cp "$2" "$work/t/config.example.toml"
        tar -czf "$1" -C "$work/t" live-qso-map config.example.toml
    else
        tar -czf "$1" -C "$work/t" live-qso-map
    fi
}

# zipfile <out> <example or empty>
zipfile() {
    rm -rf "$work/z" && mkdir "$work/z"
    cp "$work/live-qso-map" "$work/z/"
    if [ -n "${2:-}" ]; then
        cp "$2" "$work/z/config.example.toml"
        (cd "$work/z" && python3 -m zipfile -c "$1" live-qso-map config.example.toml)
    else
        (cd "$work/z" && python3 -m zipfile -c "$1" live-qso-map)
    fi
}

# package <out> <example or empty>
package() {
    rm -rf "$work/d" && mkdir -p "$work/d/DEBIAN" "$work/d/usr/bin" "$work/d/usr/share/doc/live-qso-map"
    cp "$work/live-qso-map" "$work/d/usr/bin/"
    [ -n "${2:-}" ] && cp "$2" "$work/d/usr/share/doc/live-qso-map/config.example.toml"
    if [ "$stub" -eq 1 ]; then
        (cd "$work/d" && tar -cf "$1" --exclude ./DEBIAN .)
    else
        printf 'Package: lqm-test\nVersion: 1\nArchitecture: all\nMaintainer: test <t@example.org>\nDescription: test\n' \
            > "$work/d/DEBIAN/control"
        dpkg-deb --root-owner-group --build "$work/d" "$1" >/dev/null
    fi
}

# release <dir>: a complete, valid release in <dir>
release() {
    rm -rf "$1" && mkdir -p "$1"
    cp "$example" "$1/config.example.toml"
    tarball "$1/live-qso-map-1-x86_64.tar.gz" "$example"
    tarball "$1/live-qso-map-1-aarch64.tar.gz" "$example"
    zipfile "$1/live-qso-map-1-windows-x64.zip" "$example"
    package "$1/live-qso-map_1-1_amd64.deb" "$example"
}

failures=0

fail() {
    echo "FAIL: $*"
    failures=$((failures + 1))
}

# expect <case> <wanted exit> <text the output must contain>
expect() {
    out=$(sh "$script" "$work/r" "$example" 2>&1)
    got=$?
    if [ "$got" -ne "$2" ]; then
        fail "$1: exit $got, wanted $2 ($out)"
        return
    fi
    case "$out" in
        *"$3"*) ;;
        *) fail "$1: output lacks '$3' ($out)" ;;
    esac
}

r="$work/r"

release "$r"
expect "complete release" 0 "ok live-qso-map_1-1_amd64.deb"

release "$r"
rm "$r/config.example.toml"
expect "no standalone file" 1 "FAIL [config.example.toml]"

release "$r"
cp "$work/other.toml" "$r/config.example.toml"
expect "different standalone file" 1 "FAIL [config.example.toml]"

release "$r"
tarball "$r/live-qso-map-1-aarch64.tar.gz" ""
expect "archive without the example" 1 "FAIL [live-qso-map-1-aarch64.tar.gz]"

release "$r"
zipfile "$r/live-qso-map-1-windows-x64.zip" "$work/other.toml"
expect "zip with another example" 1 "FAIL [live-qso-map-1-windows-x64.zip]"

release "$r"
package "$r/live-qso-map_1-1_amd64.deb" ""
expect "package without the example" 1 "FAIL [live-qso-map_1-1_amd64.deb]"

release "$r"
rm "$r"/*.deb
expect "no package at all" 1 "FAIL [*.deb]"

out=$(sh "$script" 2>&1) && fail "no arguments: exit 0"
case "$out" in
    *usage*) ;;
    *) fail "no arguments: output lacks 'usage' ($out)" ;;
esac

if [ "$failures" -ne 0 ]; then
    echo "$failures check(s) failed"
    exit 1
fi
echo "check-release-files: all checks passed"
