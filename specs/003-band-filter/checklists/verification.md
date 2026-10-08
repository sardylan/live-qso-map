# Verification: Band Filter

**Date**: 2026-10-09
**Build**: branch `003-band-filter` (rebased onto `main` with features 001 and 002)
**Host**: macOS (Apple Silicon), Rust 1.98.1, Node 26.11.1

## Quality gates

| Gate | Result |
|------|--------|
| `cargo fmt --check` | ✅ pass |
| `cargo clippy --all-targets -- -D warnings` | ✅ pass |
| `cargo test` | ✅ 237 passed, 0 failed, 1 ignored (`load_ws`) |
| `node --test web/` | ✅ 48 passed |

## Success criteria

| ID | Criterion | Evidence | Status |
|----|-----------|----------|--------|
| SC-001 | 0% of ignored or unknown-band QSOs reach history, pages or lookups | `tests/band_filter.rs::mixed_stream_respects_the_filter` (1,000 QSOs over every band, filter `hf`) and `only_considered_bands_reach_the_map` | ✅ |
| SC-002 | 100% of considered QSOs processed as without the filter | same test: the history holds exactly the considered QSOs, and the lookup count equals their number | ✅ |
| SC-003 | Selection examples resolve exactly; invalid selections stop startup | `band_filter` and `config` unit tests; real binary: `hf-dx` and `warc,-warc` exit with `invalid setting bands: …` | ✅ |
| SC-004 | Correction onto an ignored band removes the QSO on every page within 2 s | `tests/band_filter.rs::correction_onto_ignored_band_removes_the_qso` (QARTest 20m → 60m → 20m; unknown id gives no message) | ✅ |
| SC-005 | Every band list in frequency order | group-table test (each group strictly increasing by band rank), `hello` legend test, startup-log test | ✅ |

## Real-binary checks

| Check | Result |
|-------|--------|
| `--bands hf-contest`: 20m QSO (band from `FREQ`) kept, 2m QSO dropped with a debug log | ✅ history 1 |
| `--bands hf,2m,-60m`: startup log | ✅ `bands considered: 160m 80m 40m 30m 20m 17m 15m 12m 10m 2m` |
| `--bands hf-dx`, `--bands warc,-warc` | ✅ exit 1 with the documented messages |

## Pending manual check (needs a browser)

- **T028**: open a marker's popup, then send the QARTest correction onto 60m (quickstart §4). The
  popup closes and the marker disappears. The decision (`popupToClose`) is unit-tested; the glue
  needs a visual check.
