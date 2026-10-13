---
description: "Task list for 012-page-fixes-browser-tests"
---

# Tasks: Page fixes and real-browser release tests

**Input**: Design documents from `/specs/012-page-fixes-browser-tests/`
**Prerequisites**: plan.md, spec.md, research.md (R1–R12), data-model.md, contracts/ (browser-test.md, pipeline.md, websocket-protocol.md), quickstart.md

**Tests**: MANDATORY (constitution Principle I). Write each test task first, run it, and confirm
it fails.
- **Rust:** `cargo test`. New tests use `?` where they return `Result`, and have no inline lint
  suppressions (LL-001).
- **Page and runner logic:** pure ES modules tested with `node --test 'web/*.test.js'`. CSS
  rules are checked with the `declarations()`/`uses()` helpers already in `web/themes.test.js`
  (feature 011, T039).
- **CI configuration:** `python3 -m unittest discover -s tools -p 'test_*.py'`, reading
  `.circleci/config.yml` as text, as `tools/test_ci_config.py` already does.
- **The browser run itself** is the release-time test. Its ability to catch a broken page is
  proven by hand (quickstart §3).

**Conventions:**
- **Runner:** it uses Node built-ins only: no `package.json`, no npm packages (research R2).
- **Order:** User Story 4's fixes come before the runner (User Story 3), because the runner's
  step 03 expects "HF contest" alone (B4). User Story 2 (pipeline) needs the runner.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story the task belongs to (US1–US4)

---

## Phase 1: Setup

- [ ] T001 Confirm that `.gitignore`'s `/target/` covers the runner's default output `target/browser-test/`, and create the folder `tests/browser/` with a short `tests/browser/README.md`. The README says:
  - what the runner is (contracts/browser-test.md);
  - that it needs Node 22+ and Chrome or Chromium, with no `npm install`;
  - that Cargo ignores the folder (only top-level `tests/*.rs` are test targets).

---

## Phase 2: Foundational (blocking prerequisites)

No shared prerequisite: User Story 1 is done, and the other stories start from their own tests.

---

## Phase 3: User Story 1 - The map page works again (Priority: P1) 🎯 MVP

**Goal**: every page file is served, so the page script runs (B1).

**Independent Test**: quickstart §1, first line; open the page and see the controls.

- [X] T002 [US1] Write the failing test `every_page_asset_on_disk_is_served` in `tests/ws_protocol.rs`. It fetches every `.js` and `.css` file in `src/web/assets/` and checks status 200, the content type and the exact body. It failed on `/legend.js`. (commit 563f0de)
- [X] T003 [US1] Embed and route `themes.js`, `sun.js`, `panel.js` and `legend.js` in `src/web/mod.rs`. Make T002 pass. (commit 563f0de)
- [X] T004 [US1] Add the B1 entry under `## [Unreleased]` → `### Fixed` in `CHANGELOG.md`. (commit 563f0de)

**Checkpoint**: the 3.0.0 page bug is fixed and guarded on every change.

---

## Phase 4: User Story 4 - The smaller page bugs are fixed (Priority: P2)

**Goal**:
- **B2:** an on toggle button has an accent outline;
- **B3:** the panel is opaque;
- **B4:** no two offered band groups have the same bands.

**Independent Test**: quickstart §4, plus the tests below.

### Tests for User Story 4 (write first, confirm they FAIL)

- [ ] T005 [P] [US4] Write failing Rust tests for B4 (data-model "Offered band groups", contracts/websocket-protocol.md).
  - **`src/band_filter.rs`:** `considered_groups`:
    - with the default set (`default_set()`), gives only `hf-contest`;
    - with `hf`, gives `hf`, `hf-contest`, `warc`, unchanged;
    - with `["40m", "20m"]`, gives only `hf-contest`;
    - with `["30m"]`, gives only `warc`;
    - with `None` (every band), gives all eight groups;
    - add a test that no two returned groups have equal `bands`, for each of these sets.
  - **`tests/ws_protocol.rs`:** a test that `hello.band_groups` from a service with the default bands has exactly one entry, `hf-contest`. Remember `test_config()` pins every band, so set `config.band_filter = Some(live_qso_map::band_filter::default_set())` in the test.
