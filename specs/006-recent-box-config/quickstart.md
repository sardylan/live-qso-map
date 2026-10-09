# Quickstart: Configurable Last-QSO Box

## 1. Gates

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && node --test web/
```

## 2. Operator columns (User Story 1, SC-001, SC-002)

1. Start with no column settings and send QSOs. **Expected**: the header reads Call, Band, UTC,
   CQ, Country; each row shows those values aligned (call in bold, country in grey), with no
   lines.
2. Run with `--recent-columns country,call,mode`. **Expected**: three columns in that order.
3. Run with `--recent-header false`. **Expected**: no header row.
4. Run with `--recent-columns call,freq`, then `--recent-columns call,call`, then
   `--recent-columns " "`. **Expected**: each run exits with an `invalid setting
   map.recent_columns` message that names the problem.
5. Send QSOs with `VY1ABC`, `R8HAA`, `UA9AA` and `K1ABC`. **Expected**: CQ shows 1, 18, 17 and 5
   (SC-007).

## 3. Viewer size and order (User Story 2, SC-003, SC-004, SC-006)

1. First visit (clear site data). **Expected**: Map 20, Box 5, Newest bottom, and the newest row
   last.
2. Set Map 10 and Box 20, then send 25 QSOs. **Expected**: 10 markers and 20 box rows. The 10
   rows that are not on the map are faded, with the "Not on the map" tooltip, and do not respond
   to clicks.
3. Switch Newest to top. **Expected**: the newest row is first, at once. Send a QSO: it appears at
   the top.
4. Close the browser and reopen the page. **Expected**: Map 10, Box 20 and Newest top are kept.
5. At 1024 px and 1920 px, check that each column lines up on every row (SC-005).
