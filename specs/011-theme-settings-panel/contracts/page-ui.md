# Contract: Page UI (feature 011)

## Layout

| Element | Where | Notes |
|---------|-------|-------|
| Quick map buttons: zoom, globe/flat, auto-zoom | top left | unchanged |
| Settings button (gear, `aria-label="Settings"`, `aria-expanded`, `aria-controls="panel"`) | top right | replaces the old settings box |
| Legend | as today | lists only the chosen bands |
| Recent box, score | bottom left | as today, filtered by band |
| Settings panel `#panel` | right edge, 320 px wide; full width under 600 px | slides in; the map stays visible and usable beside it |

## Panel content

| Group (`<fieldset>` heading) | Controls |
|------------------------------|----------|
| **Map** | "Last" N QSOs (`<select>`: 10, 20, 50, 100, 200, 500, All); "Atmosphere on the globe" (checkbox, default on) |
| **Last QSOs box** | Shown (checkbox); Size (`<select>`: 3, 5, 10, 15, 20); Newest (`<select>`: bottom, top) |
| **Appearance** | Theme (`<select>` or radio group): Automatic (device) · Automatic (sun at station) · Light · Dim · Dark. Below it, a status line: the theme in use, and why (for example "Dark: sun 12° below the horizon at JM49NH" or "Light: device preference"; with sun mode and no position, "No station position yet: following the device") |
| **Legend** | "All considered bands" (checkbox, exclusive); one checkbox per `hello.band_groups` entry, `"<label> (<n> bands)"`. Ticking a group unticks "All"; unticking every group re-ticks "All" |

Every change applies at once and is saved. There is no "Apply" button.

## Closing

The panel closes with:
- the close button (✕, `aria-label="Close settings"`);
- Escape (while focus is in the panel or on the gear);
- the gear;
- 5 minutes without input in the panel.

**Focus:** it goes to the first control on opening, and returns to the gear on closing.

## Theme application

- **`<html data-theme="light|dim|dark">`**, with the palette's custom properties.
- **Base map:** for the default OpenFreeMap style, the style switches between `liberty` (Light),
  `fiord` (Dim) and `dark` (Dark). Other maps get a dimming layer under the QSO layers.
- **On theme change:** the view, the markers and any running animation are kept.
- **Re-checking:** the device preference on its `change` event; the sun every 60 s.