- [ ] T006 [P] [US4] Write failing node tests for B2 and B3 in `web/themes.test.js`.
  - **Palettes:** every palette has `panelSolid`, a 7-character `#rrggbb` equal to the first 7 characters of `panel`.
  - **Variables:** `panelSolid` is among the CSS variables `applyPalette` writes (`--panel-solid`; extend the `CSS_KEYS` list in the test).
  - **Contrast:** `accent` reaches at least 3:1 against `highlight` and against `bg` in every palette (WCAG 1.4.11).
  - **CSS:**
    - `.toggle-control button.active` has a `box-shadow` that uses `var(--accent)`;
    - `.panel` has a `background` that uses `var(--panel-solid)`;
    - `:root` in `app.css` defines `--panel-solid`.

### Implementation for User Story 4

- [ ] T007 [US4] In `src/band_filter.rs`, make `considered_groups` drop a group when another considered group has equal considered bands and is more specific. More specific means fewer bands in its full `GROUPS` definition; on a tie, the earlier one in `GROUPS` wins. Keep the result in `GROUPS` order and update the doc comment. Make T005 pass. If any feature 011 test assumed both `hf` and `hf-contest` with the default bands, fix it, for example in `src/web/protocol.rs` `hello_shape`.
- [ ] T008 [US4] In `src/web/assets/themes.js`, add `panelSolid` to each palette (light `#ffffff`, dim `#353d4d`, dark `#1b2028`) and `panelSolid: '--panel-solid'` to `CSS_VARIABLES`.
  - **`src/web/assets/app.css`:** define `--panel-solid: #ffffff` on `:root`, and set `.panel { background: var(--panel-solid); }`. Add `box-shadow: inset 0 0 0 2px var(--accent)` to `.toggle-control button.active`.
  - **Contrast:** if any palette's `accent` misses 3:1 against `highlight`, adjust `accent` or `highlight`, keeping every existing 4.5:1 text pair.

  Make T006 pass.

**Checkpoint**: B2–B4 are fixed, with tests.

---

## Phase 5: User Story 3 - The tested viewer session (Priority: P1)

**Goal**: `node tests/browser/run.mjs` plays the nine steps of contracts/browser-test.md against a
build and a local Chrome, with a report, a log and screenshots.

**Independent Test**: quickstart §2 passes. Quickstart §3 steps 1–2 fail on purpose and name the
404.

### Tests for User Story 3 (write first, confirm they FAIL)

- [ ] T009 [US3] Write failing node tests in `web/browser-lib.test.js` for a new `tests/browser/lib.mjs` (import `../tests/browser/lib.mjs`):
  - **`parseArgs(argv, env, platform)`:**
    - gives the defaults of contracts/browser-test.md (binary `target/release/live-qso-map`, `.exe` on `win32`; out `target/browser-test`; headless);
    - accepts `--binary`, `--chrome`, `--out`, `--headed`;
    - reads `CHROME_PATH` when `--chrome` is absent;
    - returns a usage error for an unknown option or a missing value.
  - **`chromeCandidates(platform, env)`:** the ordered list of paths to try per platform (`linux`, `darwin`, `win32`), as in the contract. **`findChrome(candidates, exists)`** returns the first one for which the injected `exists` is true, or `null`.
  - **`classifyEvent(event, origin)`:** returns `{ fatal: boolean }` per research R5:
    - fatal: an HTTP status ≥ 400 or a failed request on `origin`; any exception; a `console.error`;
    - not fatal: `/favicon.ico`; any other origin (tiles, CDN); a `console.error` that starts with `map error`; warnings; info.
  - **`createRun()` / `startStep(run, name)` / `finishStep(run, step, error)` / `skipRest(run, names)`:**
    - steps keep their order, status, time and message;
    - after a failure the remaining steps are `not-run`;
    - `run.result` is `fail` if any step failed.
  - **`waitFor(check, { timeout, interval, what, sleep, now })`:**
    - resolves with the first truthy result of `check`;
    - with an injected clock and `sleep`, rejects after `timeout` with a message naming `what` and the last value seen.
  - **`qsoAdif({ call, band, mode, grid, at })`:** the ADIF record the runner sends. It has `QSO_DATE` and `TIME_ON` in UTC (HR-1), and lengths are counted in bytes.

### Implementation for User Story 3

