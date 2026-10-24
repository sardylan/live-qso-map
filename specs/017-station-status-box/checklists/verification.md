# Verification: Station Status Box

**Date**: 2026-10-24 | **Branch**: `feature/017-station-status-box`

## Automated gates (T024)

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`: 470 passed (with feature 016), 0 failed
- [x] `node --test web/`: 245 passed (with feature 016), 0 failed
- [x] `node tests/browser/run.mjs` (local, macOS, release build, on top of feature 016): 13 steps
  pass, `13 station` included
- [ ] Manual rehearsal pipeline green with the `station` step (LL-007, T025)

## Success criteria

- [x] **SC-001**: each new QSO's band and operator on open pages within 2 s: T006 (`a_page_gets_the_station_status_of_the_latest_qso`, 2 s timeouts) and T021 (`12 station`, 5 s step limit, passed in 1.6 s).
- [x] **SC-002**: a page opened later shows the same status: T006 (second page).
- [x] **SC-003**: status after a restart equals the latest saved QSO: T008 (`the_station_status_comes_back_after_a_restart`) and the manual check in T014.
- [x] **SC-004**: greyed 15 minutes after the latest QSO, within 10 s: T016 (`isStale` at 900 s / 900.001 s), the 5 s tick in `app.js`, T020 and T021 (a 20-minute-old QSO greys at once).
- [x] **SC-005**: band and operator readable from the page alone: T014 (in-app browser, light and dark, 1024 px; no overlap with the score line or the box).
