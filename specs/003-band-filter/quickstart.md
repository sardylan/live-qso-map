# Quickstart: Band Filter

These steps validate the feature end to end.

## 1. Gates

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && node --test web/
```

## 2. Only considered bands (User Story 1)

```bash
cargo run --release -- --station-locator JM49NH --bands hf-contest
# log: bands considered: 160m 80m 40m 20m 15m 10m
printf '<CALL:4>K1AA <QSO_DATE:8>20261009 <TIME_ON:4>1500 <FREQ:6>14.025 <MODE:2>CW <GRIDSQUARE:4>FN42 <EOR>' | nc -u -w1 127.0.0.1 12060
printf '<CALL:4>K2BB <QSO_DATE:8>20261009 <TIME_ON:4>1501 <BAND:2>2m <MODE:2>FM <GRIDSQUARE:4>FN31 <EOR>' | nc -u -w1 127.0.0.1 12060
curl -s localhost:8080/healthz     # "history":1
```

**Expected**: the map shows only K1AA, and the legend lists 160m to 10m in frequency order.

## 3. Selections (User Story 2)

```bash
cargo run -- --bands hf,2m,-60m   # bands considered: 160m 80m 40m 30m 20m 17m 15m 12m 10m 2m
cargo run -- --bands hf-dx        # exits: invalid setting bands: unknown band or group 'hf-dx'
cargo run -- --bands warc,-warc   # exits: … leaves no bands
```

## 4. Correction onto an ignored band (User Story 3)

The integration test `tests/band_filter.rs` sends a QARTest `contactinfo` on 20m, then a
`contactreplace` with the same `<id>` and `<txfreq>535000</txfreq>` (5.35 MHz, 60m). It asserts
that every connected page receives `{"type":"remove"}` within 2 s and that the history is empty.