- [ ] T010 [US3] Implement `tests/browser/lib.mjs` with the functions of T009, pure and with no imports beyond `node:path`. Make T009 pass.
- [ ] T011 [US3] Implement `tests/browser/cdp.mjs`, the DevTools Protocol driver (research R2, R7), with Node built-ins only.
  - **`launchChrome({ chrome, profileDir, headed, ci, port })`:**
    - starts Chrome with `--remote-debugging-port`, a fresh `--user-data-dir`, `--no-first-run`, `--no-default-browser-check`, `--window-size=1400,900`, `--enable-unsafe-swiftshader` and `--use-angle=swiftshader`;
    - adds `--headless=new` unless headed, and `--no-sandbox` only when `ci`;
    - waits up to 10 s for `/json/list` and connects to the page target over the global `WebSocket`.
  - **The returned page object:**
    - `send(method, params)`;
    - an `events` array fed by `Runtime.consoleAPICalled`, `Runtime.exceptionThrown`, `Log.entryAdded`, `Network.loadingFailed` (with the request URL, kept from `Network.requestWillBeSent`) and `Network.responseReceived` (status);
    - `goto(url)` (waits for `Page.loadEventFired`), `eval(expression)` (returns the value, or throws on an exception), `screenshot(path)`;
    - `emulateDark(on)` (`Emulation.setEmulatedMedia`), `press(key)` (`Input.dispatchKeyEvent`, key down and up), `close()`, which kills Chrome.

  Domains to enable: Runtime, Log, Network, Page.
- [ ] T012 [US3] Implement `tests/browser/run.mjs`, the runner (contracts/browser-test.md, research R3–R6, R8).
  - **Arguments:** parse them with `lib.parseArgs`. Exit 2 when the binary or Chrome isn't found.
  - **The program:**
    - pick free HTTP and UDP ports by binding port 0 (`net`, `dgram`);
    - start it with `--http-port`, `--udp-port`, `--cty-path tests/fixtures/cty-excerpt.dat`, `--cty-max-age-days 0`, `--station-callsign IS0GVH` and `--station-locator JM49OF`, with every `LQM_*` variable removed from its environment;
    - write its stdout and stderr to `<out>/program-a.log` and `<out>/program-b.log`;
    - wait for `/healthz`.
  - **Chrome:** launch it with `ci` set from `process.env.CI === 'true'`.
  - **Steps:** run the nine steps of the contract table in order, each wrapped by `startStep`/`finishStep`.
    - **Waits and failures:** wait with `lib.waitFor`. Fail a step when a fatal event (`lib.classifyEvent`) arrived during it.
    - **Screenshots:** take one `NN-<step>.png` per step after its condition holds.
    - **QSOs:** send them as UDP datagrams built with `lib.qsoAdif`.
    - **Step 08:** stop the program (phase A), check the banner, then start phase B with `--bands hf`.
  - **Files:** write `report.json` and `console.log` (data-model "Browser test run").
  - **Output:** print one line per step and `browser tests: pass|fail`. Exit 0 or 1.
  - **Cleanup:** always stop Chrome and the program, also on `SIGINT` and `SIGTERM`.
