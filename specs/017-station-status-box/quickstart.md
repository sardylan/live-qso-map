# Quickstart: Station Status Box

## 1. Gates

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && node --test web/
```

## 2. Status follows the latest QSO (User Story 1)

Start the service with an empty history file and `[station] callsign = "IS0MZK"`, and open the
page. QSOs are sent with `scripts/simulate_qartest_qso.py` (`--operator` added by this feature).

1. **Expected**: above the score line, the box reads `— · —`.
2. Send `--band 40 --operator IK0ABC DL5QFN`. **Expected**: within 2 s the box reads
   `40m · IK0ABC` (FR-003).
3. Send `--band 20 --operator IS0XYZ K6QA`. **Expected**: `20m · IS0XYZ` within 2 s.
4. Open a second tab. **Expected**: it shows `20m · IS0XYZ` at once (SC-002).
5. In the settings panel, show only 40m. **Expected**: the box still reads `20m · IS0XYZ` (FR-009).
6. Send a correction (`contactreplace`) for the 40m QSO with `<operator>IK0DEF</operator>`.
   **Expected**: the box is unchanged; the 40m QSO is not the latest (scenario US1 #5).
7. Send `--band 15 K2WQX` with an empty operator. **Expected**: `15m · IS0MZK`, the station
   callsign (FR-007).
8. Stop the service gracefully, start it again, reload the page. **Expected**: `15m · IS0MZK`
   before any new QSO (FR-005, SC-003).

## 3. Staleness (User Story 2)

1. Stop sending QSOs. **Expected**:
   - within 10 s of 15 minutes after the last QSO's time, the box turns grey (SC-004);
   - hovering shows `Last QSO HH:MMZ` in UTC.
2. Send a QSO. **Expected**: the normal look returns within 2 s.

## 4. Layout

At 1024 px and 1920 px wide, on the globe and on the flat map, in light and dark themes: the box
sits above the score line, overlaps neither the controls nor the last-QSO box, and auto-zoom keeps
QSOs clear of it (FR-011).

## 5. Browser step

```bash
cargo build --release
node tests/browser/run.mjs
```

The new `station` step must pass with the others. Per LL-007, run it once in a manual rehearsal
pipeline before merging.
