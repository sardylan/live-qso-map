# Data Model: Full-Screen Side Column

**Feature**: `018-fullscreen-side-column` | **Date**: 2026-10-25

Nothing is stored by the service. The feature adds one value the service sends, one value the page
carries, and one flag the page keeps for the tab.

## Software version (service)

| Attribute | Value |
|-----------|-------|
| Source | `env!("CARGO_PKG_VERSION")`, the same string the service logs at startup (`src/main.rs`) |
| Format | `MAJOR.MINOR.PATCH`, for example `3.3.0` |
| Sent in | `hello.version` ([contract](contracts/websocket-protocol.md)) |
| Shown as | `v` + version in `#version` at the bottom of the column |

## Page version (page)

| Attribute | Value |
|-----------|-------|
| Source | `<meta name="lqm-version">` in `index.html`, filled by the service when it serves `/` (R5) |
| Valid when | a non-empty string that is not the template token `__LQM_VERSION__` |
| Lifetime | the loaded page; a reload replaces it with the serving release's version |

## Reload flag (page, per tab)

| Attribute | Value |
|-----------|-------|
| Key | `sessionStorage['lqm.reloaded-for']` |
| Value | the service version the tab last reloaded for |
| Read | on every `hello` |
| Written | just before `location.reload()` |
| Not a viewer setting | Principle IV does not apply; without session storage the page never reloads by itself (R6) |

## Reload decision

```text
on hello(version = S), page version P, flag F:
  if P valid and S valid and P != S and F != S:
      F := S; reload
  else:
      carry on (show v<S>, handle hello as today)
```

| P | S | F | Result |
|---|---|---|--------|
| 3.3.0 | 3.3.0 | — | carry on |
| 3.3.0 | 3.4.0 | — | reload |
| 3.3.0 | 3.4.0 | 3.4.0 | carry on (loop guard) |
| 3.3.0 | 3.4.0 | 3.2.0 | reload |
| 3.3.0 | missing | — | carry on |
| missing / token | 3.4.0 | — | carry on |

## Column layout (page)

Column width: `clamp(280px, 24rem, 45vw)` (R1). From top to bottom (FR-002), with the relative
text sizes of R3:

| Element | Content | Size |
|---------|---------|------|
| `#station` | `<band> · <operator>` (feature 017) | 2.4rem, largest |
| `#clock` | `HH:MM:SS UTC` | 2rem |
| `#score` | score line (feature 007) | 1.6rem |
| `#recent` | last QSOs box (features 005, 006); whole rows only, oldest dropped when short of space; hidden below 600 px | 1rem |
| `#version` | `v3.3.0` | 0.8rem, muted |
