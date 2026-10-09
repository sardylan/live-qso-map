# Contract: HTTP Endpoints (delta to features 001–003)

## New static assets

| Method | Path | Response |
|--------|------|----------|
| GET | `/qth.js` | `200`, `text/javascript` |
| GET | `/autozoom.js` | `200`, `text/javascript` |
| GET | `/autozoom-geo.js` | `200`, `text/javascript` |
| GET | `/recent.js` | `200`, `text/javascript` |

Every static page file is served with `Cache-Control: no-cache`, so browsers revalidate it and a
rebuilt service never runs with stale page modules.

Static page files are exempt from the JSON rule (`docs/http-api.md` API-5). Every existing route
is unchanged.

## Error responses (FR-027, `docs/http-api.md` API-3)

Every error has `Content-Type: application/json` and the body `{"error": "<code>", "message": "<text>"}`.

| Situation | Status | `error` | Example `message` |
|-----------|--------|---------|-------------------|
| unknown path | `404` | `not_found` | `no such endpoint: /foo` |
| known path, wrong method (e.g. `POST /healthz`) | `405` | `method_not_allowed` | `method POST not allowed on /healthz` |
| `GET /ws` without a valid WebSocket upgrade | the rejection's own status (`400`, `405` or `426`) | `websocket_upgrade_required` | the framework's rejection text |

- **405 header**: a `405` response keeps the `Allow` header.
- **Message contents**: messages never include configuration values or file system paths.
- **Unchanged**: `/healthz` keeps its JSON success body.
