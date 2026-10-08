# Contract: Configuration (delta to feature 001)

| TOML key | CLI flag | Env var | Default | Notes |
|----------|----------|---------|---------|-------|
| `map.style_url` | `--style-url` | `LQM_STYLE_URL` | `https://tiles.openfreemap.org/styles/liberty`, only when `tile_url` is unset | a MapLibre vector style |
| `map.style_attribution` | `--style-attribution` | `LQM_STYLE_ATTRIBUTION` | OpenFreeMap credit (research R2), only for the default style | HTML |
| `map.tile_url` | `--tile-url` | `LQM_TILE_URL` | **none** (changed: was the OpenStreetMap URL) | raster alternative |
| `map.tile_attribution` | `--tile-attribution` | `LQM_TILE_ATTRIBUTION` | **none** (changed) | HTML |

**Validation**: setting both `style_url` and `tile_url` stops the service with
`invalid setting map.style_url: style_url and tile_url are mutually exclusive`.

**Migration**: a feature-001 configuration that set `tile_url` keeps working as a raster base map.
Without `tile_url`, the base map becomes the OpenFreeMap vector style.
