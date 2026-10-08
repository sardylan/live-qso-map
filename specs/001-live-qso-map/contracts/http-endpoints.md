# Contract: HTTP Endpoints

The service listens on `http_port` (default 8080) on `bind_address`. There is no authentication
(FR-035). All assets are embedded in the binary.

| Method | Path | Response |
|--------|------|----------|
| GET | `/` | `index.html`: the map page (`text/html; charset=utf-8`) |
| GET | `/app.js` | the page bootstrap module (`text/javascript`) |
| GET | `/map-state.js` | the pure state module (`text/javascript`) |
| GET | `/geodesic.js` | the great-circle helper (`text/javascript`) |
| GET | `/app.css` | the styles (`text/css`) |
| GET | `/ws` | WebSocket upgrade. See [websocket-protocol.md](websocket-protocol.md). |
| GET | `/healthz` | `200 OK`, `application/json`: `{"status":"ok","history":<n>,"sessions":<n>,"uptime_s":<n>}` |
| any | other | `404` |

`index.html` loads Leaflet 1.9.4 from cdnjs with Subresource Integrity:

```html
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/leaflet/1.9.4/leaflet.min.css"
      integrity="sha512-…" crossorigin="anonymous">
<script src="https://cdnjs.cloudflare.com/ajax/libs/leaflet/1.9.4/leaflet.min.js"
        integrity="sha512-…" crossorigin="anonymous"></script>
```

The SRI hashes are filled in at implementation time from cdnjs.

The page builds the WebSocket URL from `location`: `wss:` when the page is served over `https:`,
otherwise `ws:`. This lets it work behind a TLS-terminating reverse proxy in cloud deployments.
