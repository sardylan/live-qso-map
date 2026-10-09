# Quickstart: DXCC Entity and Date in the Last-QSO Box

## 1. Gates

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && node --test web/
```

## 2. The box (User Story 1)

```bash
cargo run --release -- --station-callsign IS0MZK --station-locator JM49NH
```

1. Send QSOs with `K1ABC`, `JA1XYZ`, `IS0ABC`, `IT9ABC` and `EA8/DL1ABC`. **Expected**: the rows
   show date `YYYY-MM-DD`, time `HH:MMZ` and `K · United States`, `JA · Japan`,
   `IS · Sardinia`, `IT9 · Sicily` and `EA8 · Canary Islands`. No row shows a mode.
2. Send a QSO with an entity whose name is long, such as `FT5ZQ`
   (`FT/z · Amsterdam & St. Paul Is.`).
   **Expected**: the name is cut with "…", and hovering shows the full name.
3. Send a QSO with a callsign that matches nothing, for example `QQ9QQ`. **Expected**: its entity
   cell shows `—`.
4. Send one QSO at 23:59 and the next at 00:01 the following day. **Expected**: the two dates
   differ.
5. At a 1024 px wide window, check that the box stays narrower than 45 % of the window and that
   no row wraps (SC-003).
6. Restart with `--cty-path /nonexistent` and no network access. **Expected**: every entity is
   `—`, and everything else works (SC-002).

## 3. Popups (User Story 2)

1. Click the `JA1XYZ` marker. **Expected**: the row shows `JA · Japan` next to call, band, mode,
   time and position source.
2. Correct that QSO to another callsign (QARTest `contactreplace`) while its popup is open.
   **Expected**: the entity in the popup updates.
