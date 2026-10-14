# Implementation Plan: QSOs Kept in a Local ADIF File

**Branch**: `feature/014-adif-persistence` | **Date**: 2026-10-14 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/014-adif-persistence/spec.md`

## Summary

The service keeps working from memory and gains one file: an ADIF history read at startup and
replaced at a graceful stop.

- **Load (R2, R4, R5, R8):** every file record goes through `normalise`, the identity and the
  resend gate, like a datagram. Records with a saved position are **reused** without a lookup;
  the others are **enriched** like live QSOs on a second, lower-priority queue, so the page and
  the listener are up at once and live QSOs never wait. Records on bands not considered are
  **parked** and written back verbatim.
- **Telling the logger's fields from ours (R3):** each enriched record lists the fields the
  service added in `APP_LIVEQSOMAP_ADDED`. Removing them gives the record as received, so resends
  after a restart match and retries start clean (HR-11).
- **Failed lookups (R7):** when every online source fails (error, timeout) on the logged call,
  the QSO gets `APP_LIVEQSOMAP_LOOKUP_FAILED` and is retried at the next start; rejected positions
  are retried only in enrich-only mode (R6).
- **Save (R9, R10):** at a graceful stop, history + parked + still-pending records are written in
  QSO time order to `<name>.tmp`, synced and renamed over the file. An unreadable file is moved
  aside at startup (R11).
- **Enrich-only (R12, R13):** `--enrich-only [FILE]` loads, completes every record on every band,
  saves and exits, with no listener.
- **No history limit (R14):** `history_size` is removed (accepted and ignored with a warning); the
  page's count menu tops out at 1000 instead of All. Protocol stays 4.
- **Location (R1):** `history_file`, defaulting to `qso-history.adi` next to the country data, which
  puts it in `/var/lib/live-qso-map/` on Debian and in the container with no conffile change.

## Technical Context

**Language/Version**: Rust (edition 2024, latest stable toolchain; see `docs/rust-guidelines.md`
§2); browser ES modules for the page's count menu.

**Primary Dependencies**: none new. Existing: tokio (signals, tasks, `select!`), async-channel
(second queue), clap (`--enrich-only [FILE]`), serde/toml (settings), tracing.

**Storage**: one ADIF file (`history_file`); a temporary `<name>.tmp` during the save; an
`<name>.unreadable-<UTC>` copy only when the file could not be read. Browser `localStorage`
`lqm.limit` gains the value `"1000"` and loses `"all"`.

**Testing**: `cargo test`, plus the existing Node test runner for `web/*.test.js`:

- unit tests: file reading (header, malformed, unreadable detection), writing (layout, order,
  atomic replace, failure leaves the old file), `APP_LIVEQSOMAP_ADDED` round trip, classification
  table (data-model), failed-lookup detection in `resolve_remote`/`enrich`, config (`history_file`
  default, deprecated `history_size`), protocol limits;
- integration tests with injected fake lookup sources and a cty fixture:
  `tests/history_file.rs` (restart round trip, zero lookups on reuse, resend after restart dropped,
  correction after restart merged, QARTest logger id, parked bands, pending at shutdown, unreadable
  moved aside, file deleted while running), `tests/enrich_only.rs` (no ports, completes every band,
  retries rejected and failed, missing file exit 1, interrupt saves);
- `web/connection.test.js`: `"all"` → 1000 and saved back; the new list;
- existing `tests/ws_protocol.rs`, `tests/config_export.rs`, `tests/cli_help.rs` updated.

**Target Platform**: development on any host; release targets as configured in
`.circleci/config.yml` (Principle V).

**Project Type**: single binary

**Performance Goals**:

- startup with a 10 000-QSO file: page served within 5 s (SC-003); reused records need no
  network;
- save of 10 000 QSOs within 5 s (SC-003; about 4 MB, well under a second);
- 50 000 QSOs in memory: snapshots capped at 1000 (SC-007).

**Constraints**:

- HR-11: logger fields are never overwritten; HR-7: restored positions keep precision and source;
  HR-1: UTC timestamps in the header and the moved-aside name; HR-9: dupes stay separate.
- Only graceful stops save (assumption); the file is never touched while the service runs.
- File names valid on Windows (no `:`).

**Scale/Scope**: a contest log — up to tens of thousands of QSOs; one service per file.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First**: every behaviour above has a test listed under Testing, written before its
      code; the file reader has malformed-input, empty, header-only, not-ADIF and
      unreadable cases; the `APP_LIVEQSOMAP_ADDED` strip/restore is a parsing/conversion test.
- [x] **II. Rust Safety & Quality**: no `unsafe`. File and rename errors become
      `Error::HistoryFile { path, message }` in the crate `Error` enum and are logged; no
      `unwrap`/`expect` in runtime paths; fmt and clippy gates unchanged.
- [x] **III. Simplicity & Minimal Dependencies**: no new crate (std `rename`/`sync_all` instead of
      `tempfile`, R10); still one binary; a second queue and a pending map are the minimum to keep
      live QSOs first and lose nothing at stop.
- [x] **Domain rules**: UTC (HR-1); positions restored with saved precision and source, never
      promoted to `logger` (HR-7, R4); dupes not merged (HR-9, gate unchanged); ADIF only,
      `APP_LIVEQSOMAP_` prefix, logger values never overwritten (HR-11, R3); bands untouched.
- [x] **IV. Persistent Viewer Settings**: `lqm.limit` keeps working; `"all"` migrates to `"1000"`
      with a test; unknown values fall back to the default.
- [x] **V. Portable Development**: std file APIs only; `rename` replaces the target on every OS
      (R10); no `cfg` code needed; signals use the existing `wait_for_signal`.
- [x] **Interface rules**: no message changes shape; `null` still accepted; the contract is
      updated (contracts/websocket-protocol.md); contract test in `tests/ws_protocol.rs`.

Post-design re-check (after Phase 1): all items still hold. No violations; Complexity Tracking is
empty.

### Dependency Justification

| Crate | Why needed | Alternatives considered (std / existing dep) |
|-------|------------|----------------------------------------------|
| — | no new crate | `tempfile` rejected for std `rename` + `sync_all` (R10) |

## Project Structure

### Documentation (this feature)

```text
specs/014-adif-persistence/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── history-file.md
│   ├── command-line.md
│   └── websocket-protocol.md
├── checklists/requirements.md
└── tasks.md            (/speckit-tasks)
```

### Source Code (repository root)

```text
src/
├── history_file/          new
│   ├── mod.rs             load(path, mode) -> Loaded; save(path, …); LoadReport
│   ├── read.rs            read + unreadable detection + move aside (R11)
│   ├── write.rs           header, ordering, tmp + sync + rename (R10)
│   └── restore.rs         as-received strip (R3), classify (data-model), reuse (R4)
├── enrich/mod.rs          APP_LIVEQSOMAP_ADDED, LOOKUP_FAILED, worker biased on two queues
├── enrich/resolve.rs      resolve_remote reports failed vs answered sources (R7)
├── ingest/mod.rs          restored events through the gate before the UDP loop; parked records
├── ingest/gate.rs         unbounded
├── history.rs             no capacity; iter_in_order()
├── pipeline.rs            pending map; save on stop; run_enrich_only (R12)
├── config.rs              history_file, deprecated history_size, --enrich-only (R1, R13, R14)
├── config_export.rs       drop history_size, write history_file when set
├── errors.rs              Error::HistoryFile
├── web/protocol.rs        LIMITS, null → 1000
├── web/session.rs         null → 1000
├── web/assets/connection.js   LIMITS, "all" → 1000 migration
└── main.rs                dispatch: service or enrich-only; exit codes
tests/
├── history_file.rs        new
├── enrich_only.rs         new
├── ws_protocol.rs, config_export.rs, cli_help.rs   updated
web/connection.test.js     updated
docs: README.md, CHANGELOG.md, docs/ham-radio.md (HR-7/HR-11 fields, §2 glossary: history file),
      deploy/config.example.toml
```

**Structure Decision**: a new `history_file` module owns everything about the file (read, write,
restore rules); the pipeline only wires it at startup and stop. Enrichment and ingest gain the
small hooks they need (added-field list, failure note, second queue, parked list).

## Complexity Tracking

No violations.
