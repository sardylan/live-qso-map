# Data Model: Automatic Theme, Atmosphere, Settings Panel and Legend Selection

## Viewer settings (browser storage, constitution IV)

| Key | Values | Default | Invalid or missing value |
|-----|--------|---------|--------------------------|
| `lqm.theme` | `auto-device`, `auto-sun`, `light`, `dim`, `dark` | `auto-device` | default |
| `lqm.atmosphere` | `on`, `off` | `on` | default |
| `lqm.legend` | `all`, or a JSON array of group names | `all` | `all`; unknown names are dropped, and an array left empty becomes `all` |
| existing: `lqm.limit`, `lqm.recent`, `lqm.recent.size`, `lqm.recent.order` | unchanged | unchanged | unchanged |

The panel's open state is **not** stored: the panel always starts closed.

## Theme

| Theme | When |
|-------|------|
| `light` | fixed; `auto-device` with no dark preference; `auto-sun` with the elevation ≥ 0° |
| `dim` | fixed; `auto-sun` with the elevation in [−6°, 0°) |
| `dark` | fixed; `auto-device` with a dark preference; `auto-sun` with the elevation < −6° |

**Palette** (one per theme, `themes.js`):
- **Interface colours:** `bg`, `panel`, `text`, `muted`, `border`, `accent`, `banner`;
- **Globe colours:** `space`, and the sky's `sky`, `horizon` and `fog`;
- **Base map:** `basemapStyle` (the OpenFreeMap style name), and `dimOpacity` for
  operator-configured maps (0, 0.35 or 0.55).

**Rule:** every text/background pair has a contrast ratio of at least 4.5:1.

## Sun elevation (transient)

**Inputs:**
- **Time:** UTC now.
- **Position:** the house marker's, from `currentQth()` (the configured station, else the newest
  QSO's station).

**Output:** degrees, or `null` without a position. It is recomputed every 60 s, and when the
position changes.

## Settings panel (transient UI state, `panel.js`)

| Field | Meaning |
|-------|---------|
| `open` | whether the panel is shown |
| `lastActivity` | the time of the last input inside the panel |

| Event | Effect |
|-------|--------|
| gear | toggle `open` |
| close button, Escape | `open = false` |
| tick (every 15 s) | close when `open` and `now − lastActivity ≥ 5 min` |
| input inside the panel | `lastActivity = now` |

On closing, focus returns to the gear (FR-008).

## Band groups (server → page, `hello.band_groups`)

| Field | Type | Rule |
|-------|------|------|
| `name` | string | one of the server's group names (`lf-mf`, `hf`, `hf-contest`, `warc`, `vhf`, `uhf`, `shf`, `ehf`) |
| `label` | string | display name, for example "HF contest" |
| `bands` | array of strings | the group's **considered** bands only, in frequency order (HR-2); never empty |

The list is in the server's group order, and groups with no considered band are left out.

## Legend choice → bands shown

- **`all`:** every considered band (`hello.bands`), plus QSOs without a `BAND`.
- **A set of group names:** the union of their `bands`, in frequency order. QSOs without a
  `BAND` are hidden.

The page sends the result as `set_limit.bands` (`null` for `all`), and filters what it draws by
the same set.

## Server band setting (configuration)

| Setting | Considered bands |
|---------|------------------|
| absent or empty | `hf-contest`: 160, 80, 40, 20, 15 and 10 m (**new default**) |
| `["all"]` / `--bands all` | every band (the old default) |
| anything else | as in feature 003 |
| `all` together with other entries | configuration error |

The export always writes `bands`, as the effective list or `["all"]`.
