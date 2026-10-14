# Research: QSOs Kept in a Local ADIF File

Feature 014. Each decision states what was chosen, why, and what was rejected. Every "NEEDS
CLARIFICATION" of the Technical Context is resolved here.

## R1 — Where the history file lives, and its setting

**Decision**: a new setting `history_file` (`--history-file`, `LQM_HISTORY_FILE`, TOML
top-level `history_file`). When it is not set, the file is `qso-history.adi` **in the folder of the
country data** (`country_data.path`), after that path has been resolved (program folder on Windows
and macOS, feature 008). A relative `history_file` that is set explicitly is resolved like
`country_data.path`.

**Rationale**:

- Debian sets `country_data.path = "/var/lib/live-qso-map/cty.dat"` in its conffile and runs under
  `ProtectSystem=strict` with `StateDirectory=live-qso-map`: the only writable folder is
  `/var/lib/live-qso-map`. Deriving the default from the country data puts the file there on
  existing installs, without a conffile change.
- The container's entry point passes `--cty-path /var/lib/live-qso-map/cty.dat` (the volume), so
  the file lands in the volume too.
- On Windows and macOS `./cty.dat` is resolved to the program's folder, so the file lands next to
  the program — the rule of FR-002.

**Alternatives considered**:

- A fixed default `./qso-history.adi`: on Debian the working directory is `/`, read-only under
  `ProtectSystem=strict`; the save would fail on every stop.
- A new `[history]` table: one setting doesn't need a table; `history_file` sits with the other
  top-level service settings.

## R2 — Loading: every record is a QSO event, through the gate, behind live QSOs

**Decision**: at startup the file is parsed with the existing ADIF parser (`parse_datagram`, which
already skips the header and reports malformed records). Each record becomes a `QsoEvent` exactly
like a datagram's record: `normalise` (CALL, QSO_DATE, TIME_ON, band from FREQ), identity
(`APP_LIVEQSOMAP_LOGGER_ID` as logger id when present, R5), then the **resend gate**, in file order,
before the UDP loop starts. The events the gate forwards go on a **restore queue** served by **one
dedicated restore worker**, spawned in addition to the `workers` live workers. Live workers read
only the live queue, so a live QSO never waits for a restored lookup, even one in flight; the
restore worker reads only the restore queue. In enrich-only mode there is no live traffic, so
`workers` restore workers serve the restore queue.

**Rationale**:

- Same checks as UDP (FR-005): one code path for identity, merge, normalisation.
- The gate must know the restored QSOs, so that a live resend after the restart is dropped and a
  live correction gets the next revision (US1 scenario 5). Passing them through the gate before the
  UDP loop starts is synchronous and fast (no lookups), so the gate stays a plain struct owned by the
  ingest task.
- Live QSOs never wait behind restored lookups (FR-008): restored events have their own worker, so
  even a slow or hanging restored lookup cannot hold a live worker.
- Restored records that need no lookup (R4) cost microseconds, so the map refills in well under a
  second for 10 000 QSOs (SC-003).

**Alternatives considered**:

- Feeding restored events into the live queue: a contest QSO would wait behind every pending lookup
  of the file.
- Two queues read by the same workers with a biased `select!`: a worker already busy with a
  restored lookup cannot take a live QSO, so with the default single worker a live QSO waits up to
  the lookup timeout.
- A shared gate behind a mutex, filled by a separate loader task: needs locking on every datagram for
  a one-off startup step.

## R3 — Telling the logger's fields from the fields the service added

**Decision**: when the service enriches a record, it records the names of the fields it added in
one new field, `APP_LIVEQSOMAP_ADDED` (comma-separated, in name order). It is computed in `enrich`
as the difference between the field names before and after enrichment. On load, removing the
listed fields (and `APP_LIVEQSOMAP_ADDED` itself) gives back the record **as received**.

**Rationale**:

- Lookups merge candidate fields into the record with `set_if_absent` (`QTH`, `NAME`, `GRIDSQUARE`,
  `COUNTRY`…), and the station step fills `MY_GRIDSQUARE`. Once saved, those fields look exactly like
  the logger's. Without the list:
  - the resend gate's fingerprint of a restored record differs from the logger's resend of the same
    QSO, so every resend after a restart would count as a correction and trigger new lookups;
  - a retry (R6, R7) could not tell which fields to drop, and would keep a stale position or
    locator while claiming a new one.
- HR-11 stays checkable: everything not listed came from the logger or the loaded file and is never
  touched.
- A file never written by the service (an external log) has no `APP_LIVEQSOMAP_ADDED`: every field
  counts as the logger's, which is exactly right.

**Alternatives considered**:

- Saving the gate fingerprint: `DefaultHasher` is not stable across Rust releases, so a fingerprint
  saved by one build may not match the next.
- Saving two copies of each record (as received, and enriched): doubles the file and breaks "one
  record per QSO" for anyone opening the file in a logger.
- Treating every `APP_LIVEQSOMAP_*` field as derived and everything else as the logger's: wrong for
  `QTH`, `GRIDSQUARE`, `NAME` and the other merged fields.

