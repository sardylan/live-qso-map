# Quickstart: QTH Marker, Auto-Zoom and Map Panels

These steps validate the feature end to end. They reuse `tools/udp_capture.py`'s sender helpers
and `scripts/send_burst.sh` from feature 001.

## 1. Gates

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && node --test web/
```

## 2. QTH and lines (User Story 1, SC-001)

```bash
cargo run --release -- --station-callsign IS0MZK --station-locator JM49NH
```

1. Open `http://localhost:8080/` with no QSOs. **Expected**: a house at JM49NH within 2 s, the map
   framing Sardinia, and no lines.
2. Send three QSOs on 20m, 40m and 15m to stations in the USA, Japan and Brazil. **Expected**:
   three band-coloured markers, each joined to the house by a black great-circle line. The markers
   stay clickable even next to the house.
3. Click the house. **Expected**: `IS0MZK`, `JM49NH` and `config · locator6`, as plain text.
4. Restart without `--station-locator`, then send a QSO that has `MY_GRIDSQUARE`. **Expected**: no
   house until that QSO arrives, then the house at its station square.

## 3. Auto-zoom (User Story 2, SC-002–SC-005)

1. With auto-zoom on, send QSOs on every continent, one at a time. **Expected**: after each one,
   every marker and the house are visible, none under the legend, the box, the controls or the
   attribution.
2. Repeat at window sizes 1024×600 and 3840×2160 (DevTools device toolbar).
3. Drag the map once. **Expected**: the auto-zoom button shows off, and further QSOs don't move
   the map. Reload: it is still off. Click the button: the map refits at once.
4. Click a marker. Send QSOs for 20 s. **Expected**: the map doesn't move. Close the popup:
   exactly one refit. Open a popup and leave it alone for 30 s: it closes, then the map refits once.
5. Set the limit to All and run `scripts/send_burst.sh 2000`. Then send a second burst with
   100 new QSOs paced at about 10 QSOs/s. `send_burst.sh` gains an optional delay argument for
   this (`scripts/send_burst.sh 100 127.0.0.1 12060 0.1`) and a start offset, so the new calls
   differ from the first burst.
   **Expected**: at most one refit per second (the browser's performance panel shows camera
   animations about 1 s apart), and the map stays responsive.
6. Restart with `--station-callsign` set to each of `I0XXX`, `W1XX` and `VK2XX` and their locators,
   and with no QSOs. **Expected**: Italy, the contiguous USA or Australia fills at least half of the
   free width or height (SC-003). Start with no callsign and only a locator: the map frames 500 km
   around the house.

## 4. Globe and flat (User Story 3, SC-006)

1. Choose Globe and turn auto-zoom on, from the IS0 QTH. Send QSOs from Europe only. **Expected**:
   the map stays a globe.
2. Add JA and W6 QSOs. **Expected**: the map switches to flat and shows everything.
3. Remove them (correct them onto an ignored band with feature 003, or lower the limit).
   **Expected**: the map returns to the globe.
4. Turn auto-zoom off and add the DX QSOs back. **Expected**: the map stays a globe.
5. Reload after step 2. **Expected**: the projection button still offers "Flat", because the saved
   preference is Globe.

## 5. Last-QSO box (User Story 4, SC-007)

1. Send seven QSOs. **Expected**: the box shows the five newest, newest first, each with call,
   band, mode/submode and UTC time.
2. Send a QSO with an older time. **Expected**: it is on the map but not in the box.
3. Correct one QSO in the box (QARTest `contactreplace`). **Expected**: its row updates. Remove one
   (correct it onto an ignored band). **Expected**: the next one moves up.
4. Click a row. **Expected**: that marker's popup opens, and the map moves only if the marker was
   hidden. Auto-zoom stays on.
5. Hide the box and reload. **Expected**: it is still hidden.
6. Send a QSO whose call is `<b>X</b>`. **Expected**: it is shown literally.

## 6. Scale (User Story 5, SC-008)

1. Switch to Flat. **Expected**: a metric scale at the bottom.
2. Zoom from the world to a city through 10 levels. At each level, measure two points that are the
   scale's width apart at the map centre with an online great-circle calculator. **Expected**: the
   labelled distance is within 5 %.
3. Switch to Globe. **Expected**: the scale disappears.

## 7. JSON errors (FR-027)

```bash
curl -si localhost:8080/nope             # 404, application/json, {"error":"not_found",...}
curl -si -X POST localhost:8080/healthz  # 405, Allow: GET,HEAD, {"error":"method_not_allowed",...}
curl -si localhost:8080/ws               # 4xx, {"error":"websocket_upgrade_required",...}
```

## 8. Regenerating the entity extents (maintainers only)

```bash
python3 tools/entity_extents.py --cty cty.dat --mapping tools/entity_extents.toml \
  --out src/web/entity_extents.json   # downloads Natural Earth v5.1.2 once into .cache/
```

The script prints the cty.dat entities that have no mapping. Commit the JSON together with the
mapping.
