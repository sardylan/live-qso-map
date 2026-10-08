# Quickstart: Live QSO Map

How to build, run and validate the feature end to end. Each section maps to user stories and success
criteria in [spec.md](spec.md).

## Prerequisites

- **To build**: a stable Rust toolchain (1.85 or later, edition 2024).
- **To run the frontend unit tests** (development only): Node.js 20 or later, using the built-in
  `node --test`. No npm packages are needed.
- **For cross-compiled releases**: `cross` and Docker.
- **To view the map**: a browser with Internet access, for map tiles and the Leaflet CDN.

## 1. Quality gates (constitution II)

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
node --test web/            # frontend state and geodesic tests
```

## 2. Run locally with no lookup providers (User Story 1)

```bash
cargo run --release -- \
  --station-callsign IS0MZK --station-locator JM49NH \
  --udp-port 12060 --http-port 8080
```

Open <http://localhost:8080> in two browser windows, then send one ADIF QSO:

```bash
printf '<CALL:5>DL5AB <QSO_DATE:8>20261008 <TIME_ON:6>150404 <BAND:3>40m <MODE:3>SSB <GRIDSQUARE:6>JO62QM <EOR>' \
  | nc -u -w1 127.0.0.1 12060
```

**Expected**: both windows show one 40m-coloured marker near Berlin (JO62QM), with a great-circle line
from JM49NH, within 2 s (SC-001). `GET /healthz` reports `"history":1`.

Send the same datagram again. **Expected**: nothing changes (FR-008).

## 3. Multi-record ADIF datagram (FR-003)

```bash
printf '<ADIF_VER:5>3.1.4 <EOH><CALL:4>K1AA <QSO_DATE:8>20261008 <TIME_ON:4>1510 <BAND:3>20m <MODE:2>CW <GRIDSQUARE:4>FN42 <EOR><CALL:4>K2BB <QSO_DATE:8>20261008 <TIME_ON:4>1511 <BAND:3>20m <MODE:2>CW <GRIDSQUARE:4>FN42 <EOR><CALL:3>XX1 <QSO_DATE:3>BAD<EOR>' \
  | nc -u -w1 127.0.0.1 12060
```

**Expected**: one 20m marker at the centre of FN42, with a popup listing K1AA and K2BB (same position,
FR-024). The malformed third record is logged at warn level, and the service keeps running (SC-006).

## 4. Replay the QARTest reference capture (User Stories 4 and 5, SC-009)

The integration test `tests/qartest_replay.rs` sends every packet in
`specs/001-live-qso-map/reference/qartest-16.9.1-cqww-ssb.txt` through the pipeline. It uses the
cty.dat fixture and no online providers, so every QSO resolves to a country and no
"position not found" warning is logged. It asserts:

- 5 history entries: DL5QFN, B5Q, K6QA, K2WQX and HB9FQZ;
- K6QA at `rev = 3` with `SRX_STRING = 3`;
- HB9FQZ at `rev = 2` with `SRX_STRING = 14`;
- no warnings, and no event from the 2 `dynamicresults` messages.

```bash
cargo test --test qartest_replay
```

## 5. Online lookups (User Story 2)

```bash
export LQM_QRZ_USERNAME=... LQM_QRZ_PASSWORD=...
export LQM_HAMQTH_USERNAME=... LQM_HAMQTH_PASSWORD=...
cargo run --release -- --station-callsign IS0MZK --station-locator JM49NH
printf '<CALL:6>DL5QFN <QSO_DATE:8>20261008 <TIME_ON:6>150404 <BAND:3>40m <MODE:3>SSB <EOR>' \
  | nc -u -w1 127.0.0.1 12060
```

**Expected**: the marker appears within `lookup_timeout + 2 s` (SC-002). The popup shows the position
source and precision. The broadcast record contains `APP_LIVEQSOMAP_POS_SOURCE` and
`APP_LIVEQSOMAP_POS_PRECISION`. Without credentials, the position falls back to the cty.dat country
centre for DL.

Country-level check (SC-007): `IS0XYZ` is placed in Sardinia, `IT9XYZ` in Sicily and `I1XYZ` in mainland
Italy.

## 6. Last-N selector (User Story 3)

Send 150 QSOs, for example with `scripts/send_burst.sh 150` (a dev helper added during implementation).
Then open a new window.

**Expected**:

- it shows 20 QSOs (SC-003);
- selecting 100 shows 100, and selecting All shows 150;
- another window left at 20 is not affected.

## 7. Load (SC-004)

On a Raspberry Pi 4, run the release binary, open 50 WebSocket clients (`tests/load_ws.rs`, an ignored
test run manually), and send 10 QSOs per second for 60 s.

**Expected**: every client receives all 600 upserts, and none is lost.

## 8. Deployment (SC-008)

```bash
# static binaries
cross build --release --target x86_64-unknown-linux-musl
cross build --release --target aarch64-unknown-linux-musl
cross build --release --target armv7-unknown-linux-musleabihf

# container
docker build -t live-qso-map .
docker run -p 12060:12060/udp -p 8080:8080 \
  -e LQM_STATION_CALLSIGN=IS0MZK -e LQM_STATION_LOCATOR=JM49NH \
  -v lqm-data:/data -e LQM_CTY_PATH=/data/cty.dat live-qso-map

# systemd (Raspberry Pi or mini PC)
sudo install -m755 target/aarch64-unknown-linux-musl/release/live-qso-map /usr/local/bin/
sudo install -m644 deploy/live-qso-map.service /etc/systemd/system/
sudo install -Dm640 deploy/config.example.toml /etc/live-qso-map/config.toml
sudo systemctl enable --now live-qso-map
```
