# Contract: WebSocket Protocol (delta to feature 001)

`protocol` stays `2` (set by feature 002): pages already ignore unknown message types. Everything
else in `specs/001-live-qso-map/contracts/websocket-protocol.md` is unchanged.

## `hello`: band list

`bands` contains only the bands in the effective set that have a configured colour, in frequency
order. With no filter active, it is unchanged from feature 001.

## Server → client: `remove` (new)

```json
{ "type": "remove", "id": "L|IS0MZK|3809734F4" }
```

The QSO no longer exists, because a correction moved it onto an ignored band. The page removes it
from its state, recomputes its window and markers, and removes a marker left with no QSOs. A
`remove` for an id the page doesn't hold is ignored. A later `upsert` with the same id is a new QSO
for the page.
