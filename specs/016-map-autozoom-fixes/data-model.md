# Data Model: Map Auto-Zoom Fixes

## Operator setting: `map.globe_border_deg`

| Field | Type | Default | Validation | Sources (highest first) |
|-------|------|---------|------------|-------------------------|
| `globe_border_deg` | number (degrees of arc) | 10 | finite, 0 ≤ value ≤ 45 | `--globe-border-deg`, `LQM_GLOBE_BORDER_DEG`, `[map] globe_border_deg`, default |

- Invalid value (unparseable, NaN or infinite, negative, above 45): the service refuses to start
  with a config error naming `map.globe_border_deg`.
- Resolved into `MapConfig.globe_border_deg: f64`. `--save-config` writes it, and the example
  configuration lists it commented out.
- This is not a viewer setting: nothing about it is stored in the browser (Principle IV).

## Page frame (pure values, `autozoom-geo.js`)

| Value | Meaning |
|-------|---------|
| `cap` | `{ center: [lat, lon], radiusDeg }`, the smallest cap holding the QTH and every framed marker (unchanged) |
| `globe zoom` | the highest zoom ≤ `MAX_ZOOM` at which the cap fits the free area, or `null` when the cap edge is not at least `globe_border_deg` in front of the horizon while the horizon is in view (R2, R3) |
| `flat frame` | `{ center: [lon, lat] = QTH, zoom }` when the QTH is known (R4); otherwise the feature 004 `flatBounds` |

### Projection decision per refit (unchanged state machine, feature 004)

```text
globe preference, auto-zoom on:
  globe zoom != null  → show globe at globe zoom, centred on cap.center
  globe zoom == null  → show flat (automatic), flat frame
flat preference, or auto-zoom off: no automatic change (feature 004 FR-017)
```

Removed: `fitsGlobe(cap, 5)`, `GLOBE_MARGIN_DEG` and the `LIMB_SHARE` rule.
