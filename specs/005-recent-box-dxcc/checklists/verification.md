# Verification: DXCC Entity and Date in the Last-QSO Box

**Feature**: [spec.md](../spec.md) | **Quickstart**: [quickstart.md](../quickstart.md)

## Automated (2026-10-09)

| Gate / criterion | Result |
|------------------|--------|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass (0 warnings) |
| `cargo test` | 257 passed, 0 failed, 1 ignored (`load_ws`, as before) |
| `node --test web/` | 116 passed, 0 failed |
| SC-001 (T002) | 30 callsigns (ordinary, portable, prefixed, exact-call, WAE): the entity matches the cty.dat lookup for all of them |
| SC-004 (T004) | each upsert, with its entity, reaches the page within the 2 s timeout |

## Headless browser check (2026-10-09, 1024×700, real cty.dat)

- The rows show call, band, `2026-10-09`, `15:0xZ` and `TL · Central African R…`,
  `K · United States`, `EA8 · Canary Islands`, `IT9 · Sicily` and `—` (QQ9QQ). No mode is shown.
- SC-003: the box is about 41 % of the window width, and no row wraps.
- Finding, fixed: the wider box covered the start of the attribution line at 1024 px (feature
  004, FR-019). The box now sits above the attribution row: 30 px from the bottom, or 56 px with
  the scale bar.

## Manual (browser), to be run by the project owner

- [ ] T011 / SC-002, SC-003: quickstart §2, including the run with no country data and the
  1024 px width
- [ ] T013: quickstart §3, popups with the entity, including a correction while a popup is open
