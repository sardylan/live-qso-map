# Implementation Plan: Page fixes and real-browser release tests

**Branch**: `feature/012-page-fixes-browser-tests` | **Date**: 2026-10-13 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/012-page-fixes-browser-tests/spec.md`

## Summary

**Page fixes:**
- **B1, missing modules:** already fixed on this branch. The four modules are served, and a test
  fetches every page file on disk.
- **B2, toggle buttons:** an on toggle button gets an accent outline, tested for 3:1 contrast.
- **B3, panel:** the settings panel gets an opaque background (`--panel-solid`).
- **B4, legend groups:** the server sends no two band groups with the same considered bands. It
  keeps the more specific one.

**Browser tests:**
- **Runner:** `tests/browser/run.mjs` starts the program and Google Chrome, and drives the page
  over the Chrome DevTools Protocol with Node built-ins only. It plays nine steps, keeps a report,
  a log and screenshots, and exits non-zero on any failure.
- **Pipeline:** a new `browser-test` job runs it on release tags and in the rehearsal. It uses
  `cimg/node:24.21.0-browsers`, with Chrome for Testing 154 downloaded and SHA-256 verified.
  `publish-github-release` (and so `docker-promote`) waits for it.

**Release:** the fixes ship as 3.0.1.

## Technical Context

**Language/Version**:
- Rust (edition 2024, latest stable toolchain; see `docs/rust-guidelines.md` §2);
- browser ES modules;
- Node 22+ ES modules for the runner (CI: Node 24.21 from the image).

**Primary Dependencies**: none new.
- **Shipped program:** nothing added.
- **Runner:** Node built-ins only (`WebSocket`, `child_process`, `dgram`, `net`, `fs`).
- **CI-only:** the `cimg/node:24.21.0-browsers` image (pinned tag), and Chrome for Testing
  154.0.8037.92 (pinned URL and SHA-256).

**Storage**: none. Run artifacts go to `target/browser-test/`.

**Testing**:
- **Every change:** `cargo test`, `node --test 'web/*.test.js'` (adds the runner's pure helpers,
  the CSS and palette rules), and `python3 -m unittest` (adds CI configuration checks).
- **Releases:** `node tests/browser/run.mjs`.

**Target Platform**:
- **Runner:** any developer host with Node 22+ and Chrome or Chromium (Linux, macOS, Windows).
- **Pipeline:** the job runs on Linux x86_64 only.

**Project Type**: single binary, plus a dev and CI test runner.

**Performance Goals**: a browser run of at most 2 minutes, and under 5 minutes added to a release
pipeline (SC-003).

**Constraints**:
- no npm packages;
- the run needs no account, secret or callbook;
- tile failures must not fail it;
- Chrome runs without its sandbox only in CI.

**Scale/Scope**: nine steps, two program phases, about ten QSOs.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First**:
  - **B1:** the regression test came first and failed on `/legend.js`.
  - **B2–B4:** each gets a failing test first: CSS rules, palette contrast, `considered_groups`
    cases.
  - **Runner and CI:** the runner's pure helpers get node tests, and the CI job gets
    `test_ci_config.py` checks, before they are written.
  - **Browser run:** it is itself a test. Its "it catches a broken page" behaviour is proven in
    quickstart §3.
- [x] **II. Rust Safety & Quality**:
  - **Scope:** the only Rust change is `considered_groups`, which is pure.
  - **Runtime code:** no `unwrap` or `expect` outside tests.
  - **Gates:** fmt and clippy stay clean.
- [x] **III. Simplicity & Minimal Dependencies**:
  - **Crates:** no new crate.
  - **Runner:** no npm package. The DevTools Protocol driver is a few hundred lines of Node
    built-ins.
  - **Program:** still a single binary.
- [x] **Domain rules**: the legend still lists bands in frequency order (step 01 checks the
  order). QSO times sent by the runner are UTC.
- [x] **IV. Persistent Viewer Settings**: no new setting. Step 06 checks a saved setting survives
  a reload.
- [x] **V. Portable Development, Pipeline-Built Releases**:
  - **Runner:** it is Node and finds Chrome on Linux, macOS and Windows. No Bash is needed.
  - **Optional:** it isn't one of the merge gates, so a developer without Chrome is never
    blocked.
  - **Releases:** still built only by the pipeline. The job tests the pipeline's own x86_64
    binary.
- [x] **Interface rules**: `hello.band_groups` keeps its shape, and the contract is updated
  (contracts/websocket-protocol.md). No new endpoint.

**Post-design re-check (after Phase 1):** all items still pass. No violation, so Complexity
Tracking is empty.

### Dependency Justification

| Crate | Why needed | Alternatives considered (std / existing dep) |
|-------|------------|----------------------------------------------|
| — | no new crate | — |

**CI-only tools:**

| Tool | Pin | Why | Alternatives |
|------|-----|-----|--------------|
| `cimg/node:24.21.0-browsers` | exact tag | Chrome's system libraries, Mesa (software WebGL), Node 24, `unzip` (maintainer's choice, clarification) | `node:24` + Debian `chromium`; Playwright image (research R1) |
| Chrome for Testing 154.0.8037.92 `linux64` | URL + SHA-256 `ff43322f…8732` | the browser itself; the image has none | `browser-tools` orb (unpinned) |

## Project Structure

### Documentation (this feature)

```text
specs/012-page-fixes-browser-tests/
├── spec.md
├── plan.md
├── research.md            # R1–R12
├── data-model.md          # test run, palette key, offered groups
├── quickstart.md
├── contracts/
│   ├── browser-test.md         # runner command, options, exit codes, the nine steps
│   ├── pipeline.md             # browser-test job, where it sits in each workflow
│   └── websocket-protocol.md   # hello.band_groups without duplicates
├── checklists/requirements.md
└── tasks.md               # /speckit-tasks
```

### Source Code (repository root)

```text
src/web/mod.rs                  # B1 (done): serves themes/sun/panel/legend
src/band_filter.rs              # B4: considered_groups keeps the most specific of equal groups
src/web/assets/themes.js        # B3: panelSolid in each palette, --panel-solid
src/web/assets/app.css          # B2: accent outline on .toggle-control button.active; B3: .panel opaque
tests/ws_protocol.rs            # B1 test (done); hello.band_groups with the default bands
tests/browser/run.mjs           # new: runner entry (arguments, phases, steps, report)
tests/browser/cdp.mjs           # new: DevTools Protocol driver (Chrome start, send, events, screenshots)
tests/browser/lib.mjs           # new: pure helpers (arguments, Chrome lookup, event classification, waits)
web/browser-lib.test.js         # new: node tests for lib.mjs (run by test-web)
web/themes.test.js              # B2/B3 rules and contrast
.circleci/config.yml            # browser-test job in the release and rehearsal workflows
tools/test_ci_config.py         # job filters, pins, publish requires browser-test
docs/ci.md, docs/releasing.md   # the browser tests: when, what they block, artifacts, local run
README.md, CHANGELOG.md         # how to run them; Unreleased entries for 3.0.1
```

**Structure Decision**: existing layout.
- **Runner:** it sits in `tests/browser/` next to the Rust integration tests. Cargo only compiles
  the top-level `tests/*.rs`, and the `web/*.test.js` glob doesn't reach into it.
- **Helper tests:** they sit in `web/`, so the existing test-web gate runs them.

## Complexity Tracking

> Fill ONLY if the Constitution Check has violations that must be justified.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|--------------------------------------|