- [ ] T013 [US3] **Verify** locally: `cargo build --release && node tests/browser/run.mjs` passes all nine steps, in under 2 minutes, with screenshots in `target/browser-test/`. Then do quickstart §3 steps 1–2: remove one `.route(...)` for a module in `src/web/mod.rs`, rebuild, and check that step 01 fails and names the 404. Restore it (don't commit the break).

**Checkpoint**: the browser tests run on a developer machine and catch the 3.0.0 bug.

---

## Phase 6: User Story 2 - A release can't ship a broken page (Priority: P1)

**Goal**: the release pipeline runs the browser tests on the release binary, and doesn't publish
when they fail.

**Independent Test**: quickstart §3 steps 3–4 (rehearsal), and the first release after the
merge.

### Tests for User Story 2 (write first, confirm they FAIL)

- [ ] T014 [US2] Add failing tests to `tools/test_ci_config.py`, a class `BrowserTests` (contracts/pipeline.md):
  - **The job:**
    - a job `browser-test` exists, using the image `cimg/node:24.21.0-browsers`;
    - it `fetch-verified`s `https://storage.googleapis.com/chrome-for-testing-public/154.0.8037.92/linux64/chrome-linux64.zip` with SHA-256 `ff43322f335e436b2f4dcdfeeec5db032299e335a7e8c1c618b326e100ce8732`;
    - it runs `node tests/browser/run.mjs` with `--binary ws/bin/x86_64-unknown-linux-musl/live-qso-map`;
    - it has a `store_artifacts` of `target/browser-test` with `when: always`.
  - **Release workflow:** the job uses `filters: *release-tags` and requires `build-linux-x86_64-unknown-linux-musl`, and `publish-github-release` requires `browser-test`.
  - **Rehearsal workflow:** the job is present there, and `rehearsal-summary` requires it.
  - **Elsewhere:** the job doesn't appear in the gates workflow or in the weekly scheduled workflow.

### Implementation for User Story 2

- [ ] T015 [US2] Add the `browser-test` job to `.circleci/config.yml` per contracts/pipeline.md.
  - **Workflows:** add it to the release workflow (with `filters: *release-tags`, after the x86_64 build) and to the rehearsal workflow; add it to the `requires` of `publish-github-release` and of `rehearsal-summary`.
  - **Artifacts:** add `when: always` on the `store_artifacts` step.
  - **Checks:** run `circleci config validate`, and make T014 pass.
- [ ] T016 [US2] Document the browser tests (FR-014):
  - **`docs/ci.md`:** a section after "Audit gate": when they run, what they block, the artifacts (`browser-test/` on the job's Artifacts tab), how to bump the Chrome pin, and how to run them locally. Also add the job to the rehearsal list in §7.
  - **`docs/releasing.md`:** `browser-test` must be green before `publish-github-release`, and where to look when it fails.
- [ ] T017 [US2] **Verify** in a rehearsal run on the pushed branch (`docs/ci.md` §7, `--param rehearsal=true`): `browser-test` passes, `rehearsal-summary` runs after it, and its artifacts hold the screenshots and `report.json`.

**Checkpoint**: no release can be published past a broken page.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [ ] T018 [P] Update `README.md`: in the development or testing section, the browser tests (`node tests/browser/run.mjs`, what they need, where the output goes, that CI runs them only for releases).
- [ ] T019 [P] Add entries under `## [Unreleased]` in `CHANGELOG.md`:
  - **Fixed:** the toggle on state is visible in every theme (B2); the settings panel is opaque (B3); the Legend group no longer offers two groups with the same bands (B4);
  - **Added:** release builds are tested in a real browser before publishing.
- [ ] T020 Run the local gates:
  - `cargo fmt --check` and `cargo clippy --locked --all-targets -- -D warnings`;
  - `cargo test --locked`;
  - `node --test 'web/*.test.js'`;
  - `python3 -m unittest discover -s tools -p 'test_*.py'`;
  - `RUSTDOCFLAGS="-D warnings -A rustdoc::bare_urls" cargo doc --locked --no-deps`;
  - and `node tests/browser/run.mjs` once more against a release build.
- [ ] T021 Mark the tasks done, commit per phase, push `feature/012-page-fixes-browser-tests` and open the pull request. Merge it once the five gates are green; delete the branch afterwards (constitution, Development Workflow).
- [ ] T022 With the maintainer's go-ahead, release 3.0.1 (FR-015, quickstart §5).
  - **Prepare:** `cargo release patch --no-tag --no-push --no-publish --execute` on `release/next`, then a PR.
  - **Tag:** after the merge, `cargo release tag --execute` and `cargo release push --execute`.
  - **Check:** `browser-test` passes before `publish-github-release`, and 3.0.1 is published (SC-007).

---

## Dependencies & Execution Order

- **US1 (T002–T004):** done.
- **US4 (T005–T008):** before US3. The runner's step 03 checks the B4 result.
- **US3 (T009–T013):** T009 → T010; T011 is independent of T010; T012 needs both; T013 last.
- **US2 (T014–T017):** after US3, because the job runs the runner. T014 → T015 → T016; T017 needs
  the branch pushed.
- **Polish (T018–T022):** last. T022 only after the merge, and only with the maintainer's
  go-ahead.
- **Shared files:**
  - `web/themes.test.js` (T006) and `src/web/assets/themes.js`/`app.css` (T008) belong to US4;
  - `tests/ws_protocol.rs` is touched by T005;
  - `CHANGELOG.md` by T004 (done) and T019.

## Parallel Opportunities

- **US4 tests:** T005 (Rust) and T006 (node) are separate files.
- **US3:** T011 (`cdp.mjs`) can be written while T009/T010 (`lib.mjs`) are done.
- **Polish:** T018 and T019 are separate files.

### Parallel example: User Story 4 tests

```text
Task: "T005 [US4] considered_groups and hello.band_groups tests"
Task: "T006 [US4] palette and CSS tests in web/themes.test.js"
```

## Implementation Strategy

1. **MVP (US1):** done. The page works again, guarded by a test.
2. **US4:** the three small fixes, each with its test.
3. **US3:** the browser runner, proven locally against a good and a broken build.
4. **US2:** the pipeline job, proven in a rehearsal.
5. **Polish:** docs, changelog, gates, PR, merge, then release 3.0.1 with the maintainer.
