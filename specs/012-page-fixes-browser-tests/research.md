# Research: Page fixes and real-browser release tests

Decisions for feature 012. Each one gives the decision, the rationale and the alternatives.

## R1. Where Chrome comes from in the pipeline

**Decision:**
- **Image:** run the job in `cimg/node:24.21.0-browsers`, pinned by its exact version tag.
- **Chrome:** inside it, download **Chrome for Testing 154.0.8037.92** (`linux64`) with the
  existing `fetch-verified` command:
  - URL: `https://storage.googleapis.com/chrome-for-testing-public/154.0.8037.92/linux64/chrome-linux64.zip`;
  - SHA-256: `ff43322f335e436b2f4dcdfeeec5db032299e335a7e8c1c618b326e100ce8732` (196 MB, built
    2026-09-28).

**Rationale:**
- **The image has no Chrome.** The maintainer chose CircleCI's browser image (clarification).
  Its Dockerfile (`cimg-node/24.21/browsers`) installs Chrome's system libraries, Mesa for
  software WebGL, Xvfb and Java, but not Chrome. CircleCI expects the `browser-tools` orb to add
  a browser.
- **Why Chrome for Testing:** it is Google's versioned Chrome build for automation. Every version
  stays downloadable at a fixed URL, so it can be pinned and checksum-verified like the other CI
  tools (feature 009, FR-016).
- **Why this version:** both pins are more than two weeks old. The image tag was published
  2026-09-09, and the Chrome build is the last 154 release. The newer stable 155 build is from
  2026-10-06, under two weeks old.
- **What else the image gives:** `unzip`, Python 3 and Node 24 (`cimg/base:2026.08`, Ubuntu 24.04),
  and it runs as the unprivileged `circleci` user.

**Alternatives considered:**
- **`browser-tools` orb `install-chrome`:** installs the current stable Chrome, so it isn't
  pinned. It also adds an orb, a third-party pipeline dependency.
- **Debian `chromium` package in `node:24`:** this was the first recommendation. The maintainer
  chose the CircleCI image instead.
- **`mcr.microsoft.com/playwright`:** ships Chromium, but it is about 2 GB, and it is only worth it
  with the Playwright npm library.

## R2. How the tests drive Chrome

**Decision:**
- **No npm packages:** a small driver of the Chrome DevTools Protocol, written with Node 24
  built-ins only:
  - the global `WebSocket` (stable since Node 22);
  - `child_process` to start Chrome and the program;
  - `dgram` to send QSOs;
  - `fs` and `net`.
- **No new files of that kind:** no `package.json` and no lockfile.

**Rationale:**
- **It's enough:** the needed protocol surface is small:
  - navigate; evaluate script; screenshot;
  - emulate `prefers-color-scheme`;
  - send keys;
  - receive console messages, exceptions and network failures.

  The driver used for the manual check on 2026-10-13 (about 130 lines) already did all of it
  against the broken and the fixed page.
- **Nothing to pin:** the tests add no dependency to pin, audit or update (constitution III and
  V). They run on any developer machine that has Node 22+ and Chrome or Chromium.

**Alternatives considered:**
- **Puppeteer or Playwright (npm):** richer, but each brings a package tree, a lockfile, its own
  browser download and a supply-chain surface to audit. Rejected for the small surface needed.
- **WebDriver via the image's Selenium 3:** an old protocol. It needs a matching chromedriver
  (another pin) and can't see console errors as well.

## R3. Which program the tests run

**Decision:**
- **Pipeline:** the `x86_64-unknown-linux-musl` release binary, taken from the workspace that
  `build-linux` already persists (`ws/bin/x86_64-unknown-linux-musl/live-qso-map`).
- **Locally:** any build, `--binary` defaulting to `target/release/live-qso-map(.exe)`.

**Rationale:** FR-009 asks for the released program, not a separate build. The musl binary is
static, so it runs in the Ubuntu-based browser image.

