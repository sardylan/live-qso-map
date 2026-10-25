# Research: Full-Screen Side Column

**Feature**: `018-fullscreen-side-column` | **Date**: 2026-10-25 | **Spec**: [spec.md](spec.md)

The spec has no NEEDS CLARIFICATION items left (three settled on 2026-10-25). This file records the
design decisions the plan needs.

## R1. Page layout: column beside the map (FR-001, FR-003, FR-005)

- **Decision**: the body becomes a two-item flex row: `<aside id="column">` on the left and
  `<main id="stage">` (`position: relative; flex: 1; min-width: 0`) filling the rest. `#stage` holds
  `#map` (`height: 100%`), `#no-webgl`, `#status`, `#basemap-error` and `#legend`, so their absolute
  positions are relative to the map area; `#panel` stays a child of `body`. The column is
  `clamp(280px, 24rem, 45vw)` wide, a flex column itself: status block, score line, last QSOs box
  (`flex: 1; overflow: hidden`), version tag pinned last (`margin-top: auto`). When the box is
  taller than the space left, it drops its oldest rows until every remaining row fits; the version
  tag stays visible. Below 600 px a media query turns the body into a flex column: the column
  becomes a strip on top (auto height, full width), `#recent` gets `display: none`, the stage fills
  the rest. The old `#corner-bl` wrapper, its `bottom` offsets and the `scale-on` body class go away.
- **Rationale**: plain CSS for the layout; only the row fitting needs script. The width is in `rem`
  so the column grows with its text (SC-003): about 415 px at 1080p, 830 px at 4K, 312 px at 720p;
  the 45vw cap keeps the map at least 55% of the window. MapLibre watches its container with a
  `ResizeObserver` (MapLibre 5), so the map redraws when the column or strip changes its size, not
  only on window resize.
- **Alternatives**: keep the elements absolutely positioned and pad the map: the map would still
  draw under the column and auto-zoom would need the padding; rejected by clarification Q1.

## R2. Auto-zoom no longer avoids the moved elements (FR-004)

- **Decision**: remove `#recent`, `#score` and `#station` from `PANELS` in `app.js`. The map
  container is already the area beside the column, so `paddingFor` gets the container's rectangle
  and no column rectangles.
- **Rationale**: those selectors made auto-zoom keep markers away from boxes floating over the map.
  Beside the map they no longer overlap it; keeping them would add a wrong padding on the left.
- **Alternatives**: none worth it; leaving them in would push QSOs right for no reason.

## R3. Text sizes that grow with the screen (FR-008, FR-010, SC-003)

