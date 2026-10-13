# Research: Automatic Theme, Atmosphere, Settings Panel and Legend Selection

Feature: `011-theme-settings-panel` · Spec: [spec.md](spec.md) · Date: 2026-10-13

**Facts checked on 2026-10-13:**
- **OpenFreeMap** serves the styles `liberty` (the default base map), `bright`, `positron`, `fiord`
  and `dark` at `https://tiles.openfreemap.org/styles/<name>`. Each answered HTTP 200.
- **MapLibre GL JS 5.24.0**, the pinned version, has `map.setSky()` with `sky-color`,
  `horizon-color`, `fog-color`, `sky-horizon-blend`, `horizon-fog-blend` and `atmosphere-blend`.
  The globe's atmosphere comes from `atmosphere-blend`.
- **The page today:**
  - viewer settings sit in a fixed top-right box: "Map: last N", and the box's shown, size and
    order;
  - globe/flat and auto-zoom are map buttons, top left;
  - saved keys: `lqm.limit`, `lqm.projection`, `lqm.autozoom`, `lqm.recent`, `lqm.recent.size`,
    `lqm.recent.order`.
- **Protocol** (version 3):
  - the page sends `set_limit {limit}`;
  - the session answers with a `snapshot` of the newest `limit` QSOs, then forwards every
    `upsert`/`remove`;
  - `hello.bands` lists the considered bands with their colours.
- **Band setting** (feature 003):
  - `parse_setting` returns `None` (every band) when the setting is empty;
  - the named groups are `lf-mf`, `hf`, `hf-contest`, `warc`, `vhf`, `uhf`, `shf`, `ehf`;
  - QSOs on bands outside the setting are dropped before the history;
  - the configuration export (feature 008) omits `bands` when every band is considered.

## R1. Themes and how the page applies them

- **Decision**: three palettes, `light`, `dim` and `dark`, defined in a new pure module,
  `src/web/assets/themes.js`, as CSS custom-property values (background, panel, text, muted text,
  border, accent, the colours of the space behind the globe and of the atmosphere).
  - **Applying:** the page sets `data-theme` on `<html>` and writes the palette's variables;
    `app.css` uses only `var(--…)`.
  - **Testing:** a `node --test` file checks every palette's text/background pairs against the
    4.5:1 contrast ratio (FR-002, SC-002), with the WCAG relative-luminance formula.
- **Rationale**: one source for the colours, testable without a browser (Principle I). Contrast
  becomes a test, not a review comment.
- **Alternatives**:
  - **Palettes only in `app.css`** under `[data-theme]` selectors: the contrast check would need a
    CSS parser.
  - **A CSS framework:** a new dependency, which Principle III rejects.

## R2. Choosing the theme

- **Decision**: a pure function, `resolveTheme(choice, { prefersDark, sunElevation })`, returns
  `light`, `dim` or `dark`:

  | Choice (`lqm.theme`) | Result |
  |----------------------|--------|
  | `auto-device` (default) | `dark` if the device prefers dark, else `light` |
  | `auto-sun` | `light` if the elevation is ≥ 0°, `dim` if it is in [−6°, 0°), `dark` if below −6°. With no station position (elevation `null`), it acts as `auto-device` |
  | `light`, `dim`, `dark` | that theme |

  **Inputs:**
  - **The device preference** comes from `matchMedia('(prefers-color-scheme: dark)')`, whose
    `change` event fires when it changes.
  - **The sun elevation** is recomputed every 60 seconds, and when the house-marker position
    changes (FR-003).
- **Rationale**: every input is explicit, so every row of the table is a unit test. Re-checking
  every 60 seconds meets FR-003 at negligible cost.
- **Alternatives**:
  - **The Ambient Light Sensor API:** experimental and off by default (FR-004).
  - **Precomputed sunrise and sunset times:** they break in polar day and night (spec edge
    case). The elevation always works.

