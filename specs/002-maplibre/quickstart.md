# Quickstart: MapLibre Map Rendering

These steps validate the feature end to end.

## 1. Gates

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && node --test web/
```

## 2. Parity (User Story 1, SC-001)

Run feature 001's quickstart §2, §3 and §6. Then, in two browsers, check:

- markers merged by position, in the colour of the most recent QSO's band, newest on top;
- popups with call, band, mode, UTC time and source/precision;
- great-circle lines, including one across the 180° meridian, for example a QSO with `JM49` station
  and a Pacific locator such as `BL11` (Hawaii);
- the legend in frequency order, the last-N selector, and reconnection after a service restart.

## 2b. Base map unreachable (edge case)

In DevTools, block requests to `tiles.openfreemap.org` (Network → request blocking) and reload.
**Expected**: the banner "Base map unavailable; QSOs are still shown." appears, and markers and
lines still render on an empty background.

## 3. Base map (User Story 2)

```bash
cargo run --release --                                   # OpenFreeMap vector style; attribution visible
cargo run --release -- --tile-url 'https://tile.openstreetmap.org/{z}/{x}/{y}.png' \
  --tile-attribution '&copy; OpenStreetMap contributors' # raster
cargo run -- --tile-url 'https://t/{z}/{x}/{y}.png'
# exits: invalid setting map.tile_attribution: raster tiles need an attribution (FR-009)
cargo run -- --style-url https://example/style.json --tile-url 'https://t/{z}/{x}/{y}.png'
# exits: invalid setting map.style_url: style_url and tile_url are mutually exclusive
```

## 4. Globe (User Story 3)

The page opens as a globe. Use the toggle to switch to flat, reload, and it opens flat. A second
browser still opens as a globe.

## 5. No WebGL (FR-012)

In Chrome, start with `--disable-webgl`, or turn off hardware acceleration, and open the page.
**Expected**: the FR-012 message, not a blank page.

## 6. Performance (SC-002)

Run `scripts/send_burst.sh 2000`, select All, then pan and zoom for 10 s while recording with the
browser's Performance panel. **Expected**: at least 30 fps in both globe and flat.
