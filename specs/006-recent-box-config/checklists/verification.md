# Verification: Configurable Last-QSO Box

**Feature**: [spec.md](../spec.md) | **Quickstart**: [quickstart.md](../quickstart.md)

## Automated (2026-10-09)

| Gate / criterion | Result |
|------------------|--------|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass (0 warnings) |
| `cargo test` | 269 passed, 0 failed, 1 ignored (`load_ws`, as before) |
| `node --test web/` | 124 passed, 0 failed |
| SC-001 (T002, T003, T008) | column order and content for the default, all seven keywords, single and reordered lists |
| SC-002 (T002, T003, real binary) | unknown keyword, repeated keyword and empty list all exit with `invalid setting map.recent_columns: …`; an invalid `--recent-header` is rejected by the argument parser (`LQM_RECENT_HEADER=maybe` gives `invalid setting map.recent_header`) |
| SC-003 (T017) | every map size (10, 20, 50, All) × box size (3–20) × order gives the expected counts |
| SC-007 (T004) | 20 callsigns, including the VY1 and R8H zone overrides, match the cty.dat CQ zone |

## Headless browser check (2026-10-09, 1280×800, real cty.dat)

- With map 10, box 20 and newest at the top, for 25 QSOs:
  - the box lists 20 rows, newest first;
  - the 10 rows outside the map window are faded, with the "Not on the map" tooltip;
  - the other 10 can be clicked.
- The header shows Call, Band, UTC, CQ, Country.
- The panel shows "Map: last 10 QSOs", then "Box: last 20 · Newest: top", visibly separate.
- The selects were restored from local storage.

## Manual (browser), to be run by the project owner

- [ ] T015: quickstart §2, operator columns, header, startup errors and CQ zones
- [ ] T024: quickstart §3, viewer size and order, faded rows, browser restart, alignment at
  1024 px and 1920 px (SC-005), and step 6 (settings changed while the box is hidden)
