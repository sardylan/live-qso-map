# Verification: MapLibre Map Rendering

**Date**: 2026-10-09
**Build**: branch `002-maplibre`
**Host**: macOS (Apple Silicon), Rust 1.98.1, Node 26.11.1

## Quality gates

| Gate | Result |
|------|--------|
| `cargo fmt --check` | ✅ pass |
| `cargo clippy --all-targets -- -D warnings` | ✅ pass |
| `cargo test` | ✅ 209 passed, 0 failed, 1 ignored (`load_ws`) |
| `node --test web/` | ✅ 44 passed (feature-001 modules unchanged; new `view.js` and `layers.js` tests) |
| Leaflet leftovers in `src/`, `README.md`, `deploy/` | ✅ none |

## Success criteria

| ID | Criterion | Evidence | Status |
|----|-----------|----------|--------|
| SC-001 | Feature-001 frontend behaviour on the new renderer | Pure modules and tests unchanged (FR-007). New `layers.js` tests cover `[lon, lat]` order, zRank order, great-circle lines and antimeridian continuity. `ws_protocol` checks the assets, MapLibre 5.24.0 SRI and the absence of Leaflet. | ✅ automated checks pass; the owner tested in a browser on 2026-10-09 and reported no problems (T016) |
| SC-002 | ≥ 30 fps with 2,000 QSOs, globe and flat | GPU circle and line layers (research R3) | ✅ accepted as observed by the owner on 2026-10-09: no problems seen; no fps figure recorded (T029) |
| SC-003 | Live QSO on every page within 2 s | unchanged test `tests/ws_protocol.rs::every_page_receives_hello_snapshot_and_live_upserts` | ✅ |

## Requirement checks done with the real binary

| Check | Result |
|-------|--------|
| No base-map settings → `hello.basemap` = OpenFreeMap style with its credit, protocol 2 | ✅ |
| `--tile-url … --tile-attribution …` → `hello.basemap` = raster with that credit | ✅ |
| `--style-url` together with `--tile-url` | ✅ exits: `invalid setting map.style_url: style_url and tile_url are mutually exclusive` |
| `--tile-url` without an attribution | ✅ exits: `invalid setting map.tile_attribution: raster tiles need an attribution (FR-009)` |
| Assets `/`, `/app.js`, `/view.js`, `/layers.js` served with the right types | ✅ |

## Browser checks

- **T016 and T026**: done by the owner on 2026-10-09: "tested in a browser, it seems that there
  are no problems".
- **T029 (SC-002)**: accepted as observed by the owner on 2026-10-09. No problems were seen; the
  30 fps figure was not measured. To measure it later: `scripts/send_burst.sh 2000`, select All,
  and record fps in globe and flat with the browser's Performance panel.
