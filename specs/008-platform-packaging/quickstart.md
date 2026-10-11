# Quickstart: verify feature 008

Sections 1–3 run on any development host. Sections 4–7 use the artifacts of a release tag, built
by the pipeline (constitution Principle V). To test before a real release, push a tag such as
`v2.0.0` to a fork.

## 1. Help (any host)

```bash
cargo run -- -h            # one line per option, no "Environment:" text
cargo run -- --help        # each option also shows "Environment: LQM_… Default: …"
COLUMNS=80 cargo run -- --help | awk 'length > 80' | wc -l    # 0
cargo run -- --udp-prot 1  # exit 2, "tip: a similar argument exists: '--udp-port'"
NO_COLOR=1 cargo run -- --help | grep -c $'\e\['             # 0
```

## 2. Configuration export (any host)

```bash
cargo run -- --station-callsign IS0MZK --station-locator JM49NH --bands hf-contest \
  --save-config /tmp/lqm.toml          # exits at once; "configuration written to /tmp/lqm.toml"
ls -l /tmp/lqm.toml                    # -rw------- on Linux and macOS
cargo run -- --config /tmp/lqm.toml --save-config -    # same settings printed (header date aside)
cargo run -- --bands nope --save-config /tmp/x.toml    # exit 1, unknown band; /tmp/x.toml not created
cargo run -- --save-config /nonexistent/dir/x.toml     # exit 1, "cannot write configuration file …"
cargo run -- --dump-config /tmp/lqm.toml               # writes, then the map starts
```

**Expected**: while `--save-config` runs, no port is opened. Check with
`lsof -i :12060` during a slow run, or simply note that the program exits at once.

## 3. File next to the program (any host)

```bash
cargo build
./target/debug/live-qso-map --station-callsign TEST1 --save-config target/debug/live-qso-map.toml
cd /tmp && ~/…/target/debug/live-qso-map   # logs show station TEST1
```

**Expected on Windows and macOS**: `cty.dat` is downloaded to `target/debug/`, not `/tmp`.
**Expected on Linux**: `cty.dat` goes to the current folder, as before.

## 4. Debian package (Debian 13 or Ubuntu 24.04, amd64)

```bash
sudo apt install ./live-qso-map_2.0.0-1_amd64.deb
systemctl is-enabled live-qso-map     # disabled
systemctl is-active live-qso-map      # inactive; ss -lun | grep 12060 → nothing
getent passwd live-qso-map            # …:/var/lib/live-qso-map:/usr/sbin/nologin, GECOS "live-qso-map service"
ls -l /etc/live-qso-map/config.toml   # -rw-r----- root live-qso-map
sudo live-qso-map --station-callsign IS0MZK --config /etc/live-qso-map/config.toml \
  --save-config /etc/live-qso-map/config.toml
sudo systemctl enable --now live-qso-map
ls /var/lib/live-qso-map/              # cty.dat after the first download
sudo apt install --reinstall ./live-qso-map_2.0.0-1_amd64.deb   # config kept, service restarted
sudo apt remove live-qso-map           # config, data and account remain
sudo apt purge live-qso-map            # /etc/live-qso-map, /var/lib/live-qso-map and the account gone
```

Running `--save-config` as root rewrites the file as root-owned `0600`. Restore the packaged
ownership with `sudo chown root:live-qso-map /etc/live-qso-map/config.toml && sudo chmod 0640 …`.
The read-me and the `postinst` message say so.

## 5. Container image

```bash
docker run --rm -p 12060:12060/udp -p 8080:8080 -v lqm-data:/var/lib/live-qso-map \
  -v "$PWD/config.toml:/etc/live-qso-map/config.toml:ro" sardylan/live-qso-map:2.0.0
docker run --rm sardylan/live-qso-map:2.0.0 --version
docker run --rm --entrypoint id sardylan/live-qso-map:2.0.0    # uid=10001
```

Repeat on an ARM64 host (a Raspberry Pi 4/5 or Apple silicon with Docker).

## 6. Windows (Windows 11, x64)

1. Extract `live-qso-map-2.0.0-windows-x64.zip` to a new folder and double-click
   `live-qso-map.exe`.
2. A console opens and `http://localhost:8080/` shows the map. `cty.dat` appears in the folder.
3. Start a second copy while the first runs. The port is busy, so the error stays visible with
   `Press Enter to close this window.`.
4. In PowerShell: `.\live-qso-map.exe --udp-port 0 < NUL`. On error it exits at once, with no
   wait.

## 7. macOS (Apple silicon)

1. Extract `live-qso-map-2.0.0-macos-arm64.zip` and double-click `live-qso-map`. Confirm "Open
   Anyway" once in System Settings → Privacy & Security.
2. Terminal opens, the map works, and `cty.dat` appears next to the program, not in the home
   folder.

## 8. Release integrity

```bash
sha256sum -c SHA256SUMS                # every line OK
git tag v9.9.9 && git push origin v9.9.9   # on a fork: the check-version job fails, nothing is published
```
