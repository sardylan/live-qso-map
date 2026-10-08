# Data Model: Live QSO Map

**Feature**: `001-live-qso-map` | **Date**: 2026-10-08

Everything is held in memory. Nothing is persisted except the downloaded country data file.

## AdifRecord

An ordered map from uppercase field name to string value. It is the single QSO representation inside
the service (FR-006).

| Rule | Source |
|------|--------|
| Field names are uppercase ASCII. Values are UTF-8 text, and the length on output is in bytes. | ADIF 3.1 |
| Required for the pipeline: `CALL`, plus `QSO_DATE` and `TIME_ON` (`YYYYMMDD`, `HHMM[SS]`). A record without them is rejected at ingestion. | FR-007 (identity), FR-030 (ordering) |
| `BAND` and `MODE` are optional, but take part in the composite identity when present. | FR-007 |
| Enrichment never overwrites a field that is already present. | FR-011, FR-017 |
| Application fields use the `APP_LIVEQSOMAP_` prefix. | R7 |

Application fields:

| Field | Meaning |
|-------|---------|
| `APP_LIVEQSOMAP_LOGGER_ID` | Logger QSO id (QARTest `<id>`) |
| `APP_LIVEQSOMAP_POS_SOURCE` | `logger`, `qrz`, `hamqth`, `qrzcq`, `cty` |
| `APP_LIVEQSOMAP_POS_PRECISION` | `coordinates`, `locator6`, `locator4`, `country` |
| `APP_LIVEQSOMAP_STATION_POS_SOURCE` | `logger`, `config` |
| `APP_LIVEQSOMAP_QARTEST_<ELEMENT>` | `COUNTRYPREFIX`, `NR`, `POINTS`, `DUPLICATE`, `XQSO`, `STATIONNAME` |

## SourceFormat

`Adif | QarTest | Unrecognised`, decided from the content of the datagram (research R5).

## QsoEvent

What a parsed message means, produced at ingestion:

```text
QsoEvent {
    kind: New | Correction,     // QARTest contactinfo → New, contactreplace → Correction; ADIF → New
    record: AdifRecord,         // normalised, not yet enriched
    logger_id: Option<String>,  // QARTest <id>
    previous: Option<(call, timestamp)>, // QARTest oldcall/oldtimestamp; used only when no logger_id
    received_at: Instant,
}
```

QARTest `dynamicresults` and `contactdelete` messages produce no event (FR-005a).

## Identity

```text
Identity = Logger { station_call, logger_id }
         | Composite { station_call, call, band, mode, qso_date, time_on_hhmm }
```

- All components are trimmed and uppercased before comparison (FR-007).
- `station_call` is `STATION_CALLSIGN`, then the configured default callsign, then empty. `OPERATOR`
  is not used, because it changes per operator in multi-op stations. The station callsign default is
  applied at ingest, **before** the identity is computed, so identities stay stable.
- The canonical string form is used as the `id` in the WebSocket protocol, for example
  `L|IS0MZK|3809734F4` or `C|IS0MZK|DL5QFN|40M|SSB|20261008|1504`.

## ResendGate (before enrichment)

```text
ResendGate {
    entries: HashMap<Identity, GateEntry>,  // GateEntry { fingerprint: u64, rev: u32 }
    order: VecDeque<Identity>,              // insertion order, for eviction
    capacity: usize,                        // = history_size; 0 = unbounded
}
```

| Incoming | State | Result |
|----------|-------|--------|
| identity unknown | — | insert `rev = 1` and forward as **new** |
| identity known, same fingerprint | — | **drop** (identical resend, FR-008) |
| identity known, different fingerprint | — | `rev += 1`, store the fingerprint and forward as an **update** (FR-009) |
| Correction without `logger_id` | an `old*` composite identity exists | match by the old identity, re-key to the new one, `rev += 1` |
| Correction with an unknown identity | — | treat as **new** (spec edge case) |

The fingerprint is a hash of the normalised record before enrichment, with fields in sorted order.