## R4 — Restoring a QSO without a lookup

**Decision**: a restored record is **reused** (no lookup) when all of these hold:

1. it carries `APP_LIVEQSOMAP_POS_SOURCE` and `APP_LIVEQSOMAP_POS_PRECISION` with known values, and
   `LAT`/`LON` that parse;
2. it carries no `APP_LIVEQSOMAP_LOOKUP_FAILED` (R7);
3. in enrich-only mode, it carries no `APP_LIVEQSOMAP_POS_REJECTED` (R6).

A reused record keeps every field as saved. Its remote position is `LAT`/`LON` with the saved
precision and source (so a `cty` point stays country precision, HR-7). Its station position and its
entity are computed again, offline: the station from the record as received (R3) and the current
configuration (`apply_station`), the entity from the current country data. Every other record goes
through `enrich` from the record as received, exactly like a live QSO.

**Rationale**:

- SC-002: zero lookups for QSOs that already had a position.
- The position is restored to the precision of ADIF `XDDD MM.MMM` (1/60 000 degree, about 2 m),
  which is what was saved; SC-001's "same positions" is judged at that resolution.
- The entity and station position are cheap and must follow the current country data and
  configuration, like any QSO.
- A record with `LAT`/`LON` but no `APP_LIVEQSOMAP_POS_SOURCE` (an external log) is the logger's
  position: `enrich` already handles it (`logger_position`), with no lookup.

**Alternatives considered**: re-running `enrich` on the saved record with lookups disabled — it
would read the derived `LAT`/`LON` as the logger's (source `logger`, precision `coordinates`),
which breaks HR-7.

## R5 — Identity of restored QARTest QSOs

**Decision**: when a restored record has `APP_LIVEQSOMAP_LOGGER_ID`, it is passed as the event's
logger id. The identity is then `Logger { station_call, logger_id }`, as when the QSO came from
QARTest.

**Rationale**: QARTest corrections carry the logger id; after a restart they must find the restored
QSO (spec edge case "Logger identities"). The field is already written by the QARTest parser and is
saved with the record.

**Alternatives considered**: none; the composite identity would split every corrected QARTest QSO
in two after a restart.

## R6 — Rejected positions

**Decision**: normal run — a record with `APP_LIVEQSOMAP_POS_REJECTED` and a saved position is
reused (R4), with its note. Enrich-only mode — the record is taken back to as received (R3), and
`enrich` runs on it: a new accepted position fills the fields, a new rejection writes a new note,
and no rejection leaves no note.

**Rationale**: clarification Q of 2026-10-14 (FR-028). Taking the record back to as received drops
the old note and the old fallback at once, so nothing stale survives the retry.

## R7 — Telling a failed lookup from "not found"

**Decision**: `resolve_remote` also reports which online sources **failed** (error, timeout,
task failure) as opposed to those that **answered** (`Ok(Some)` or `Ok(None)`). After the
logged-call round, when at least one online source is configured and **none answered**, `enrich`
writes `APP_LIVEQSOMAP_LOOKUP_FAILED` = `source:kind` entries, comma-separated in the configured
order (`qrz:timeout,hamqth:error`), with `kind` one of `error`, `timeout`. The country fallback is
still used as today. At the next start, in both modes, a record with this note is taken back to as
received and enriched again; a successful lookup leaves no note (FR-029).

**Rationale**:

- "Not found" is an answer and must not be retried forever; a network error, a timeout, a quota or
  a login failure is not an answer.
- The provider client already turns quota and credential problems into `Err(Error::Provider)`, so
  "error" covers them; the timeout is the `tokio::time::timeout` around each source.
- Only the logged-call round decides: when every online source failed on the logged call, the
  base-call round is not run (`known_online` is false but there was no answer), because it would
  fail the same way and double the wait.

