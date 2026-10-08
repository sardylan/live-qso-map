# Verification: Live QSO Map

**Date**: 2026-10-09
**Build**: branch `001-live-qso-map`
**Host**: macOS (Apple Silicon), Rust 1.98.1, Node 26.11.1

## Quality gates

| Gate | Result |
|------|--------|
| `cargo fmt --check` | ✅ pass |
| `cargo clippy --all-targets -- -D warnings` | ✅ pass |
| `cargo test` | ✅ 198 passed, 0 failed, 1 ignored (the `load_ws` load test, run separately below) |
| `node --test web/` | ✅ 28 passed |

## Success criteria

| ID | Criterion | Evidence | Status |
|----|-----------|----------|--------|
| SC-001 | Marker on every page within 2 s, 95% | `tests/ws_protocol.rs` (fan-out within 2 s). `tests/load_ws.rs`, 60 s on macOS: p95 1.26 ms, max 2.04 ms | ✅ on macOS |
| SC-002 | Lookup timeout + 2 s, even with a dead provider | `src/enrich/resolve.rs` tests: a 5 s source is dropped at 200 ms; sources run in parallel | ✅ |
| SC-003 | New page shows the last 20 within 3 s | `tests/ws_protocol.rs::pages_choose_their_own_window` | ✅ |
| SC-004 | Raspberry Pi 4: 10 QSO/s, 50 pages, no loss | `tests/load_ws.rs` on macOS: 50 pages × 600 QSOs, all received | ⚠️ passes on macOS; **run on a Pi 4 still pending** |
| SC-005 | Resends ignored, corrections in place | `tests/resend_gate.rs` (single lookup, rev 2 in place, newest wins with 4 workers) | ✅ |
| SC-006 | Malformed datagrams never stop the service | `tests/malformed_input.rs` (11-file corpus, mixed and alone), ADIF fuzz (10,000 inputs), QARTest mutation fuzz (5,000 inputs) | ✅ |
| SC-007 | Country placement by ham-radio entity | `tests/lookup_resolution.rs` (IS0 → Sardinia, IT9 → Sicily, I1 → Italy). Real cty.dat (346 entities) end to end | ✅ |
| SC-008 | Deploy in under 10 minutes on each target | Dockerfile, systemd unit, `Cross.toml` and README written | ⚠️ **not built here**: Docker daemon not running, `cross` not installed |
| SC-009 | QARTest reference capture → 5 entries, no warnings | `tests/qartest_replay.rs`, and a replay against the real binary with the real cty.dat: history 5, no warnings | ✅ |

## Quickstart walkthrough

| Section | Done | Notes |
|---------|------|-------|
| §1 Quality gates | ✅ | see above |
| §2 Single ADIF QSO | ✅ | real binary; identical resend ignored |
| §3 Multi-record datagram | ✅ | 2 QSOs, malformed third record warned and skipped |
| §4 QARTest replay | ✅ | test and real binary |
| §5 Online lookups | ⚠️ | cty.dat path verified with the real file. **QRZ.com/HamQTH/QRZCQ untested against live services: no credentials** (T063) |
| §6 Last-N selector | ⚠️ | protocol verified by tests; `scripts/send_burst.sh 150` gives history 150. **Browser UI not checked by a person** (T045) |
| §7 Load | ⚠️ | macOS only; Pi 4 pending |
| §8 Deployment | ⚠️ | artifacts written, not built (T091) |

## Deferred owner checks

The feature was closed on 2026-10-09, with these checks deferred to the project owner by
decision. They are tracked here and in `tasks.md` (marked DEFERRED).

1. **T045**: open the map in two browsers and run quickstart §2/§3. Check the markers, colours,
   great-circle lines, popups, legend, the selector, and that the page reconnects after a service
   restart.
2. **T063**: with real QRZ.com/HamQTH (and QRZCQ, if available) credentials, check the popup
   source and precision. Note any differences from the reconstructed fixtures in
   `tests/fixtures/README.md`.
3. **T091**: `cross build --release` for the three targets, and `docker build .`.
4. **SC-004**: `cargo test --release --test load_ws -- --ignored --nocapture` on a Raspberry Pi 4.
