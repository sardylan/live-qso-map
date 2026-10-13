# Browser tests

`run.mjs` starts live-qso-map and Google Chrome (or Chromium), opens the map page and plays a short
viewer session over the Chrome DevTools Protocol: QSOs, the settings panel, the themes, a reload,
a service restart and a band-group choice. The steps and the rules for a failure are in
`specs/012-page-fixes-browser-tests/contracts/browser-test.md`.

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