## R3. Sun elevation

- **Decision**: a new pure module, `src/web/assets/sun.js`, computes
  `solarElevation(date, lat, lon)` in degrees with the NOAA Solar Calculator equations (Julian
  century, declination, equation of time, hour angle), without atmospheric refraction.
  - **Accuracy:** about ±0.5°, enough for the 0° and −6° thresholds.
  - **Position:** the house-marker position from the existing `currentQth()` (clarification:
    the configured position, else the newest QSO's station position).
  - **Time:** UTC throughout (HR-1).
- **Rationale**: about 40 lines, no dependency (Principle III). Known values can be tested (for
  example, Cagliari at the equinox).
- **Alternatives**: SunCalc (a library). A new dependency for one function.

## R4. Base map per theme

- **Decision**: it depends on which base map the server sends.
  - **The default style** (`hello.basemap.url` equals the default OpenFreeMap `liberty` URL):
    - Light uses `liberty`;
    - Dim uses `fiord`, a softer dark;
    - Dark uses `dark`.

    The page swaps the style with `map.setStyle(url, { diff: false })`. On `style.load`, it
    re-adds the QSO, QTH and great-circle sources and layers, and re-applies the sky.
  - **Any operator-configured map** (another style URL, or raster tiles): the style stays, and
    the page inserts a `background` layer of the palette's "dim" colour, under the QSO layers.
    Its opacity is 0 in Light, 0.35 in Dim and 0.55 in Dark (spec edge case).
- **Rationale**: a real dark style reads better than a darkened light one. The dimming layer
  works for any map, needs no knowledge of the style's layers, and never dims the markers.
- **Alternatives**:
  - **A CSS `filter` on the canvas:** it would also dim the QSO markers and colours, which must
    stay readable.
  - **Rewriting every layer's paint properties:** fragile, and specific to each style.
- **Risk**: re-adding layers on `style.load` touches the map code from features 002 and 004. The
  tests for the pure parts stay unchanged, and quickstart §3 checks it by hand.

## R5. Atmosphere

- **Decision**: `map.setSky()`, using the palette's `sky-color`, `horizon-color` and `fog-color`.
  `atmosphere-blend` is 1 when the atmosphere is on and the projection is the globe, otherwise 0.
  - **Setting:** `lqm.atmosphere`, `on` (the default) or `off` (FR-010).
  - **Space behind the globe:** the map container's CSS background (`--space`), dark in every
    theme, but deeper in Dark.
  - **When re-applied:** on projection change, theme change and `style.load`.
- **Rationale**: the built-in renderer is cheap, and it already knows the globe's limb.
- **Alternatives**: a custom glow layer. More code, for the same result.

## R6. Settings panel

- **Decision**:
  - **The button:** a MapLibre control (one gear button) in the top-right corner, where the
    settings box used to be.
  - **The panel:** a `<aside id="panel" role="dialog" aria-modal="false">` sliding in from the
    right, 320 px wide, or the full width under 600 px (FR-005).
  - **Groups:** `<fieldset>` elements with `<legend>` headings: Map, Last QSOs box, Appearance,
    Legend (FR-006).
  - **Closing** (FR-007): the close button, Escape, the gear, or 5 minutes without input
    (pointer, key or focus inside the panel). It always starts closed.
  - **Accessibility** (FR-008): the gear has `aria-expanded` and `aria-controls`; on opening,
    focus moves to the first control; on closing, it returns to the gear.
  - **Logic:** the open/close and timer logic sits in a pure module,
    `src/web/assets/panel.js`, with an injected clock, so it can be tested with `node --test`.
- **Rationale**: the spec chose a drawer over a modal. Its logic is small enough to keep pure and
  tested, like the rest of the page's state.
- **Alternatives**:
  - **`<dialog>` with `showModal()`:** modal, and the spec rejects hiding the map.
  - **A third-party drawer component:** a dependency (Principle III).

## R7. Band groups on the page

- **Decision**: `hello` gains `band_groups`, a list of `{ "name": "hf-contest", "label": "HF
  contest", "bands": ["160m", …] }` entries, in the order of the server's `GROUPS`.
  - **Contents:** each group keeps only its **considered** bands, in frequency order (HR-2).
    Groups left empty are omitted (FR-012).
  - **Labels:** they live next to `GROUPS` in `src/band_filter.rs`, as one table.
- **Rationale**: the server decides which bands are considered, so it sends ready-to-use groups.
  The page doesn't duplicate the group table (one source of truth).
- **Alternatives**: a copy of the group table in the page. It could drift from the server's.

## R8. Showing only the chosen groups

- **Decision**:
  - **Server:** `set_limit` gains an optional `bands`, a list of band names; when it is absent
    or `null`, every band is shown. The session remembers it, and builds each `snapshot` from the
    newest `limit` QSOs **on those bands** (`History::latest_matching`). `upsert` and `remove`
    still reach every page unchanged.
  - **Page:** it filters what it draws (map and recent box) by the same bands. A correction that
    moves a QSO to a hidden band therefore disappears at once (FR-013).
  - **Protocol version:** 3 → 4.
  - **QSOs without a `BAND`:** shown only with "All considered bands".
- **Rationale**: "last N" counts only the shown QSOs (clarification). Only the server can give
  the newest N **on the chosen bands**, because the page holds only the window it asked for.
  Filtering live updates on the page too avoids per-page work in the broadcast path.
- **Alternatives**:
  - **Page-only filtering of a larger window:** it can't guarantee N shown QSOs.
  - **Server-side filtering of the live updates as well:** more session state in the hot path,
    and the page has to filter anyway, to handle corrections.

## R9. Saved choices

- **Decision**: three new keys, read with the existing `localStore()` helpers. Each read falls
  back to its default on any error or unknown value (constitution IV).

  | Key | Values | Default |
  |-----|--------|---------|
  | `lqm.theme` | `auto-device`, `auto-sun`, `light`, `dim`, `dark` | `auto-device` |
  | `lqm.atmosphere` | `on`, `off` | `on` |
  | `lqm.legend` | `all`, or a JSON array of group names | `all` |

  A legend array is filtered against `hello.band_groups`. If nothing valid remains, it is `all`
  (FR-014).
- **Rationale**: the same pattern as features 002–006.

## R10. Server default bands

- **Decision**:
  - **`parse_setting`:**
    - an empty setting now means the `hf-contest` group (FR-015);
    - a single `all` entry means every band (`None`, FR-016);
    - `all` combined with other entries is a configuration error that names the setting.
  - **Command line and environment:** `--bands all` / `LQM_BANDS=all`, per feature 003's
    precedence.
  - **Export** (FR-017): it always writes `bands`, as the effective list, or `["all"]`. A reload
    then gives the same set. The test that expects "no `bands` key" changes.
  - **Example configuration:** `deploy/config.example.toml` explains the default and `all`.
- **Rationale**: it keeps the explicit way back to every band, and the export round-trip that
  feature 008 promised.
- **Alternatives**: an empty list `bands = []` meaning "all". It reads like "no bands", and
  `--bands ""` is awkward on the command line.

## R11. Test strategy

- **Rust:** unit tests for the `parse_setting` default and `all`, `hello.band_groups`, the
  session's band-filtered snapshot, `History::latest_matching`, and the export round-trip. The
  `ws_protocol` integration test gets protocol version 4, `band_groups`, and `set_limit` with
  `bands`.
- **Page:**
  - `node --test` for the new pure modules (`themes.js`, `sun.js`, `panel.js`, `legend.js`) and
    for the extended `map-state.js` (the band filter, and "last N" over shown QSOs);
  - the DOM wiring in `app.js` is checked by hand in quickstart §3–§6, as features 002–007 did.
