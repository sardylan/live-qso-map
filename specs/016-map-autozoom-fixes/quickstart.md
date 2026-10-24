# Quickstart: Map Auto-Zoom Fixes

## Automated checks

```bash
node --test web/
cargo test
node tests/browser/run.mjs
```

- `web/autozoom-geo.test.js`: the border check and the centred flat frame on the snapshot
  positions (research R7).
- `cargo test`: the `map.globe_border_deg` layering, its range, `hello`, and the example config.
- The browser step for the snapshot at 1024 × 768 and 1920 × 1080 (spec SC-004).

## Manual check

1. Start the service with a station position in Sardinia (`--station-latitude 39.05
   --station-longitude 8.38`). Send the snapshot QSOs (spec Assumptions) over ADIF UDP.
2. Open the page in a 1920 × 1080 window. Choose the globe, then turn auto-zoom on.
3. Expect the flat map (button reads "Globe", tooltip says auto-zoom switched to flat). The house
   is in the middle of the area left clear of the panels, and both Australian QSOs and Sweden are
   in view, with no repeated world.
4. Remove the two VK QSOs (or set the limit so that only European QSOs show). Expect the globe,
   centred over Europe, filling the free area.
5. Restart with `--globe-border-deg 45` and repeat step 4. Expect flat if the European cap plus
   45° no longer fits inside the horizon.
6. Restart with `--globe-border-deg 50`. Expect a startup error naming `map.globe_border_deg`.
