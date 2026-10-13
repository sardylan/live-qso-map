# Data Model: Page fixes and real-browser release tests

There is no new server data. This feature adds the browser test run's records, a palette key,
and a narrower rule for offered band groups.

## Browser test run

One run of `tests/browser/run.mjs` against one build.

| Field | Type | Notes |
|-------|------|-------|
| `binary` | path | the program under test |
| `chrome` | path | the browser used |
| `started` | UTC time | HR-1 |
| `steps` | list of Step | in run order |
| `result` | `pass` / `fail` | `fail` if any step failed |
| `events` | list of Event | console, exceptions, network failures |

**Step**

| Field | Type | Notes |
|-------|------|-------|
| `name` | string | e.g. `load`, `panel-open`, `theme-dark`, `legend-group`, `reconnect` |
| `status` | `pass` / `fail` | |
| `ms` | number | how long it took |
| `message` | string | on failure: what was expected, what was seen |
| `screenshot` | file name | `NN-<name>.png`, one per tested state |

**Event**

| Field | Type | Notes |
|-------|------|-------|
| `at` | UTC time | |
| `kind` | `console` / `exception` / `request-failed` / `http-error` | |
| `level` | string | console level |
| `text` | string | message, or URL and status |
| `fatal` | boolean | per research R5 |

A step fails as soon as a fatal event arrives while it runs. Fatal events between steps fail
the next step.

**Lifecycle:**
1. Start the program (phase A).
2. Start Chrome.
3. Run the steps for scenarios 1–4 and 6.
4. Restart the program (phase B).
5. Run the steps for scenarios 7 and 5.
6. Write the report.
7. Stop both. They are also stopped on failure and on Ctrl-C.

## Palette (feature 011, extended)

| Key | CSS variable | Rule |
|-----|--------------|------|
| `panelSolid` (new) | `--panel-solid` | opaque (`#rrggbb`); the same colour as `panel` without its alpha |

**Tests on every palette:**
- `accent` against `highlight` ≥ 3:1;
- `accent` against `bg` ≥ 3:1.

These are WCAG 1.4.11 for the on-state outline. The existing 4.5:1 text pairs are unchanged.

## Offered band groups (feature 011, narrowed)

`considered_groups(filter)`:
1. Takes every group in `GROUPS` order, with only its considered bands. A group with none is
   dropped (unchanged).
2. **New:** among groups whose considered band lists are equal, keeps only the most specific.
   That is the group with the fewest bands in its full definition, and on a tie the first in
   `GROUPS` order.
3. Returns the kept groups in `GROUPS` order.

| Considered bands | Offered groups |
|------------------|----------------|
| default (`hf-contest`) | `hf-contest` |
| `hf` | `hf`, `hf-contest`, `warc` |
| `40m,20m` | `hf-contest` |
| `30m` | `warc` |
| every band (`all`) | every group (no two are equal) |