**Alternatives considered**: noting partial failures (one provider down, another answered "not
found") — the answer from the working provider is authoritative enough, and retrying would spend
quota on every start.

## R8 — Records on bands the service does not consider

**Decision**: normal run — records whose band is not considered (or with no band, when a filter is
active) are **parked**: kept verbatim, keyed by their history id, never queued, never sent to the
page. They still pass through the resend gate, so the gate knows their identity. At save, a parked
record is written back unchanged, unless the history now holds a QSO with the same id (a live
correction moved it onto a considered band), in which case the history's version wins.
Enrich-only mode — the band filter is not applied at all (FR-026).

**Rationale**: clarification of 2026-10-14. Parking costs no lookup and no memory beyond the record.

**Alternatives considered**: putting them in the history with a "hidden" flag — every session query
(`latest_matching`) and the health count would need to skip them.

## R9 — Records waiting at shutdown

**Decision**: a **pending map** (`id → (rev, record as received or as loaded)`) holds every event
handed to the workers, live or restored, until a worker publishes (or drops as stale) its result. At
save, a pending entry is written as it was received or loaded when its id is not in the history,
**or when its revision is higher than the history's** (a correction still waiting at the stop): a restored
record not yet re-enriched is written back exactly as loaded; a live record not yet enriched is
written as received, and the next start enriches it.

**Rationale**: spec edge case "Stop while lookups are still running" — nothing loaded is ever lost
by stopping early, and enrich-only mode interrupted with Ctrl+C saves what it has (FR-019).

**Alternatives considered**: waiting for the queues to drain before saving — a stop could take
minutes with thousands of pending lookups (FR-015).

## R10 — Writing the file safely

**Decision**: the save writes the whole file to a temporary file in the same folder
(`<name>.tmp`), calls `sync_all`, then renames it over the target (`std::fs::rename`). Layout:

- a header line comment, then `<ADIF_VER:5>3.1.4`, `<PROGRAMID:12>live-qso-map`,
  `<PROGRAMVERSION:n>…`, `<CREATED_TIMESTAMP:15>YYYYMMDD HHMMSS` (UTC, HR-1), `<EOH>`;
- one record per line (`to_adif_string`), in QSO time order (history order, merged with parked and
  pending records by `QSO_DATE` + `TIME_ON`).

On any error the temporary file is removed, the error is logged, and the previous file stays.

**Rationale**:

- FR-014/SC-006: a rename within one folder is atomic on Linux and macOS; on Windows
  `std::fs::rename` replaces the target (`MoveFileExW` with `MOVEFILE_REPLACE_EXISTING`), and the
  previous file is intact until it succeeds. No new crate.
- Writing 10 000 records of about 400 bytes is about 4 MB: well under a second (FR-015, SC-003).

**Alternatives considered**: the `tempfile` crate (`persist`) — std does the same in a few lines;
syncing the folder too — needs `cfg(unix)` code for a power-loss case the spec doesn't promise to
cover (only graceful stops save).

## R11 — An unreadable file at startup

**Decision**: the file is **unreadable** when reading it fails (`std::fs::read` error) or when it
is non-empty and contains no `<EOH>` and no `<EOR>` at all (not ADIF). Normal run: it is renamed to
`<name>.unreadable-YYYYMMDDTHHMMSSZ` (UTC; no `:` so the name is valid on Windows), a warning names
the reason and the new name, and the service starts empty. If the rename fails too, startup fails
with both reasons (FR-011). Enrich-only mode: missing or unreadable → error, exit code 1, nothing
written (FR-018). A file with some malformed records is loaded, skipping them (FR-010).

**Rationale**: clarification of 2026-10-14.

## R12 — Graceful stop and enrich-only completion

**Decision**:

- **Service**: `run` waits for Ctrl+C or SIGTERM as today, stops the tasks (2 s, then aborts), then
  saves (R10) and returns. The UDP and HTTP listeners are closed before the save, so the saved state
  is final.
- **Enrich-only**: no sockets are bound. The loader fills the restore queue; the program waits until
  the queue is empty and the pending map is empty, or until Ctrl+C/SIGTERM; then it stops the
  workers, saves and exits. A missing or unreadable file exits with code 1 before any lookup.

**Rationale**: systemd allows 90 s and `docker stop` 10 s by default; the save takes well under a
second (R10).

## R13 — `--enrich-only [FILE]` on the command line

**Decision**: a clap option `--enrich-only [FILE]` (`num_args = 0..=1`, type
`Option<Option<PathBuf>>`). With a path, that file is completed in place; without one, the
configured history file (R1). It conflicts with `--save-config`. It needs the same configuration as
the service (credentials, timeout, workers, station); the bind address and ports are ignored.

**Rationale**: clarification of 2026-10-14 (FR-016).

## R14 — No history limit; the page shows at most 1000

**Decision**:

- `History::new` loses its capacity (the history is never trimmed). The resend gate becomes
  unbounded too (it was sized like the history).
- `history_size` is removed: the TOML key, `LQM_HISTORY_SIZE` and `--history-size` are still
  **accepted and ignored with a warning** ("history_size is no longer used: the history is not
  limited; remove it"). The configuration export no longer writes it.
- The protocol's `LIMITS` become `10, 20, 50, 100, 200, 500, 1000`; `hello.limits` lists them.
  A `set_limit` with `limit: null` (an older page asking for every QSO) is answered as `1000`.
- The page's `LIMITS` become the same list; `loadLimit` maps a saved `"all"` to `1000` and saves
  `1000` back (FR-025, constitution IV).
- The protocol version stays **4**: no message changes shape; `null` is still accepted.

**Rationale**:

- A config with `history_size` must not stop the service from starting after an upgrade (the TOML
  tables use `deny_unknown_fields`); a warning is the "clear message" of FR-023.
- `latest_matching` walks the ordered history from the newest; at 50 000 QSOs with a 1000 cap a
  snapshot is still fast (SC-007).
- API-4 requires a version bump only for breaking changes; an old page keeps working (its "All"
  shows 1000).

**Alternatives considered**: rejecting `history_size` with an error — breaks startup for every
Debian install that set it; bumping the protocol to 5 — nothing breaks.

## R15 — What is not saved

The contest score, tombstones, revisions and page settings are not saved. A restored QSO starts at
revision 1 (or the gate's revision for duplicates within the file); the gate numbers later
corrections from there, which is all ordering needs.
