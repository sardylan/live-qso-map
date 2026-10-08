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
| SC-001 | Feature-001 frontend behaviour on the new renderer | Pure modules and tests unchanged (FR-007). New `layers.js` tests cover `[lon, lat]` order, zRank order, great-circle lines and antimeridian continuity. `ws_protocol` checks the assets, MapLibre 5.24.0 SRI and the absence of Leaflet. | ⚠️ automated parts pass; **visual check in two browsers pending** (T016) |
| SC-002 | ≥ 30 fps with 2,000 QSOs, globe and flat | GPU circle and line layers (research R3) | ⚠️ **measurement pending** (T029) |
| SC-003 | Live QSO on every page within 2 s | unchanged test `tests/ws_protocol.rs::every_page_receives_hello_snapshot_and_live_upserts` | ✅ |

## Requirement checks done with the real binary

| Check | Result |
|-------|--------|
| No base-map settings → `hello.basemap` = OpenFreeMap style with its credit, protocol 2 | ✅ |
| `--tile-url … --tile-attribution …` → `hello.basemap` = raster with that credit | ✅ |
| `--style-url` together with `--tile-url` | ✅ exits: `invalid setting map.style_url: style_url and tile_url are mutually exclusive` |
| `--tile-url` without an attribution | ✅ exits: `invalid setting map.tile_attribution: raster tiles need an attribution (FR-009)` |
| Assets `/`, `/app.js`, `/view.js`, `/layers.js` served with the right types | ✅ |

## Pending manual checks (need a real browser)

1. **T016**: quickstart §2 (parity, including an antimeridian path, e.g. station `JM49` and a QSO
   in `BL11`), §2b (block `tiles.openfreemap.org` → banner, QSOs still drawn) and §5 (no WebGL →
   message).
2. **T026**: quickstart §4. The page opens as a globe; Flat, then reload, opens flat; a second
   browser still opens as a globe.
3. **T029**: quickstart §6. `scripts/send_burst.sh 2000`, select All, record fps in globe and flat
   together with the machine (CPU, GPU, OS) and the browser version.