## R4. The program's settings during a run

**Decision:** the runner starts the program itself, with:
- **Ports:** free ports for HTTP and UDP, found by binding port 0 first.
- **Country data:** `--cty-path tests/fixtures/cty-excerpt.dat --cty-max-age-days 0` (no
  download).
- **Station:** `--station-callsign IS0GVH --station-locator JM49OF`.
- **No callbook:** no provider credentials. Every `LQM_*` variable is removed from the child's
  environment, so no account or secret is involved (FR-013).
- **Config file:** none (`--config` isn't passed).

It runs twice:
- **Phase A:** with the default bands, for scenarios 1–4 and 6.
- **Phase B:** restarted with `--bands hf`, for scenarios 5 and 7. The restart is the
  reconnection test (scenario 7), and it also brings a new `hello`, with three distinct groups
  (`hf`, `hf-contest`, `warc`) for the legend scenario.

**Rationale:**
- **One reconnection, two jobs:** the restart covers both reconnection and a `hello` with other
  groups.
- **Phase B is needed:** with the default bands and the B4 fix, only "HF contest" is offered. A
  group choice that changes what is shown needs groups with different bands.

## R5. What fails a run

**Decision:** a step fails on any of these:
- **Failed request:** a request to the program's own origin that fails or returns a status of 400
  or more. The favicon is ignored: the program has none, by design.
- **Script error:** `Runtime.exceptionThrown`.
- **Console error:** a `console.error`, except the page's own `map error` reports. Those come
  from base-map tiles and are kept in the log.
- **Timeout:** a wait that passes its limit. Each wait names what it was waiting for.

These only add a precise message:
- **MapLibre missing:** `maplibregl` undefined after load → "MapLibre could not be loaded from its
  CDN".
- **No WebGL:** `#no-webgl` shown → "WebGL unavailable".

Map-library warnings and base-map tile failures never fail a run (FR-007).

**Rationale:** these are the signs of the 3.0.0 bug: four 404s and a page script that never ran.
Tiles depend on a third-party server, so they must not decide a release (edge case).

## R6. Waiting without flaky sleeps

**Decision:**
- **Waits:** every wait polls a condition in the page every 100 ms, up to a limit: 15 s for the
  first load, 5 s for the rest.
- **Map moves:** auto-zoom moves animate. The tests wait for the condition they need (a legend
  row, a box row, a theme attribute), not for the animation to end.
- **Screenshots:** taken after the condition holds, plus one animation frame.

**Rationale:** a slow CI machine then makes the run slower, not red.

## R7. Running Chrome in the container

**Decision:** start Chrome with:
- `--headless=new`, `--no-sandbox`, `--enable-unsafe-swiftshader`, `--use-angle=swiftshader`;
- `--window-size=1400,900`;
- a fresh `--user-data-dir` per run.

`--no-sandbox` is used only when the runner sees it is in CI (`CI=true`).

**Rationale:**
- **Sandbox:** CircleCI Docker jobs run without the user namespaces the Chrome sandbox needs. The
  container is thrown away after the job, so turning the sandbox off there is the usual practice.
- **WebGL:** SwiftShader gives software WebGL, which the page needs (edge case: no GPU).
- **Profile:** a fresh profile means empty local storage, so each run starts with the defaults.

## R8. Artifacts

**Decision:** every run writes to `target/browser-test/` (or `--out`):
- `report.json`: steps, results, timings;
- `console.log`: every console message, exception and failed request, with times;
- `NN-<state>.png`: one screenshot per tested state.

The CircleCI job keeps the folder with `store_artifacts`, whether the run passed or failed. The
test step saves the runner's exit code instead of failing, `store_artifacts` runs next, and a last
step exits with the saved code. (`when: always` is documented for `run` steps only, so it isn't
relied on for `store_artifacts`.)

**Rationale:** FR-011 and SC-004. A failure must be understandable without a re-run.

## R9. Where the gate goes in the pipeline

**Decision:** a new job `browser-test`:
- **In the release workflow:** runs on release tags, requires `build-linux-x86_64-unknown-linux-musl`,
  and is added to the `requires` of `publish-github-release`. `docker-promote` already requires
  `publish-github-release`, so a failure blocks both (FR-010).
- **In the rehearsal workflow:** added too, and required by `rehearsal-summary`.
- **Elsewhere:** not in the gates workflow, so it never runs on pull requests, on `main` pushes
  or on the weekly schedule (FR-008).

**Rationale:**
- **Timing:** it runs alongside the packaging jobs. It starts once the x86_64 build is ready and
  takes about 2 minutes, which is within SC-003's 5 minutes.
- **Image tag:** `image-push` only pushes an `-rc` tag, which is never shown to users until it is
  promoted. So it doesn't need to wait.

## R10. Unit tests for the runner itself

**Decision:**
- **Pure helpers:** the runner's pure parts live in `tests/browser/lib.mjs`:
  - argument parsing;
  - finding Chrome on Linux, macOS and Windows;
  - classifying console and network events;
  - the step and wait bookkeeping.
- **Tests:** they have node tests in `web/browser-lib.test.js`, which the existing
  `node --test 'web/*.test.js'` gate (test-web) runs on every change.
- **CI configuration:** `tools/test_ci_config.py` gains checks:
  - the job exists only on release tags and in the rehearsal;
  - its Chrome download is pinned and verified;
  - `publish-github-release` requires it.

**Rationale:** constitution I. The parts that can be tested without a browser are tested on
every change. The browser run itself is the release-time test.

## R11. The page fixes

**B1 (done on this branch, commit 563f0de):**
- **Fix:** the four modules are routed in `src/web/mod.rs`.
- **Test:** `every_page_asset_on_disk_is_served` fetches every `.js` and `.css` in
  `src/web/assets/` and compares the body.

**B2, button on state:** `.toggle-control button.active` gains `box-shadow: inset 0 0 0 2px
var(--accent)`. Tests:
- the CSS rule (the `web/themes.test.js` CSS reader from T039);
- the accent colour reaches at least 3:1 against both `highlight` and `bg` in every palette. That
  is WCAG 1.4.11 for non-text UI parts. Text stays at 4.5:1.

**B3, panel shows through:**
- **New variable:** a `--panel-solid` CSS variable (palette key `panelSolid`), the panel colour
  without alpha.
- **Use:** `.panel` uses it. The legend and the recent box keep their light see-through
  background over the map.
- **Tests:** the palette tests check `panelSolid` is opaque and matches `panel`'s colour; a CSS
  test checks `.panel` uses it.

**B4, duplicate groups:**
- **Change:** `band_filter::considered_groups` drops a group whose considered bands equal those
  of another considered group that is more specific. "More specific" means fewer bands in its
  full definition; on a tie, the first in `GROUPS` wins. The kept groups stay in `GROUPS` order.
- **Rust tests:**
  - default bands → only `hf-contest`;
  - `hf` → `hf`, `hf-contest` and `warc`;
  - `["40m", "20m"]` → only `hf-contest` (`hf` has the same two, and more in its definition);
  - every band → every group, unchanged.
- **Contract:** `hello.band_groups` changes accordingly (contracts/websocket-protocol.md). The
  shape is the same, so it is still protocol 4.

**Rationale:** each fix is the smallest one that removes the symptom seen on 2026-10-13, with a
test that fails first.

## R12. Release 3.0.1

**Decision:** after the merge, the release is made with the feature 009 process:
- `cargo release patch` on `release/next`;
- the PR, then `cargo release tag` and `push`.

The changelog's "Unreleased" already holds the B1 entry. B2–B4 and the browser tests are added
there too.

**Rationale:** FR-015. 3.0.0's page is broken, and the fix is a patch release.
