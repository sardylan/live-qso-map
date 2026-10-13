# Quickstart: verify feature 011

## 1. Automated checks (any host)

```bash
cargo test --locked                    # band default and "all", band_groups, filtered snapshot, export round-trip
node --test 'web/*.test.js'            # themes (contrast), sun elevation, panel timer, legend choice, map-state filter
```

## 2. Server default bands

```bash
cargo run -- --station-callsign IS0MZK --station-locator JM49NH
# log: "bands considered: 160m, 80m, 40m, 20m, 15m, 10m"; the page's legend shows exactly six bands
cargo run -- --bands all --save-config -   # prints bands = ["all"]
cargo run -- --save-config -               # prints bands = ["160m", "80m", "40m", "20m", "15m", "10m"]
```

## 3. Theme (browser)

1. Open the page. In the settings panel → Appearance, the choice is "Automatic (device)".
2. **Device preference:** switch the OS (or the browser devtools' `prefers-color-scheme`) between
   light and dark. Within seconds, the page, the base map (`liberty` → `dark`), the panel, the
   legend, the box and the score follow. The view and the markers don't move.
3. **Sun at station:** choose "Automatic (sun at station)". The status line shows the sun's
   elevation at the house marker. In devtools, fake the time (or set a station position where it
   is night) and check Light, then Dim (0° to −6°), then Dark.
4. **Fixed themes:** choose Dim, reload, and check that Dim is kept.
5. **Operator base map:** start with `--tile-url … --tile-attribution …`. In Dark, the map is
   dimmed; the markers aren't.

## 4. Atmosphere

1. On the globe, the glow along the limb is visible.
2. Untick "Atmosphere on the globe": the glow is gone. Reload: it stays off.
3. Switch to flat: no atmosphere. In Dark, the sky colours are night colours.

## 5. Settings panel

1. On load, no setting is visible; there's one gear, top right.
2. Open with the gear, by click and by keyboard (Tab, Enter). Focus moves into the panel. Close
   with ✕, Escape and the gear; focus returns to the gear.
3. Leave it open for 5 minutes without touching it: it closes by itself.
4. Narrow the window under 600 px: the panel takes the full width.
5. The existing settings ("last N", box) are in the panel, with their saved values.

## 6. Band groups

1. Start with `--bands hf` and send QSOs on 40 m and 30 m (`scripts/send_burst.sh`, or
   `tools/udp_capture.py` replay).
2. **Choose "HF contest" in Legend:** the legend shows six bands, and the 30 m QSO leaves the map
   and the box.
3. **Choose "All considered bands":** it comes back.
4. **Reload:** the choice is kept. With "Last 10" and many 30 m QSOs, ten non-30 m QSOs are still
   shown.
