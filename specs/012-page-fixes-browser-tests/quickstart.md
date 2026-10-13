# Quickstart: Page fixes and real-browser release tests

## 1. Automated checks (every change)

```bash
cargo test --locked --test ws_protocol every_page_asset   # B1: every page file is served
cargo test --locked band_filter                           # B4: no duplicate groups
node --test 'web/*.test.js'                               # B2/B3 CSS and palette rules; runner helpers
python3 -m unittest discover -s tools -p 'test_*.py'      # the browser-test job and its pins
```

## 2. Browser tests on your machine

You need Node 22+ and Chrome or Chromium installed.

```bash
cargo build --release
node tests/browser/run.mjs                    # finds Chrome; uses target/release/live-qso-map
node tests/browser/run.mjs --headed           # watch it
node tests/browser/run.mjs --chrome /path/to/chrome --binary /path/to/live-qso-map
```

**Expected:**
- **Output:** nine passed steps, `browser tests: pass`, and exit code 0.
- **Files:** `target/browser-test/` holds:
  - `report.json`;
  - `console.log`, with no fatal entries;
  - screenshots `01-load.png` … `09-legend-group.png` (step 04 gives three: light, dim, dark).

## 3. Prove that the gate stops a broken page

1. Remove one `.route(...)` line from `src/web/mod.rs` (locally; for the rehearsal, as a temporary commit, task T017).
2. Build and run step 2 above: `load` fails and names the 404.
3. Start a rehearsal (`docs/ci.md` §7): `browser-test` fails, and `rehearsal-summary` doesn't run.
4. Restore the line (`git revert` of the temporary commit). Both pass.

## 4. Look at the fixes by hand (optional)

- **B2:** in Dim, with Auto-zoom on, the button has an accent outline. Off, it has none.
- **B3:** open the panel over the legend. Nothing shows through it.
- **B4:** with the default bands, the Legend group offers only "HF contest (6 bands)". With
  `--bands hf`, it offers HF (10 bands), HF contest (6 bands) and WARC (3 bands).

## 5. Release 3.0.1

After the merge, follow `docs/releasing.md`:
- **Prepare:** `cargo release patch`, which opens a PR.
- **Tag:** `cargo release tag`, then `push`.

**Check:** the release pipeline shows `browser-test` green before `publish-github-release`, and
its artifacts hold the screenshots.
