# Contract: WebSocket Protocol (delta to features 001–003)

- **Protocol version**: **3**.
- **Unchanged**: everything else, including `snapshot`, `upsert`, `remove`, `set_limit`, the band
  list in frequency order and `basemap`.
- **Format**: every message is one JSON object in a text frame, with a `type` field
  (`docs/http-api.md` API-2).

## `hello`: new fields `station` and `entity`

```json
{
  "type": "hello",
  "protocol": 3,
  "limits": [10, 20, 50, 100, 200, 500, null],
  "default_limit": 20,
  "bands": [ { "band": "160m", "color": "#8c564b" } ],
  "fallback_color": "#7f7f7f",
  "basemap": { "kind": "style", "url": "https://tiles.openfreemap.org/styles/liberty", "attribution": "…" },
  "station": {
    "callsign": "IS0MZK",
    "locator": "JM49NH",
    "position": { "lat": 39.3125, "lon": 9.125, "precision": "locator6", "source": "config" }
  },
  "entity": { "prefix": "IS", "name": "Sardinia", "bounds": [8.13, 38.86, 9.83, 41.31] }
}
```

### `station`

- It is `null` when none of the callsign, locator or position is configured.
- Each of its fields is either a value or `null`.
- `position` has the same shape as `qso.remote` and `qso.station` (feature 001): `lat`, `lon`,
  `precision`, `source`.
- **Position rule**: explicit `station.latitude`/`station.longitude` win and give precision
  `coordinates`. Otherwise the centre of `station.locator` is used, with precision `locator4` or
  `locator6`. With neither, `position` is `null`, and the page takes the QTH from QSOs (FR-002).

### `entity`

- It is `null` unless all of these hold:
  - a callsign is configured;
  - the country data was loaded at startup;
  - the callsign resolves to an entity;
  - the bundled table has an extent for that entity.
- `bounds` is `[west, south, east, north]` in degrees, with longitudes in [-180, 180]. When
  `west > east`, the box crosses the antimeridian. For example, `"bounds": [176.8, -19.3, -178.2, -16.0]`
  is Fiji (approximate).
- `prefix` is the cty.dat primary prefix without the WAE `*` marker. `name` is the cty.dat entity
  name.

## Page behaviour (informative)

- The page takes the QTH from `station.position`. When that is null, it uses the newest QSO's
  `station` in its own state (FR-002).
- With no displayed markers, auto-zoom frames `entity.bounds`. When `entity` is null, it frames
  500 km around the QTH (FR-010).
- An old page that receives `protocol: 3` keeps working. A new page connected to a protocol-2
  server sees no `station` and no `entity`, so it shows no house until a QSO carries a station
  position.
