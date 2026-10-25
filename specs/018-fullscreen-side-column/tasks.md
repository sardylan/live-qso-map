---
description: "Task list for 018-fullscreen-side-column"
---

# Tasks: Full-Screen Side Column

**Input**: Design documents from `/specs/018-fullscreen-side-column/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: MANDATORY (constitution Principle I). Write each test task before the implementation
it covers, run it, and confirm it fails first. Use `cargo test` for Rust and `node --test web/`
for the frontend.

**Conventions** (as in features 001–017):

- **Errors**: the crate `Error`/`Result` (`docs/rust-guidelines.md`); nothing in this feature is
  fallible, so no new error variant.
- **Rules**: `docs/ham-radio.md` §1 (HR-1 UTC); `docs/http-api.md` (API-2 typed JSON frames, API-4
  `snake_case`; an additive field keeps protocol 4).
- **Page code**: decisions live in pure modules with `web/*.test.js` tests; `app.js` is glue only.
  Values reach the DOM only through `textContent`.
- **Rust tests**: unit tests in `#[cfg(test)] mod tests` in the same file; integration tests use
  `tests/common`.
- **Commits**: one commit per phase, with no co-author trailers.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1–US5 from spec.md

**Story order**: US2 (the column) comes first because US1, US3 and US4 place or size things inside
it; both are P1. US5 needs only the foundational `hello.version`.

---

## Phase 1: Setup

- [X] T001 Confirm the baseline is green before any change: `cargo test` and `node --test web/`,
  run from the repository root, pass on `feature/018-fullscreen-side-column`. No file changes.

---

## Phase 2: Foundational (the version, used by US4 and US5)

**Purpose**: the service tells each page its version, in `hello` and in the served `index.html`
(research R5, R7; contract `contracts/websocket-protocol.md`).

### Tests ⚠️ write first, see them fail

- [X] T002 [P] Unit test in `src/web/protocol.rs` (`#[cfg(test)] mod tests`): the JSON of
  `ServerMessage::hello(..)` has `"version"` equal to `env!("CARGO_PKG_VERSION")` and `"protocol"`
  still `4`; extend the existing hello serialisation test next to the `assert_eq!(v["protocol"], 4)`
  asserts rather than adding a new fixture.
- [X] T003 [P] Contract tests in `tests/ws_protocol.rs`:
  - in `every_page_receives_hello_snapshot_and_live_upserts`, assert
    `hello["version"] == env!("CARGO_PKG_VERSION")`;
  - new `the_page_carries_the_service_version`: `GET /` returns a body that contains
    `<meta name="lqm-version" content="{CARGO_PKG_VERSION}">` and does not contain
    `__LQM_VERSION__`, with `content-type` `text/html` and `cache-control: no-cache`.

### Implementation

- [X] T004 In `src/web/protocol.rs`: add `version: &'static str` to `ServerMessage::Hello`, right
  after `protocol`, doc comment `/// The service's software version (feature 018, research R5).`,
  filled with `env!("CARGO_PKG_VERSION")` in `ServerMessage::hello`. T002 passes.
- [X] T005 In `src/web/assets/index.html` add `<meta name="lqm-version" content="__LQM_VERSION__">`
  in `<head>` after the viewport meta. In `src/web/mod.rs`: a
  `static INDEX_PAGE: LazyLock<String>` = `INDEX_HTML.replace("__LQM_VERSION__", env!("CARGO_PKG_VERSION"))`
  with a one-line comment (feature 018, R5), and the `/` route serving it with the same headers as
  `asset` (`text/html; charset=utf-8`, `no-cache`). Keep `asset` for every other file. T003 passes.

**Checkpoint**: `hello.version` and the page meta exist; nothing on the page uses them yet.

---

## Phase 3: User Story 2 - A permanent column beside the map (Priority: P1) 🎯 MVP

**Goal**: the station line, the score line and the last QSOs box live in a full-height column
beside the map; below 600 px it is a strip above the map without the box (FR-001 to FR-006,
research R1, R2).

**Independent Test**: quickstart §2 steps 1, 3 and 5, and §3 step 3.

### Tests for User Story 2 ⚠️ write first, see them fail

- [X] T006 [US2] Browser step `column` in `tests/browser/run.mjs` (add `'column'` at the end of
  `STEPS`; set sizes with CDP `Emulation.setDeviceMetricsOverride` and clear it at the end of the
  step). Checks, after at least one QSO is on the page:
  - at 1920×1080 and at 1280×720: `#column` has `top` 0 and `height` equal to the window height;
    `#map`'s left edge equals `#column`'s right edge (±1 px); `#map`'s right edge is the window's;
  - `#station`, `#score` and `#recent` are inside `#column` (DOM ancestry) and none of their
    rectangles overlaps `#map`'s;
  - at 1920×1080 with the box shown, `#map`'s `dataset.padding` left value is at most what the
    map's own left-hand controls account for (their right edge plus `FIT.gap`): auto-zoom adds no
    padding for the column (FR-004); before this change it is the box's width plus `FIT.gap`,
    above 300;
  - `#status`, made visible, is horizontally centred on `#map` (±2 px) and does not overlap
    `#column`;
  - at 375×812: `#column` spans the full width above `#map` (`#map`'s top equals `#column`'s
    bottom), `#recent` is not displayed (`offsetParent === null`), `#station` and `#score` are;
  - back at 1920×1080: `#recent` is displayed again.
  Save a screenshot per size. Run it and see it fail (no `#column` yet).

### Implementation for User Story 2

- [X] T007 [US2] In `src/web/assets/index.html`: replace `<div id="corner-bl">…</div>` with
  `<aside id="column" class="column">` placed first in `<body>`, holding in order `#station`,
  `#score`, `#recent` (ids, classes and `aria-*` unchanged); update the comment to cite feature 018
  FR-002. After it, a `<main id="stage">` holding `#map`, `#no-webgl`, `#status`, `#basemap-error`
  and `#legend` (research R1). `#panel` stays a child of `<body>`.
- [X] T008 [US2] In `src/web/assets/app.css`:
  - `body`: `display: flex; height: 100%`; `#stage`: `position: relative; flex: 1;
    min-width: 0`; `#map`: `height: 100%` (keep its background); `.banner`, `.legend` and
    `.no-webgl` keep their absolute positions, now relative to `#stage`;
  - `.column`: `display: flex; flex-direction: column; width: clamp(280px, 24rem, 45vw);
    flex-shrink: 0; box-sizing: border-box; overflow: hidden; background: var(--panel-solid);
    color: var(--text); box-shadow: 2px 0 8px var(--shadow)`; `#recent` inside it
    `flex: 1; min-height: 0; overflow: hidden`;
  - `@media (max-width: 599px)`: `body { flex-direction: column }`, `.column` full width with
    auto height, `#recent { display: none }`, `#stage { flex: 1; min-height: 0 }`;
  - delete `.corner-bl`, `.corner-bl[hidden]` and `body.scale-on .corner-bl`; drop the floating
    `background`/`box-shadow` from `.recent, .score` now that the column has them.
- [X] T009 [US2] In `src/web/assets/app.js`:
  - remove `'#recent'`, `'#score'` and `'#station'` from `PANELS` (R2);
  - the no-WebGL branch: replace `show('corner-bl', false)` with `show('column', false)`;
  - remove any remaining `corner-bl` reference (`grep -n corner-bl src/web/assets`) and the
    `scale-on` toggle in `updateScale` (its only CSS rule goes in T008).
  T006 passes.
- [X] T010 [P] [US2] Node tests in `web/recent.test.js` for `rowsThatFit(rowHeights, available)`:
  `[20, 20, 20]` in 60 → 3; in 59 → 2; in 0 → 0; `[]` → 0; a header height counted by the caller
  (pass `available` already reduced). Then export `rowsThatFit` from `src/web/assets/recent.js`
  (how many whole rows fit, counting from the newest), with a doc comment citing feature 018 R1.
- [X] T011 [US2] In `src/web/assets/app.js`, after every `renderRecent()` and on window resize:
  measure the visible rows of `#recent tbody`, call `rowsThatFit`, and set the `hidden` attribute
  on the oldest rows beyond that count (the top rows when newest is at the bottom, the bottom rows
  otherwise), clearing it on the others, so the box can grow back. Rows are never removed.
- [X] T012 [US2] Run quickstart §2 steps 1, 3 and 5 and §3 step 3, in Light and Dark, globe and
  flat; click a QSO row in the box (FR-006, US2 #5).
  *Closed by the owner after the visual test, 2026-10-25.*

**Checkpoint**: the column works on its own (MVP).

---

## Phase 4: User Story 1 - Read the station status from across the room (Priority: P1)

**Goal**: the top of the column shows band, operator and a ticking UTC clock in the largest text,
the score below in smaller text (FR-007 to FR-009, research R4, R8).

**Independent Test**: quickstart §2 steps 2, 4 and 6.

### Tests for User Story 1 ⚠️ write first, see them fail

- [X] T013 [P] [US1] Node tests in `web/station.test.js` for `utcClock(date)`:
  `new Date('2026-10-25T00:00:00Z')` → `'00:00:00 UTC'`;
  `new Date('2026-10-25T09:05:07Z')` → `'09:05:07 UTC'`;
  `new Date('2026-10-25T23:59:59.999Z')` → `'23:59:59 UTC'`;
  a date built from a local-time string with a non-zero offset gives the UTC hour, not the local one.
- [X] T014 [US1] Extend the browser step `column` in `tests/browser/run.mjs`:
  - `#clock` is inside `#column` between `#station` and `#score` (DOM order) and matches
    `/^\d\d:\d\d:\d\d UTC$/`; after 2 s its text has changed; its hour equals
    `new Date().getUTCHours()` evaluated in the page (the test clock shift from step 10 applies to
    both);
  - computed `font-size`: `#station` > `#clock` > `#score` > the `#recent td` cells;
  - `#clock` never has the `stale` class, even while `#station` has it.

### Implementation for User Story 1

- [X] T015 [P] [US1] In `src/web/assets/station.js` export `utcClock(date)` returning
  `HH:MM:SS UTC` from `getUTCHours`/`getUTCMinutes`/`getUTCSeconds`, zero-padded, with a JSDoc line
  citing feature 018 R4 and HR-1. T013 passes.
- [X] T016 [US1] In `src/web/assets/index.html` add
  `<div id="clock" class="clock" aria-hidden="true"></div>` between `#station` and `#score`
  (`aria-hidden`: a per-second change must not flood screen readers).
- [X] T017 [US1] In `src/web/assets/app.js`: import `utcClock`; `renderClock()` writes
  `utcClock(new Date())` to `#clock` via `textContent`; call it once at start-up and from a new
  `setInterval(renderClock, 1000)` in the WebGL branch next to the existing intervals.
- [X] T018 [US1] In `src/web/assets/app.css`: column sizes from data-model.md: `.station`
  `2.4rem` bold, `.clock` `2rem` with `font-variant-numeric: tabular-nums`, `#score` `1.6rem`;
  long values wrap (`overflow-wrap: anywhere`) inside the column; `.stale` still greys
  `#station` and `#score` only. T014 passes (with T022's root size, sizes are relative; the order
  check holds already).
- [X] T019 [US1] Run quickstart §2 steps 2, 4 and 6; send a portable call `IS0/IK0ABC/P` as
  operator and check it stays inside the column.
  *Closed by the owner after the visual test, 2026-10-25.*

**Checkpoint**: US2 and US1 work.

---

## Phase 5: User Story 3 - Text sized for monitors and TVs (Priority: P2)

**Goal**: every text size scales with the window height, with a 13 px floor (FR-010, research R3,
TR-026).

**Independent Test**: quickstart §3 steps 1 and 2.

### Tests for User Story 3 ⚠️ write first, see them fail

- [X] T020 [US3] Extend the browser step `column` in `tests/browser/run.mjs`:
  - the root (`html`) computed `font-size` is `13px` at 1280×720, between `17px` and `18px` at
    1920×1080 and twice the 1080 value (±1 px) at 3840×2160;
  - at 3840×2160 the ratio `#station` height / window height is within 10% of the 1920×1080 ratio
    (SC-003);
  - the legend's computed `font-size` at 1920×1080 is larger than 13 px;
  - `#column`'s width / window width at 3840×2160 is within 10% of the same ratio at 1920×1080.
- [X] T021 [US3] Unit test in `src/web/mod.rs` (`#[cfg(test)] mod tests`, add the module if
  missing), `text_sizes_scale_with_the_root`: every `APP_CSS` `font-size:` or `font:` declaration
  in `px` is either the root's `clamp(13px, 1.6vh, 48px)` or inside a rule whose selector is a map
  control button (`-control button`: projection, auto-zoom, settings gear; research R3), so later
  edits keep the scale. Today it fails on
  lines such as `font: 13px/1.4` and `font-size: 15px`.

### Implementation for User Story 3

- [X] T022 [US3] In `src/web/assets/app.css`: `html { font-size: clamp(13px, 1.6vh, 48px) }`; turn
  every text size into `rem` (body `font: 1rem/1.4 …`; the 11 px, 12 px, 15 px and 16 px sizes
  become `0.85rem`, `0.92rem`, `1.15rem`, `1.23rem`; the map control buttons keep
  `font: 12px/29px`, research R3); the legend swatch, popup and panel paddings and the
  panel width (`320px` → `24rem`, capped by the existing narrow-screen rule) in `rem`; leave
  MapLibre's own controls alone. T020 and T021 pass.
- [X] T023 [US3] Run quickstart §3 steps 1 and 2 (a 4K screen or a browser window zoomed to emulate
  it), and the settings panel at 1920×1080 and 375 px.
  *Closed by the owner after the visual test, 2026-10-25.*

**Checkpoint**: US2, US1 and US3 work.

---

## Phase 6: User Story 5 - Screens update themselves after a service update (Priority: P2)

**Goal**: a page whose own version differs from the service's reloads once by itself, never in a
loop (FR-012 to FR-014, research R6).

**Independent Test**: quickstart §4.

### Tests for User Story 5 ⚠️ write first, see them fail

- [X] T024 [P] [US5] Node tests in `web/connection.test.js` for
  `shouldReload(pageVersion, serviceVersion, reloadedFor)`, every row of the table in
  data-model.md, plus: `'__LQM_VERSION__'` as page version → false; empty strings → false;
  `undefined`/`null` service version → false.
- [X] T025 [P] [US5] Node tests in `web/connection.test.js` for `pageVersion(doc)`: reads
  `meta[name="lqm-version"]` `content` from a stub `{ querySelector }`; missing meta, empty content
  or the token → `null`.

### Implementation for User Story 5

- [X] T026 [US5] In `src/web/assets/connection.js` export `shouldReload` and `pageVersion` (and
  `RELOAD_KEY = 'lqm.reloaded-for'`), with a doc comment citing feature 018 R6. T024 and T025
  pass.
- [X] T027 [US5] In `src/web/assets/app.js`, at the top of the `hello` branch (before `onHello`):
  inside one `try`/`catch`, when `shouldReload(pageVersion(document), message.version,
  sessionStorage.getItem(RELOAD_KEY))`, store the version under `RELOAD_KEY`, call
  `location.reload()` and return; when session storage throws, do not reload (research R6).
- [X] T028 [US5] Run quickstart §4 steps 1–4; confirm the `reconnect` browser step (same binary)
  still passes with no reload.
  *Done 2026-10-25: a page served by 3.3.0 reloaded itself into a 3.4.0-test service and kept
  its marker across a same-version restart (steps 1–3, scripted over CDP). Step 4, the stale
  cached tab, is covered by the `shouldReload` unit tests only.*

**Checkpoint**: US2, US1, US3 and US5 work.

---

## Phase 7: User Story 4 - Know the software version (Priority: P3)

**Goal**: the bottom of the column shows `v` + the service version (FR-011).

**Independent Test**: quickstart §2 step 2 (last line).

### Tests for User Story 4 ⚠️ write first, see them fail

- [X] T029 [US4] Extend the browser step `column` in `tests/browser/run.mjs`: `#version` is the
  last child of `#column`, its bottom edge is the column's bottom edge minus its padding (±2 px)
  at 1920×1080 with the box at size 20, and its text is `v` + the version from the program's
  `--version` output (or `Cargo.toml`, read by the runner). At 1280×720 with more QSOs than fit:
  `#version` is inside the window and every visible `#recent tbody tr` has its bottom at or above
  `#recent`'s bottom (spec edge case "box taller than the space left").

### Implementation for User Story 4

- [X] T030 [US4] In `src/web/assets/index.html` add `<div id="version" class="version"></div>` as
  the last child of `#column`; in `src/web/assets/app.css` `.version { margin-top: auto;
  font-size: 0.8rem; color: var(--muted) }`; in `src/web/assets/app.js` set its `textContent` to
  `` `v${message.version}` `` on `hello`, or `''` when `version` is missing. T029 passes.

**Checkpoint**: every story works.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [X] T031 [P] Browser docs: list step 14 `column` in `tests/browser/README.md` and in
  `specs/012-page-fixes-browser-tests/contracts/browser-test.md`, pointing to this feature's
  research R9.
- [X] T032 [P] Documentation:
  - **`README.md`**: the layout table row "bottom left" (line ~93) and the "Last QSOs" line
    (~145) describe the left column (status, UTC clock, score, box, version), the strip below
    600 px, and the automatic reload after an update;
  - **`specs/001-live-qso-map/contracts/websocket-protocol.md`**: a note pointing to
    `specs/018-fullscreen-side-column/contracts/websocket-protocol.md` for `hello.version`.
- [X] T033 [P] Entries under `## [Unreleased]` in `CHANGELOG.md` (`docs/releasing.md`): Added (the
  column with the UTC clock and version tag; the automatic reload after an update; `hello.version`,
  protocol unchanged), Changed (text sizes scale with the screen; the bottom-left corner moved to
  the column).
- [X] T034 Run the complete gates: `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`, `node --test web/`, and
  `cargo build --release && node tests/browser/run.mjs`.
- [X] T035 Per LL-007, run a manual rehearsal pipeline on the branch and tick this task with its
  pipeline number once the `column` browser step is green there. Record SC-001 to SC-006 in
  `checklists/verification.md`:
  - SC-001 from T019 (read from 3 m, owner);
  - SC-002 from T006;
  - SC-003 from T020;
  - SC-004 from T028;
  - SC-005 from T028 and the `reconnect` browser step;
  - SC-006 from T013 and T014;
  - FR-014 from the existing `reload-keeps-settings` browser step (the automatic reload is a plain
    `location.reload()`).

---

## Dependencies & Execution Order

- **Phase 1** → **Phase 2** → stories.
- **US2 (Phase 3)** needs only Phase 1; it is the MVP.
- **US1 (Phase 4)** needs US2 (the column to place the clock in).
- **US3 (Phase 5)** needs US1 (it checks the column's size order and sizes).
- **US5 (Phase 6)** needs Phase 2 only; it can run in parallel with US2–US3 (different files except
  `app.js`, where its change is confined to the `hello` branch).
- **US4 (Phase 7)** needs Phase 2 and US2.
- **Polish (Phase 8)** needs every story.

Within each story: tests → see them fail → implementation → manual check.

## Parallel Opportunities

- Phase 2: T002 and T003 together.
- US1: T013 alongside T014; T015 alongside T016.
- US5: T024 and T025 together; the whole phase alongside Phases 3–5 (one developer per side).
- Polish: T031, T032 and T033 together.

```text
# Example: US5 next to US2
Developer A: T006 → T007 → T008 → T009 → T010 → T011 → T012
Developer B: T024 + T025 → T026 → T027 → T028
```

## Implementation Strategy

1. **MVP**: Phases 1–3 (version plumbing, then the column). Shippable: the corner moves into the
   column with today's sizes.
2. Add US1 (clock and size order), then US3 (scaling), checking on a real TV after US3.
3. Add US5 (automatic reload), then US4 (version tag).
4. Polish, gates, rehearsal pipeline (LL-007).
