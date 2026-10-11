# Contract: Release Artifacts (feature 008)

Produced only by the CircleCI pipeline for tags `vX.Y.Z` (constitution Principle V). `X.Y.Z`
MUST equal the `version` in `Cargo.toml`, otherwise the workflow stops before building
(FR-002).

## GitHub release `vX.Y.Z`

| File | Content |
|------|---------|
| `live-qso-map_X.Y.Z-1_amd64.deb` | Debian package, x86-64 |
| `live-qso-map_X.Y.Z-1_arm64.deb` | Debian package, ARM64 |
| `live-qso-map_X.Y.Z-1_armhf.deb` | Debian package, 32-bit ARM |
| `live-qso-map-X.Y.Z-windows-x64.zip` | `live-qso-map.exe`, `LICENSE.md`, `README.txt` |
| `live-qso-map-X.Y.Z-macos-arm64.zip` | `live-qso-map` (executable bit set), `LICENSE.md`, `README.txt` |
| `live-qso-map-X.Y.Z-x86_64-unknown-linux-musl` | static binary |
| `live-qso-map-X.Y.Z-aarch64-unknown-linux-musl` | static binary |
| `live-qso-map-X.Y.Z-armv7-unknown-linux-musleabihf` | static binary |
| `SHA256SUMS` | `sha256sum` output for every file above, one line each |

The release is created only after every build, package and test job has succeeded. With any
failure, no file is attached (SC-008).

## Container image (Docker Hub)

| Item | Value |
|------|-------|
| Repository | `sardylan/live-qso-map` |
| Tags | `X.Y.Z` and `latest` (`latest` moves only with a release tag) |
| Platforms | `linux/amd64`, `linux/arm64` (one multi-platform manifest) |
| Base | `debian:trixie-slim` |
| User | `10001:10001` (not root) |
| Entry point | `/usr/bin/live-qso-map`, plus `--config /etc/live-qso-map/config.toml` when that file exists |
| Environment | `LQM_CTY_PATH=/var/lib/live-qso-map/cty.dat` |
| Volume | `/var/lib/live-qso-map` |
| Ports | `12060/udp`, `8080/tcp` |
| Labels | `org.opencontainers.image.source`, `.version`, `.licenses=GPL-3.0-only`, `.title` |
| Contents | the binary, `ca-certificates`, `/usr/share/doc/live-qso-map/LICENSE.md` |

## Read-me files in the zips

Plain text with CRLF line endings on Windows and LF on macOS. Each covers:
1. start by double-clicking the program; a terminal window opens;
2. open `http://localhost:8080/`; the logger sends to UDP port 12060;
3. settings: create `live-qso-map.toml` next to the program, for example with
   `live-qso-map --station-callsign IS0MZK --save-config live-qso-map.toml`;
4. the country data file `cty.dat` is downloaded next to the program;
5. macOS only: the first start of this unsigned program needs System Settings → Privacy &
   Security → Open Anyway.

## Pipeline variables (CircleCI project settings)

| Variable | Used by | Content |
|----------|---------|---------|
| `GITHUB_TOKEN` | `publish-github-release` | token with `contents: write` on the repository |
| `DOCKERHUB_USERNAME` | `docker-image` | Docker Hub account |
| `DOCKERHUB_TOKEN` | `docker-image` | Docker Hub access token with write access to `sardylan/live-qso-map` |
