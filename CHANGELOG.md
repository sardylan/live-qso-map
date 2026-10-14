# Changelog

All notable changes to live-qso-map are recorded here, in the
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format, with dates in UTC. Every pull
request adds its entries under "Unreleased"; a release turns them into the version's section,
which becomes the GitHub release notes (`docs/releasing.md`).

## [Unreleased]

## [3.1.0] - 2026-10-14

### Security
- rustls 0.23.46: fixes RUSTSEC-2026-0346 (TLS secrets in `Debug` output), RUSTSEC-2026-0347,
  RUSTSEC-2026-0348 and RUSTSEC-2026-0349, which affect the HTTPS connections to the callbooks and
  the cty.dat download.

### Fixed
- The globe's atmosphere turns with the Earth: its glow no longer stays in the same corner of
  the window when the globe is dragged or auto-zoom moves it. It is now brightest on the edge
  towards the real Sun, and stays a thin rim that never hazes the map.

### Added
- Grayline: the night side is shaded on the flat map and on the globe, with a smooth fade from
  the horizon to full night 12° below it, following the Sun. A "Grayline (day and night)"
  setting, on by default, switches it off.
- Sun position: an optional Sun symbol where the Sun is overhead ("Sun position" setting, off by
  default).
- The release browser test checks the glow after hand moves and auto-zoom, and the grayline on
  both views.
- History file: the QSOs are kept in an ADIF file (`--history-file`, by default
  `qso-history.adi` next to `cty.dat`), read at startup and written back at a graceful stop, so a
  restart no longer empties the map. QSOs that already had a position come back with no new
  lookup; an exported log placed there is completed and saved back. To reset: stop, delete the
  file, start.
- `--enrich-only [FILE]` completes the QSOs in an ADIF file and writes it back, then exits,
  without opening any port.
- ADIF fields `APP_LIVEQSOMAP_ADDED` (the fields the service added to a record) and
  `APP_LIVEQSOMAP_LOOKUP_FAILED` (every online lookup failed; tried again at the next start).

### Changed
- The service keeps every QSO: the history is no longer limited. The page's "Last N QSOs" menu
  goes up to 1000 instead of "All"; a saved "All" becomes 1000.

### Removed
- The `history_size` setting (`--history-size`, `LQM_HISTORY_SIZE`): it is ignored, with a
  warning.

## [3.0.1] - 2026-10-13

### Fixed
- The map page loads again: 3.0.0 didn't serve four of its script modules (`themes.js`,
  `sun.js`, `panel.js`, `legend.js`), so the page stopped before drawing its controls, the
  settings button and the legend.
- A map button that is on (Auto-zoom) has an accent outline, so it can be told apart in every
  theme, Dim included.
- The settings panel is opaque: the legend and the map credit no longer show through it.
- The Legend group no longer offers two groups that select the same bands: with the default
  bands it offers "HF contest" only.

### Added
- Every release is tested in a real browser (Chrome) before it is published: the map page of the
  release build must load without errors and pass a short viewer session.

## [3.0.0] - 2026-10-13

### Fixed
- A station operating from another country with a prefix (`IS0/DL1ABC`, `I/HB9ABC`) is drawn in
  that country, not at its home abroad.
- Positions at 0°/0°, and the `AA00` locator outside Antarctica, are ignored instead of drawn.
- An online position outside the callsign's country is ignored, and the country centre is used.

### Added
- Portable and compound callsigns unknown online are looked up again without their prefix or
  suffix (`IS0ABC/P` → `IS0ABC`).
- The QSO field `APP_LIVEQSOMAP_POS_REJECTED` lists ignored positions and why.
- Light, Dim and Dark themes for the page. They follow the device's preference (the default) or
  the sun at the station, with Dim in civil twilight, or stay fixed.
- An atmosphere around the globe, on by default.
- A settings panel, opened with a gear at the top right, holding every viewer setting.
- The legend, the map and the box can show only some band groups ("HF contest", "WARC", …).

### Changed
- With no `bands` setting, the service considers the six contest HF bands (160, 80, 40, 20, 15
  and 10 m) instead of every band. Set `bands = ["all"]` (or `--bands all`) for every band.
- `--save-config` always writes `bands`.
- WebSocket protocol 4: `hello.band_groups`, and an optional `bands` in `set_limit`.

## [2.0.1] - 2026-10-13

### Fixed
- CI checks out the code over HTTPS through the CircleCI GitHub App; every earlier run had failed
  at checkout over SSH.
- The page tests run on Node 24 in CI (`node --test 'web/*.test.js'`); `node --test web/` loaded
  `web` as a module there and ran no test.
- Re-running the publishing of a release keeps its notes and only replaces files.
- Every tool the pipeline downloads is pinned and checked against its SHA-256.

### Changed
- CI runs the quality gates on pushes to `main`, on pull requests and on release tags, once per
  event; branches without a pull request no longer start a run.
- `main` is protected: changes arrive through pull requests with green gates.
- Publishing secrets live in a CircleCI context that only release tags can use.
- New `audit` gate: vulnerable dependencies, licences and documentation warnings.
- The ARM64 Debian package and container image are tested on ARM64 before a release.
- Releases are cut with cargo-release in two steps (`docs/releasing.md`), and their notes come
  from this file.

### Notes
- 2.0.0 was tagged but never published, because every CI run failed at checkout. 2.0.1 is the
  2.0.0 program plus these pipeline fixes.

## [2.0.0] - 2026-10-11

### Notes
- Tagged, never released: the CI pipeline failed at checkout, so no package, archive or image was
  built. Its changes ship in 2.0.1.

Versions up to 1.0.0 are listed in the
[GitHub history](https://github.com/sardylan/live-qso-map/releases).
