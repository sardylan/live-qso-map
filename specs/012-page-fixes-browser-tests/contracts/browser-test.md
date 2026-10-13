# Contract: the browser test runner

## Command

```
node tests/browser/run.mjs [--binary PATH] [--chrome PATH] [--out DIR] [--headed]
```

| Option | Default |
|--------|---------|
| `--binary` | `target/release/live-qso-map` (`.exe` on Windows) |
| `--chrome` | `$CHROME_PATH`, otherwise the first one found among the usual Chrome and Chromium locations: Linux `google-chrome`, `chromium`, `chromium-browser` on `PATH`; macOS `/Applications/Google Chrome.app/…`, `/Applications/Chromium.app/…`; Windows `%ProgramFiles%\Google\Chrome\Application\chrome.exe` and `%LOCALAPPDATA%\…` |
| `--out` | `target/browser-test` |
| `--headed` | off: headless. With it, the window is shown (local debugging) |

**Needs:** Node 22 or newer, and nothing else to install (no `npm install`).

**Exit codes:**
- `0`: every step passed;
- `1`: a step failed;
- `2`: bad usage, the program or Chrome wasn't found, or Chrome couldn't start.

**Output:** a one-line summary for each step on stdout, then the result. Then
`<out>/report.json`, `<out>/console.log` and `<out>/NN-<step>.png` (data-model.md).

## Steps

| # | Step | Phase | Checks | Spec |
|---|------|-------|--------|------|
| 01 | `load` | A | no fatal event; `maplibregl` defined; WebGL available; zoom, Globe/Flat, Auto-zoom and settings buttons present; legend lists 160m 80m 40m 20m 15m 10m in that order | US1 1, US3 |
| 02 | `qsos` | A | QSOs sent over UDP on 40m, 20m, 30m and 2m: 40m and 20m appear in the box and as markers within 5 s; 30m and 2m never do | US3 1 |
| 03 | `panel-open` | A | gear click: the panel is shown, with groups Map, Last QSOs box, Appearance and Legend; focus on its first control; `aria-expanded="true"`; Legend offers exactly "HF contest (6 bands)" | US3 2, US4 3 |
| 04 | `theme-light`, `theme-dim`, `theme-dark` | A | each choice sets `data-theme`, and the status line names it ("Dim: chosen"); Auto-zoom stays on; the box keeps its rows | US3 3 |
| 05 | `panel-escape` | A | Escape: the panel is hidden; focus on the gear; `aria-expanded="false"` | US3 2 |
| 06 | `reload-keeps-settings` | A | after a reload, the theme choice (Dark) is kept | US3 6 |
| 07 | `device-dark` | A | with `lqm.theme` = `auto-device` and dark emulated, a reload shows `data-theme="dark"` | US3 4 |
| 08 | `reconnect` | A→B | the program stops: the reconnection banner shows within 5 s. It restarts with `--bands hf`: the banner goes away within 15 s, and a 40m and a 30m QSO sent then appear (the history starts empty after a restart) | US3 7 |
| 09 | `legend-group` | B | Legend offers HF (10), HF contest (6), WARC (3). Ticking WARC: the legend lists only 30m 17m 12m, and the box and markers show only the 30m QSO, and the 40m one is hidden. Unticking it: everything is shown again | US3 5 |

A failed step stops the run. The steps after it are reported as not run. The program and Chrome
are always stopped.