## Position

```text
Position  { lat: f64, lon: f64, precision: Precision, source: PositionSource }
Precision = Country < Locator4 < Locator6 < Coordinates   (Ord)
PositionSource = Logger | Qrz | HamQth | QrzCq | Cty | Config
```

Validation: `lat` must be in [-90, 90] and `lon` in [-180, 180]. Out-of-range values make the
candidate invalid.

## LookupCandidate

The result of one source for one callsign:

```text
LookupCandidate { source, position: Option<Position>, fields: AdifRecord /* NAME, QTH, DXCC, CQZ… */ }
```

Selection: the highest `precision` wins. A tie goes to the source listed first in `provider_order`,
and `cty` always comes last (research R11).

## CountryEntity and CountryData

```text
CountryEntity { name, primary_prefix, wae_only: bool, cq: u8, itu: u8, continent, lat, lon /* +E */ }
CountryData   { exact: HashMap<String, (EntityIdx, Overrides)>, prefixes: HashMap<String, (EntityIdx, Overrides)>, max_prefix_len }
```

The longitude is converted from the cty.dat convention (+W) to +E at parse time.

## EnrichedQso (history element)

```text
EnrichedQso {
    id: String,                    // canonical Identity
    rev: u32,
    sort_key: (DateTime, String),  // first-seen QSO_DATE+TIME_ON, id. Never changed by corrections (FR-028)
    record: AdifRecord,            // enriched
    remote: Option<Position>,      // None → stored and broadcast, but not drawn (FR-019)
    station: Option<Position>,     // None → no geodesic line
}
```

## History

```text
History {
    by_id: HashMap<String, EnrichedQso>,
    ordered: BTreeSet<(DateTime, String)>,
    capacity: usize,               // history_size; 0 = unbounded
}
```

| Operation | Rule |
|-----------|------|
| `upsert(q)` | Accept only when `id` is unknown or `q.rev` is greater than the stored `rev`. Keep the stored `sort_key` on update. Return `Accepted` or `Stale`. |
| eviction | After an insert, while `len > capacity > 0`, remove the smallest `sort_key`. |
| `latest(n)` | The last `n` by `sort_key` (`None` means all), in ascending order. |

## Broadcast message (channel 2)

```text
HistoryChange::Upsert(Arc<EnrichedQso>)
```

It is sent while the history lock is held (research R4).

## MapSession (one per open page)

```text
MapSession { limit: Option<usize> /* default 20; None = All */, rx: broadcast::Receiver<HistoryChange> }
```

| Event | Action |
|-------|--------|
| connect | subscribe, then send `hello` and a `snapshot(latest(limit))` |
| client `set_limit` | store the limit and send `snapshot(latest(limit))` |
| channel `Upsert` | send `upsert` |
| channel `Lagged` | send `snapshot(latest(limit))` |
| socket closed | drop the session |

## Configuration

See [contracts/configuration.md](contracts/configuration.md).

## Frontend state (`map-state.js`)

```text
qsos:    Map<id, QsoView>            // newest rev wins
limit:   number | null
visible: QsoView[]                   // last `limit` by sort_key
markers: Map<"lat,lon", { qsos: QsoView[] /* visible only, sorted */, color, zIndex, station }>
```

`diffLayers(prev, next)` compares two `markers` maps and returns `{add, update, remove}` by marker
key, so that `app.js` only applies the result to Leaflet.

Each marker's colour comes from the band of its most recent visible QSO. Its z-index is the rank of
that QSO in time order. Its line runs from the station position of that QSO. A marker with no visible
QSOs is removed.

## Connection state (`connection.js`)

```text
attempt: number        // reset by onOpen
limit:   number | null // loaded from / saved to sessionStorage, default 20
```

- `backoffDelay(attempt, rand)`: 1, 2, 4, 8, 16, then 30 s, with ±20% jitter.
- `onHello(state)`: returns the `set_limit` message to send after every (re)connect.
