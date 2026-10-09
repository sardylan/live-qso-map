# Verification: QTH Marker, Auto-Zoom and Map Panels

**Feature**: [spec.md](../spec.md) | **Quickstart**: [quickstart.md](../quickstart.md)

## Automated (2026-10-09)

| Gate | Result |
|------|--------|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass (0 warnings) |
| `cargo test` | 250 passed, 0 failed, 1 ignored (`load_ws`, as before) |
| `node --test web/` | 104 passed, 0 failed |
| `python3 -m unittest tools/test_entity_extents.py` | 15 passed |
| Entity extents coverage | 333 of 346 cty.dat entities mapped (96.2 %), 13 listed as unmapped with reasons |
| `boundingCap` with 2,000 points (plan goal < 5 ms) | median 3.5 ms (Node 26, development Mac) |
| JSON errors (FR-027) | 404, 405 with `Allow`, and `/ws` without an upgrade: all JSON, checked in `tests/ws_protocol.rs` |

## Manual (browser), run by the project owner

Confirmed by the owner in a browser on 2026-10-09, after the fixes T052 (globe zoom), T053
(button follows the view) and T054 (a switch turns auto-zoom off; page files revalidate).

- [x] T013 / SC-001: the house is shown within 2 s, with black lines from it and the house popup (§2)
- [x] T028 / SC-002, SC-003, SC-004: auto-zoom keeps everything clear of the panels at 1024×600
  and 3840×2160; entity framing for IS, I, W, VK; manual moves (drag, zoom, rotate) switch it
  off; popups pause it; reconnection (§3)
- [x] SC-005: at most one refit per second at about 10 QSOs/s with 2,000 shown (§3 step 5)
- [x] T035 / SC-006: the globe is kept while everything fits, flat otherwise, the globe again
  afterwards; the preference is never overwritten; a drag keeps the automatic flat view (§4)
- [x] T042 / SC-007: the box shows the five newest, late QSOs stay out, corrections and removals
  are followed, rows open popups, the hidden state is remembered, text is shown literally (§5)
- [x] T045 / SC-008: the scale is within 5 % over 10 zoom levels, and hidden on the globe (§6)
- [x] §7: the JSON errors reproduced with `curl`
