# Verification: Contest Score and Page Layout

**Feature**: `007-contest-score-layout` | **Date**: 2026-10-09

## Automated gates (T027)

| Gate | Result |
|------|--------|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test` | pass: 292 tests, 0 failed (1 ignored, as before) |
| `node --test web/` | pass: 131 tests, 0 failed |

## Success criteria

| Criterion | Evidence | Status |
|-----------|----------|--------|
| SC-001: replayed scores show 90/6/10, then 132/7/11, within 2 s | T002 (reference packets), T008 (`a_score_reaches_open_pages_and_pages_opened_later`), T009 (replay) | automated: pass; browser: pending (T020) |
| SC-002: a new page shows the score within 1 s | T008 | automated: pass |
| SC-003: greys out within 10 s of 5 minutes, back within 2 s | T010 (`isStale`), 5 s re-check in `app.js` | automated: pass; browser: pending (T020) |
| SC-004: corners at 1024 and 1920 px, no overlap | T021 (markup guard) | browser: pending (T025) |
| SC-005: `12.345.678 pts · 9.876 QSO · 1.234 mults` on one line at 1024 px | T010 (text) | browser: pending (T025) |
| SC-006: 100 malformed scores change nothing | T008 (`malformed_scores_change_nothing`), T004 (logged) | automated: pass |

## Manual checks

- [ ] T020: quickstart §2 (score line), in a browser
- [ ] T025: quickstart §3 (layout), at 1024 px and 1920 px, globe and flat
