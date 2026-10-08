# Contract: WebSocket Protocol

> **Added by feature 003**: the server message `{"type":"remove","id":…}` and the band-filtered
> `hello.bands`; the protocol stays 2. See [`../../003-band-filter/contracts/websocket-protocol.md`](../../003-band-filter/contracts/websocket-protocol.md).

> **Changed by feature 002**: protocol version 2. `hello.tiles` is replaced by `hello.basemap`. See
> [`../../002-maplibre/contracts/websocket-protocol.md`](../../002-maplibre/contracts/websocket-protocol.md).

**Endpoint**: `GET /ws` on `http_port` (default 8080), upgraded to WebSocket. There is one connection
per open page (FR-020).
**Encoding**: UTF-8 JSON text frames. Each frame holds one object with a `type` discriminator.
Unknown `type` values MUST be ignored by both sides (forward compatibility).

## Server → client

### `hello` (sent once, immediately after connect)

```json
{
  "type": "hello",
  "protocol": 1,
  "limits": [10, 20, 50, 100, 200, 500, null],
  "default_limit": 20,
  "bands": [
    { "band": "160m", "color": "#8c564b" },
    { "band": "80m", "color": "#9467bd" },
    { "band": "40m", "color": "#d62728" },
    { "band": "20m", "color": "#1f77b4" }
  ],
  "fallback_color": "#7f7f7f",
  "tiles": { "url": "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
             "attribution": "&copy; OpenStreetMap contributors" }
}
```

`null` in `limits` means "All". `bands` is an array in frequency order, lowest frequency first
(`docs/ham-radio.md` HR-2). A JSON object cannot guarantee key order, so it is not used, and
clients MUST NOT re-sort the list.

### `snapshot` (after `hello`, after every `set_limit`, and after the server detects lag)

```json
{ "type": "snapshot", "limit": 20, "qsos": [ /* QsoView, ascending by sort_key */ ] }
```

The client replaces its whole state with this list.

### `upsert` (live: new QSO or correction)

```json
{ "type": "upsert", "qso": { /* QsoView */ } }
```

The client applies it only if `qso.id` is unknown or `qso.rev` is greater than the stored `rev`. Then
it recomputes the visible window: the last `limit` QSOs by `sort_key`.

## Client → server

### `set_limit`

```json
{ "type": "set_limit", "limit": 100 }
```

`limit` is one of the `hello.limits` values. `null` means All. The server answers with a `snapshot`.
An invalid value is ignored and logged at debug. The client sends `set_limit` again after every
reconnect (FR-031).

## QsoView

```json
{
  "id": "L|IS0MZK|3809734F4",
  "rev": 3,
  "sort_key": "2026-10-08T15:04:22Z",
  "call": "K6QA",
  "band": "40m",
  "mode": "SSB",
  "submode": "LSB",
  "datetime": "2026-10-08T15:04:22Z",
  "station_call": "IS0MZK",
  "remote":  { "lat": 37.42, "lon": -122.08, "precision": "coordinates", "source": "qrz" },
  "station": { "lat": 39.3125, "lon": 9.125, "precision": "locator6", "source": "config" },
  "adif": { "CALL": "K6QA", "BAND": "40m", "...": "..." }
}
```

| Field | Rule |
|-------|------|
| `remote` | `null` when the position could not be resolved. The QSO still counts toward the window but draws nothing (FR-019). |
| `station` | `null` when there is no station position. The marker is drawn without a line. |
| `sort_key` | Fixed at the first insert. It decides the order of the window and of markers (FR-028, FR-030). |
| `adif` | The full enriched record, shown in the popup and available for later features. |
| `band` | Missing or unknown bands use `fallback_color`. |

## Reconnection (FR-031)

When the socket closes, the client retries with exponential backoff (1 s, doubling, capped at 30 s, with
jitter). After it reconnects it waits for `hello`, then sends `set_limit` with its current selection.
The `snapshot` that follows replaces its state.
