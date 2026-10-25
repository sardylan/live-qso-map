# Verification: Full-Screen Side Column

**Date**: 2026-10-25 | **Branch**: `feature/018-fullscreen-side-column`

## Automated gates (T034)

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`: 472 passed, 0 failed
- [x] `node --test web/`: 254 passed, 0 failed
- [x] `node tests/browser/run.mjs` (local, macOS, release build): 14 steps pass, `14 column`
  included, at the default 813 px viewport and at CI's 757 px
- [x] Manual rehearsal pipeline green with the `column` step (LL-007, T035): rehearsal #133 failed
  at `10 atmosphere` (the glow reading, fixed in `784e710`); rehearsal #134 green, every job

## Success criteria

- [x] **SC-001**: band, operator, UTC time and score readable from 3 m: closed by the owner after
  the visual test on 2026-10-25 (T019), which also led to the score on three lines.
- [x] **SC-002**: nothing in the column overlaps the map from 1280×720 to 3840×2160: T006 (`14
  column`, 1920×1080 and 1280×720 rectangles) and T020 (3840×2160).
- [x] **SC-003**: column text the same share of the height at 1080p and 4K, within 10 %: T020
  (`#station` height share and `#column` width share, 1920×1080 against 3840×2160).
- [x] **SC-004**: open pages show the new version within 60 s of the service being back: T028 (a
  3.3.0 page reloaded into a 3.4.0-test service, scripted over CDP; well inside 60 s).
- [x] **SC-005**: no reload on a same-version reconnect: T028 (marker kept across a same-version
  restart) and the `08 reconnect` browser step.
- [x] **SC-006**: the clock within 1 s of the device's UTC clock: T013 (`utcClock`) and T014 (the
  clock reads the page's UTC hour and ticks within 3 s).

## Changes after the visual test (owner's requests, 2026-10-25)

- Score, QSOs and multipliers on three lines.
- The box fills the column (its size is computed by the page; the "Size" setting is gone).
- Settings in a modal dialog, from a gear at the column's foot; newest QSO on top (the "Newest"
  setting is gone); monospace callsigns.
- Map controls at the top right.
- Feature 016's auto-zoom fit check moved from 1024×768 to 1280×800 (owner's choice).