- **Decision**: one root size, `html { font-size: clamp(13px, 1.6vh, 48px) }`, and every text size
  in `app.css` moves from `px` to `rem`. At 1080 px of height the root is about 17 px, at 2160 px
  about 35 px (same share of the height), at 720 px it stops at 13 px (today's size). The column uses
  a fixed ladder: status block `2.4rem` (band and operator) and `2rem` (clock), score `1.6rem`, last
  QSOs box `1rem`, version tag `0.8rem`. Legend, popups, banners and the settings panel use
  `1rem` or their current ratio to 13 px. Swatches, paddings and the column width use `rem` or `vw`
  so they scale together. MapLibre's own controls (zoom, attribution, scale bar) are left as they are,
  and so are the projection and auto-zoom buttons (`.projection-control`, `.toggle-control`), which
  keep `font: 12px/29px`: their height is MapLibre's fixed 29 px control size.
- **Rationale**: tying the root to the viewport height makes SC-003 hold by construction; one
  number to tune on a real TV, with the ladder preserving the order FR-008 asks for. The lower bound
  keeps a 1280×720 laptop as today.
- **Alternatives**:
  - fixed larger `px` sizes: right for one screen, wrong for 4K (SC-003 fails);
  - a viewer setting for text size: out of scope in the spec;
  - `vw` for the root: wide 21:9 screens would get oversized text.

## R4. UTC clock (FR-007, SC-006)

- **Decision**: a pure helper `utcClock(date)` in `station.js` returns `HH:MM:SS UTC` from
  `getUTCHours/Minutes/Seconds`. `app.js` writes it into `#clock` from a `setInterval` of 1 s, started
  once with the page; the first write happens at once so the clock never shows empty. The clock is
  never part of the stale greying.
- **Rationale**: HR-1 (UTC everywhere); the device clock is the only time source the page has and
  the spec accepts it. A 1 s tick keeps the drift under 1 s (SC-006).
- **Alternatives**: aligning the tick to the next full second: at most 1 s of lag either way, not
  worth the extra code.

## R5. How the page knows its own version (FR-011, FR-012, clarification Q3)

- **Decision**: `index.html` carries `<meta name="lqm-version" content="__LQM_VERSION__">`. The `/`
  handler serves `INDEX_HTML` with that token replaced by `env!("CARGO_PKG_VERSION")`, built once in
  a `LazyLock<String>`. `app.js` reads the meta at start-up. The service also adds
  `version: env!("CARGO_PKG_VERSION")` to `hello`.
- **Rationale**: the page then knows the release it was served by, even when the browser shows a
  copy kept from before an update (the case clarification Q3 asked to cover). The other page files
  are already served `no-cache`, so a reload fetches all of them from the new service.
- **Alternatives**:
  - comparing with the version from the first `hello`: misses stale cached pages (rejected in Q3);
  - a build script that writes the version into the HTML: a `build.rs` for one string, more than a
    `replace` at start-up;
  - a separate `/version.json`: one more request and one more route for the same value.

## R6. Reload rule and loop guard (FR-012, FR-013, FR-014)

- **Decision**: a pure helper in `connection.js`:
  `shouldReload(pageVersion, serviceVersion, reloadedFor)` is true only when both versions are
  non-empty strings, they differ, and `reloadedFor !== serviceVersion`. On `hello`, `app.js` calls it
  with `sessionStorage['lqm.reloaded-for']`; when true it stores the service version there and calls
  `location.reload()` before handling the rest of `hello`. Storage errors count as "not reloaded yet"
  only once per page load (the helper's caller keeps an in-memory flag too), so a browser that
  refuses storage still cannot loop.
- **Rationale**: `sessionStorage` lives for the tab across reloads, which is exactly the loop to
  break (a proxy or cache still serving an old page). The template token left unreplaced (a page not
  served by the service, for example the browser tests' fixture) is treated as missing.
- **Alternatives**: `localStorage`: would also stop other tabs from reloading; a counter with a
  time window: more state for no gain.
- Viewer settings already live in `localStorage` (Principle IV), so they survive the reload with no
  extra work (FR-014). The reload flag is not a viewer setting.

## R7. Protocol version unchanged (FR-013)

- **Decision**: `hello.version` is a new field; the protocol stays 4.
- **Rationale**: API-4 asks for a bump only on breaking changes. Older pages ignore unknown fields;
  a new page talking to an older service sees no `version` and never reloads (R6).

## R8. Status block content (FR-002, FR-007, FR-009)

- **Decision**: the status block keeps `#station` (band and operator, text and greying unchanged
  from feature 017, `— · —` when unknown) and adds `#clock` below it. `#score` keeps its text and
  greying (feature 007). `#version` shows `v<hello.version>`, or nothing when `hello` has none.
- **Rationale**: the existing helpers and their tests stay valid; only placement and size change.
  The browser steps that read `#station`, `#score` and `#recent` keep working.

## R9. Tests (Principle I)

- `web/station.test.js`: `utcClock` (midnight, single digits padded, a non-UTC device offset).
- `web/connection.test.js`: `shouldReload` (same, different, missing either side, unreplaced token,
  already reloaded for that version, reloaded for an older version).
- `src/web/protocol.rs` and `tests/ws_protocol.rs`: `hello.version` equals `CARGO_PKG_VERSION`.
- `src/web/mod.rs` test (or `tests/ws_protocol.rs` over HTTP): `GET /` carries the version meta and
  no `__LQM_VERSION__` token.
- `tests/browser/run.mjs`: a new `column` step checks that the map's left edge equals the column's
  right edge at 1920×1080 and at 1280×720, that the column runs the window's full height, the text
  order (status larger than score larger than box), the version tag text, and the strip at 375 px
  wide with `#recent` not shown. LL-007: run before merge.
