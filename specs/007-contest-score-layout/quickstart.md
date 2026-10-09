# Quickstart: Contest Score and Page Layout

## 1. Gates

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && node --test web/
```

## 2. Score line (User Story 1)

1. Start the service and open the page. **Expected**: the bottom-left line reads `Score —`.
2. Replay the reference capture (`specs/001-live-qso-map/reference/qartest-16.9.1-cqww-ssb.txt`)
   by sending each packet body to the service's UDP port, for example
   `printf '%s' '<packet body>' | nc -u -w0 127.0.0.1 <udp port>`. **Expected**:
   - after packet 4, the line reads `90 pts · 6 QSO · 10 mults`;
   - after packet 10, it reads `132 pts · 7 QSO · 11 mults`;
   - each change shows within 2 s (SC-001).
3. Open a second tab. **Expected**: it shows `132 pts · 7 QSO · 11 mults` at once (SC-002).
4. Send a score message with `<score>12345678</score>`, a QSO total of `9876` and multipliers
   summing to `1234`, with the window 1024 px wide. **Expected**: the line reads
   `12.345.678 pts · 9.876 QSO · 1.234 mults` on one line (SC-005).
5. Send 100 malformed score messages, for example a missing `<score>` or `<score>abc</score>`, then
   a QSO. **Expected**:
   - the line keeps the last good score;
   - the log shows one warning per message;
   - the QSO still reaches the map (SC-006).
6. Stop sending for 5 minutes. **Expected**:
   - within 10 s of the 5-minute mark, the line turns grey;
   - hovering shows `Last updated HH:MMZ` in UTC;
   - the next score message restores the normal look within 2 s (SC-003).
7. Restart the service with the page open. **Expected**: after the reconnect, the line reads
   `Score —` until the next score message.

## 3. Layout (User Story 2)

Run each check at 1024 px and at 1920 px wide, on the globe and on the flat map (SC-004).

1. **Top left**: zoom, Globe/Flat and Auto-zoom, in that order, and nothing else.
2. **Top right**: one panel with:
   - "Map: last N QSOs";
   - the "Last QSOs" checkbox;
   - "Box: last N";
   - "Newest".
3. **Bottom right**: the legend, above the attribution.
4. **Bottom left**:
   - with the box shown, the score line sits directly above the box;
   - untick "Last QSOs", and the score line drops to the box's place;
   - on the flat map, the scale bar is below both and nothing overlaps.
5. Untick "Last QSOs", then close and reopen the browser. **Expected**: the box is still hidden
   (Principle IV). A viewer who had hidden the box before this feature still has it hidden.
6. With auto-zoom on, send QSOs near the bottom-left corner of the frame. **Expected**: the markers
   stay clear of the score line and the box (FR-013).
