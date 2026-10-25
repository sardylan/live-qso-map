# Implementation Plan: Full-Screen Side Column

**Branch**: `feature/018-fullscreen-side-column` | **Date**: 2026-10-25 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/018-fullscreen-side-column/spec.md`

## Summary

Move the station status line, the score line and the last QSOs box out of the bottom-left corner
into a permanent full-height column beside the map, with a UTC clock under the status and the
software version at the bottom. The layout is plain CSS flex; below 600 px the column becomes a
strip above the map without the box (R1). Text sizes move to `rem` on a root size tied to the
viewport height, so a 4K TV gets the same proportions as a 1080p one (R3). The service adds
`version` to `hello` and writes the same version into `index.html`; a page whose own version
differs from the service's reloads itself once, guarded per tab against loops (R5, R6). Protocol
stays 4 (R7).

## Technical Context

**Language/Version**: Rust (edition 2024, latest stable toolchain; see `docs/rust-guidelines.md` §2)
and browser ES modules
**Primary Dependencies**: none new (axum, serde as today; MapLibre 5.24 on the page as today)
**Storage**: none on the service; one per-tab `sessionStorage` flag on the page (R6)
**Testing**: `cargo test`, `node --test web/`, release browser steps (`tests/browser/run.mjs`)
**Target Platform**: development on any host; release targets as configured in `.circleci/config.yml` (Principle V)
**Project Type**: single binary
**Performance Goals**: clock within 1 s of the device clock (SC-006); updated pages reload within
60 s of the service being back (SC-004), already met by the reconnect backoff's 30 s cap
**Constraints**: protocol version unchanged; no reload loop; no new setting (spec, out of scope)
**Scale/Scope**: tens of pages, screens from 375 px phones to 3840×2160 TVs

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First**: tests first for `utcClock` and `shouldReload` (node), for `hello.version`
      and the served version meta (Rust unit and `tests/ws_protocol.rs`), and a browser step for
      the layout (R9).
- [x] **II. Rust Safety & Quality**: no `unsafe`; the version comes from `env!`, the page string is
      built once in a `LazyLock` with `str::replace`, nothing fallible at runtime; fmt and clippy
      gates unchanged.
- [x] **III. Simplicity & Minimal Dependencies**: no new crate, no build script, no new route (R5);
      layout in CSS only; no text-size setting.
- [x] **Domain rules** (`docs/ham-radio.md` §1): the clock is UTC, `HH:MM:SS UTC` (HR-1); band and
      operator as in feature 017.
- [x] **IV. Persistent Viewer Settings**: no new viewer setting; existing settings live in
      `localStorage` and survive the automatic reload (FR-014). The reload flag is tab state, not a
      setting; without session storage the page never reloads by itself (R6).
- [x] **V. Portable Development, Pipeline-Built Releases**: no platform-specific code.
- [x] **Interface rules** (`docs/http-api.md`): `version` is a snake_case string field added to a
      JSON text frame; additive, so protocol stays 4 (R7); contract test planned.

Post-design re-check: all still pass; no Complexity Tracking entry.

### Dependency Justification

| Crate | Why needed | Alternatives considered (std / existing dep) |
|-------|------------|----------------------------------------------|
| — | no new crate | — |

## Project Structure

### Documentation (this feature)

```text
specs/018-fullscreen-side-column/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/websocket-protocol.md
└── tasks.md            (next: /speckit-tasks)
```

### Source Code (repository root)

```text
src/web/protocol.rs        Hello gains `version: &'static str` (CARGO_PKG_VERSION)
src/web/mod.rs             `/` serves INDEX_HTML with __LQM_VERSION__ replaced, built once (LazyLock)
src/web/assets/index.html  version meta; <aside id="column">: #station, #clock, #score, #recent, #version;
                           <main id="stage"> around the map and its overlays; #corner-bl removed
src/web/assets/app.css     flex layout, column and <600 px strip, rem sizes on a clamp() root,
                           corner-bl and scale-on offsets removed
src/web/assets/app.js      clock tick, version tag, reload check on hello; #recent/#score/#station
                           out of PANELS; whole-row fitting of the box; scale-on toggle removed
src/web/assets/recent.js   rowsThatFit(rowHeights, available)
src/web/assets/station.js  utcClock(date)
src/web/assets/connection.js  shouldReload(pageVersion, serviceVersion, reloadedFor)
web/station.test.js        utcClock tests
web/connection.test.js     shouldReload and pageVersion tests
web/recent.test.js         rowsThatFit tests
tests/ws_protocol.rs       hello.version and the served meta
tests/browser/run.mjs      new `column` step (LL-007: rehearse before merge)
tests/browser/README.md    step 14 listed
CHANGELOG.md               Unreleased: Added (column, clock, version tag, auto reload), Changed (sizes)
README.md                  page description, if it mentions the bottom-left corner
```

**Structure Decision**: existing single-binary layout; no new module on either side. The two new
helpers join the modules that own their topic (`station.js` for the status block, `connection.js`
for the connection handshake).

## Complexity Tracking

None.
