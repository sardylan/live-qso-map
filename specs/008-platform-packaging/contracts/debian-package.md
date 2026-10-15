# Contract: Debian Package (feature 008)

> **Feature 015:** superseded for the example configuration and the Debian live configuration by
> [`specs/015-example-config-release/contracts/release-deliverables.md`](../../015-example-config-release/contracts/release-deliverables.md).

## Control fields

| Field | Value |
|-------|-------|
| Package | `live-qso-map` |
| Version | the crate version (`2.0.0`), plus cargo-deb's revision `-1` |
| Architecture | `amd64`, `arm64` or `armhf`, one package each |
| Depends | `adduser` |
| Section / Priority | `hamradio` / `optional` |
| Maintainer | `Luca Cireddu <sardylan@gmail.com>` (from `Cargo.toml` `authors`) |
| Description | `Cargo.toml` `description`, with the README's first paragraph as the extended text |
| Homepage | `Cargo.toml` `homepage` |

## Files

| Path | Owner | Mode | Kind |
|------|-------|------|------|
| `/usr/bin/live-qso-map` | root:root | 0755 | static binary |
| `/etc/live-qso-map/config.toml` | root:live-qso-map | 0640 | conffile (from `deploy/config.example.toml`) |
| `/usr/lib/systemd/system/live-qso-map.service` | root:root | 0644 | systemd unit |
| `/usr/share/doc/live-qso-map/README.md` | root:root | 0644 | documentation |
| `/usr/share/doc/live-qso-map/copyright` | root:root | 0644 | GPL-3.0 license text (FR-004) |
| `/var/lib/live-qso-map/` | live-qso-map:live-qso-map | 0750 | data folder, created by `postinst` and by `StateDirectory=` |

## Service unit

```ini
[Unit]
Description=Live QSO map
Documentation=https://github.com/sardylan/live-qso-map
After=network-online.target
Wants=network-online.target

[Service]
User=live-qso-map
Group=live-qso-map
ExecStart=/usr/bin/live-qso-map --config /etc/live-qso-map/config.toml
StateDirectory=live-qso-map
StateDirectoryMode=0750
Restart=on-failure
RestartSec=5
# existing hardening, unchanged: NoNewPrivileges, ProtectSystem=strict, ProtectHome, PrivateTmp,
# PrivateDevices, ProtectKernel*, ProtectControlGroups, RestrictAddressFamilies, …

[Install]
WantedBy=multi-user.target
```

## Maintainer scripts

Every script starts with `set -e` and contains `#DEBHELPER#`, which cargo-deb fills in with the
systemd fragments for `enable = false`, `start = false`.

| Script | Argument | Actions |
|--------|----------|---------|
| `postinst` | `configure` | 1. If `getent passwd live-qso-map` fails: `adduser --system --group --home /var/lib/live-qso-map --no-create-home --shell /usr/sbin/nologin live-qso-map`, then `usermod -c "live-qso-map service" live-qso-map` (data-model.md "Service account"). 2. Create `/var/lib/live-qso-map` (0750, account-owned) if missing. 3. Unless `dpkg-statoverride --list /etc/live-qso-map/config.toml` matches, set it to root:live-qso-map 0640. 4. `#DEBHELPER#`. 5. On upgrade (`$2` not empty): `deb-systemd-invoke try-restart live-qso-map.service` (restarts only if running). 6. On first install (`$2` empty): print the next steps (edit the file, `systemctl enable --now live-qso-map`). |
| `postrm` | `remove` | `#DEBHELPER#` (stop and disable, daemon-reload); nothing else |
| `postrm` | `purge` | 1. `#DEBHELPER#`. 2. Delete `/etc/live-qso-map/` and `/var/lib/live-qso-map/`. 3. If the account exists and its GECOS is `live-qso-map service`: `deluser --system --quiet live-qso-map`. 4. `delgroup --system --only-if-empty --quiet live-qso-map` when the group exists. |
| `prerm`, `preinst` | | only `#DEBHELPER#` (generated) |

- **Idempotency**: running any script twice with the same argument succeeds and changes nothing
  more.
- **No interaction**: the scripts never prompt. dpkg's own conffile prompt is unaffected.

## Life-cycle test (`scripts/test-deb.sh`, CI)

It runs in `debian:12`, `debian:13`, `ubuntu:22.04` and `ubuntu:24.04` containers on amd64, as
root. The test containers have no running systemd, so the service checks use `systemctl`'s
offline queries (`is-enabled`), and the start check is skipped.

| Step | Checks |
|------|--------|
| install | data-model.md "Package life cycle", first-install row: files, owners, modes; account shell and GECOS; unit not enabled |
| edit config, install again (upgrade path) | the edited file is unchanged; the account is unchanged; the unit is still not enabled |
| `--save-config /etc/live-qso-map/config.toml` as root | the file is still root:live-qso-map 0640 |
| remove | the binary is gone; `/etc/live-qso-map`, `/var/lib/live-qso-map` and the account remain; the unit is not enabled |
| install, then purge | no `/etc/live-qso-map`, no `/var/lib/live-qso-map`, no account, no group |
| pre-created account with another GECOS, install, purge | the account survives purge |
