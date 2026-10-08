# Contract: WebSocket Protocol (delta to feature 001)

- **Protocol version**: **2**. `hello.tiles` is replaced by `hello.basemap`.
- **Unchanged**: everything else, including snapshots, upserts, `set_limit` and the band list in
  frequency order.

```json
{
  "type": "hello",
  "protocol": 2,
  "limits": [10, 20, 50, 100, 200, 500, null],
  "default_limit": 20,
  "bands": [ { "band": "160m", "color": "#8c564b" } ],
  "fallback_color": "#7f7f7f",
  "basemap": {
    "kind": "style",
    "url": "https://tiles.openfreemap.org/styles/liberty",
    "attribution": "<a href=\"https://openfreemap.org\" target=\"_blank\">OpenFreeMap</a> © …"
  }
}
```

The `basemap` field takes one of two shapes:

| `kind` | `url` | `attribution` |
|--------|-------|---------------|
| `style` | a MapLibre style JSON URL | HTML from the operator's configuration, shown as the map attribution |
| `raster` | a tile URL template with `{z}`, `{x}`, `{y}` | HTML from the operator's configuration |
