# Quickstart: Full-Screen Side Column

## 1. Gates

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && node --test web/
```

```bash
cargo build --release && node tests/browser/run.mjs
```

## 2. The column (User Stories 1 and 2)

Start the service with `[station] callsign = "IS0MZK"` and open the page full screen on a
1920×1080 screen. Send QSOs with `scripts/simulate_qartest_qso.py`.

1. **Expected**: a column runs the full height on the left; the map starts at its right edge and
   nothing covers it (FR-001, FR-003).
2. **Expected**, top to bottom: `— · —`, the UTC clock ticking every second, `Score —`, the last
   QSOs box, `v` + the version at the bottom (FR-002, FR-007, FR-011).
3. Send `--band 40 --operator IK0ABC DL5QFN`. **Expected**: `40m · IK0ABC` at the top, the QSO in
   the box, auto-zoom framing it in the map area (FR-004).
4. **Expected**: the status line is the largest text, the clock a little smaller, the score smaller
   still, the box smaller than the score (FR-008).
5. Untick "Shown" for the last QSOs box. **Expected**: the column stays, the box is gone, the
   version tag still at the bottom (US2 #3).
6. Read the column from 3 m. **Expected**: band, operator, clock and score read correctly (SC-001).

## 3. Sizes (User Story 3)

1. Open the same page full screen on a 4K screen of the same physical size. **Expected**: the text
   takes the same share of the height as on the 1080p screen (SC-003).
2. Use a 1280×720 window. **Expected**: nothing overflows or overlaps, the map keeps most of the
   window (US3 #3).
3. Narrow the window below 600 px. **Expected**: a strip above the map with status, clock, score and
   version; no last QSOs box (FR-005). Widen it again: the column and the box come back.

## 4. Automatic reload (User Story 5)

1. With a page open, stop the service and start it again with the same binary. **Expected**: the
   page reconnects and does not reload (US5 #2; the browser's network panel shows no new `/` load).
2. Bump `version` in `Cargo.toml` (for example to `3.4.0-test`), rebuild, restart. **Expected**:
   every open page reloads once by itself within 60 s and shows `v3.4.0-test` (SC-004); the limit,
   theme, legend and box choices are as before (FR-014).
3. Restart again with the same `3.4.0-test` binary. **Expected**: no reload.
4. Put back the original version, rebuild, keep the page open in a second tab from before step 2
   in the browser history, and go back to it. **Expected**: it reloads once and shows the version
   running (clarification Q3).
