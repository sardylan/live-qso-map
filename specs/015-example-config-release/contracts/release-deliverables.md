# Contract: release deliverables carry the example

This contract extends `specs/008-platform-packaging/contracts/release-artifacts.md` and
`debian-package.md`. Where they differ, this contract wins.

## Contents per deliverable

| Deliverable | Contents (root of the archive, or installed paths) |
|-------------|------------------------------------------------------|
| `live-qso-map-<v>-<target>.tar.gz` | `live-qso-map`, `LICENSE.md`, `config.example.toml` |
| `live-qso-map-<v>-windows-x64.zip` | `live-qso-map.exe`, `LICENSE.md`, `README.txt`, `config.example.toml` |
| `live-qso-map-<v>-macos-arm64.zip` | `live-qso-map`, `LICENSE.md`, `README.txt`, `config.example.toml` |
| `live-qso-map_<v>-1_<arch>.deb` | `/usr/bin/live-qso-map`, `/etc/live-qso-map/config.toml` (short live file, conffile), `/usr/share/doc/live-qso-map/{README.md,copyright,config.example.toml}`, the unit |
| Container image | `/usr/bin/live-qso-map`, `/usr/share/doc/live-qso-map/{LICENSE.md,config.example.toml}`, … |
| GitHub release | everything above that is a file, plus `config.example.toml`, plus `SHA256SUMS` covering all of them |

## Pipeline gates

- `package-deb`: the contents check also requires `./usr/share/doc/live-qso-map/config.example.toml`,
  and `dpkg-deb --ctrl-tarfile | tar -xOf - ./conffiles` must be exactly
  `/etc/live-qso-map/config.toml`. The example is never a conffile, so upgrades overwrite it
  without asking; the live file follows dpkg's standard conffile flow.
- `image-test` → `scripts/test-image.sh`: a step `example` requires
  `/usr/share/doc/live-qso-map/config.example.toml` in the image.
- `publish-github-release`: run
  `sh scripts/check-release-files.sh ws/release deploy/config.example.toml` before the checksums
  and the upload. A non-zero exit stops the job: nothing is published.
- `rehearsal-summary`: run the same check after `store_artifacts`, so a failing rehearsal still
  keeps the files to inspect (nothing is published there anyway).
- `test-tools` (every commit): `sh scripts/test-check-release-files.sh` tests the script itself
  against valid and broken releases. It uses a real `.deb` where `dpkg-deb` exists, and a stub
  elsewhere (macOS).

## `scripts/check-release-files.sh`

```text
usage: check-release-files.sh <release-dir> <example>
exit 0  every check passed (prints one line per file checked)
exit 1  "FAIL [<file>]: <reason>" for the first failure
```

Checks, in order:
1. `<release-dir>/config.example.toml` exists and `cmp`s equal to `<example>`.
2. At least one `*.tar.gz`, `*.zip` and `*.deb` exists. An empty release is a failure, not a
   pass.
3. Each `*.tar.gz`: `tar -xzOf <f> config.example.toml` equals `<example>`.
4. Each `*.zip`: `unzip -p <f> config.example.toml` equals `<example>`.
5. Each `*.deb`: `dpkg-deb --fsys-tarfile <f> | tar -xOf - ./usr/share/doc/live-qso-map/config.example.toml`
   equals `<example>`.

POSIX `sh`, CI-only like `test-deb.sh` and `test-image.sh`. It is not a development step
(Principle V).
