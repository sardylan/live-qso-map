# Browser tests

`run.mjs` starts live-qso-map and Google Chrome (or Chromium), opens the map page and plays a short
viewer session over the Chrome DevTools Protocol: QSOs, the settings panel, the themes, a reload,
a service restart and a band-group choice (steps 01–09), then the globe's glow after hand moves
and an auto-zoom flight (10 `atmosphere`), the grayline and Sun symbol on both views
(11 `grayline`), the auto-zoom fit of a world-wide snapshot (12 `autozoom-fit`), and the
station status line on a fresh history (13 `station`). The steps and the
rules for a failure are in `specs/012-page-fixes-browser-tests/contracts/browser-test.md`,
`specs/013-atmosphere-follows-globe/contracts/browser-test.md` and step 13 in
`specs/017-station-status-box/research.md` (R8).

- **Test clock:** from step 10 on, the page's clock is shifted to the June solstice moment when
  the Sun is 90° west of the house, so the expected glow side and night side don't depend on
  when the run happens.
- **Smoothness:** step 11 records frame rates (`framesPerSecond` in `report.json`) but doesn't
  judge them: the test browser renders in software.

```bash
cargo build --release
node tests/browser/run.mjs            # --headed to watch, --chrome / --binary / --out to override
```

- **Needs:** Node 22 or newer, and Chrome or Chromium installed. There is no `npm install`: the
  runner uses Node built-ins only.
- **Output:** `target/browser-test/` holds `report.json`, `console.log`, the program's logs and one
  screenshot per tested state.
- **CI:** the release pipeline runs it on every release tag and in the rehearsal, before anything
  is published (`docs/ci.md`).
- **Cargo:** only the top-level `tests/*.rs` files are Cargo test targets, so this folder is
  ignored by `cargo test`. The runner's pure helpers (`lib.mjs`) are tested by
  `web/browser-lib.test.js`.
