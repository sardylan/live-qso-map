# HTTP and WebSocket Interface Rules

These rules are binding: the constitution's Technical Constraints link to them. They apply to every
interface that live-qso-map itself exposes over HTTP: REST endpoints, WebSocket sessions and error
responses. Plans check them in the Constitution Check ("Interface rules"), and so do code reviews.

Each feature's contracts (`specs/*/contracts/`) define which endpoints and messages exist. This
document defines how their payloads look.

## 1. Rules

### API-1 HTTP payloads are JSON

- Every request body and every response body of an HTTP endpoint MUST be JSON.
- It MUST be encoded as UTF-8 and sent with `Content-Type: application/json`.
- An endpoint that accepts a body MUST reject a body that is not JSON. It answers
  `415 Unsupported Media Type` when the content type is wrong and `400 Bad Request` when the JSON is
  malformed, both with an API-3 error body.
- Example: `GET /healthz` returns `{"status":"ok","history":12,"sessions":2,"uptime_s":3600}`.

### API-2 WebSocket messages are JSON text frames

- Every application message, in either direction, MUST be one JSON object in one text frame.
- Every message MUST carry a string field `type` that names the message (for example `hello`,
  `snapshot`, `upsert`, `remove`, `set_limit`).
- Binary frames MUST NOT be used.
- Control frames (ping, pong, close) are part of the WebSocket protocol, not payloads, so this rule
  does not cover them.
- A malformed or unknown client message is ignored (and may be logged). It never closes the
  session, so pages and service versions can differ without breaking each other.

### API-3 Errors have a JSON body

- Every `4xx` and `5xx` response MUST have a JSON body, for example:

  ```json
  {"error": "not_found", "message": "no such endpoint: /foo"}
  ```

- `error` is a stable code in `snake_case`; clients may branch on it.
- `message` is human-readable text for logs and people; clients MUST NOT parse it.
- Other fields may be added when they help, such as `"setting"` for a configuration error.
- This covers every error the service returns, including:
  - unknown paths (`404`);
  - methods that are not allowed (`405`);
  - failed WebSocket upgrades on `/ws` (`400`/`426`);
  - errors produced by the HTTP framework itself, such as body rejections.
- Error bodies MUST NOT leak secrets: callbook credentials, tokens, full file system paths.

### API-4 JSON conventions

- **Field names**: field names are `snake_case`, such as `fallback_color` and `uptime_s`.
- **Times**: times are UTC in ISO 8601 with a `Z` suffix (HR-1), for example
  `"2026-10-09T15:04:00Z"`.
- **Order**: when order matters, use an array. A JSON object's key order is not guaranteed, so a list
  that must keep its order must not be sent as an object used as a map. This is what made the
  feature-001 legend alphabetical, against HR-2.
- **Absent values**: an absent value is `null` or an omitted field, as the contract says. It is never
  an empty string standing in for "unknown".
- **Breaking changes**: a breaking change to a message or a body MUST bump the protocol version
  announced to clients (for example `hello.protocol`), and update the feature's contract.

### API-5 What these rules do not cover

These rules do not apply to:

- **The page's static files**: HTML, JavaScript, CSS and other assets the service serves for the
  map page are files, not payloads, and keep their own content types.
- **Data the service fetches** from third parties, such as callbook XML or `cty.dat`. Its format is
  set by the provider, so these rules govern only what the service itself exposes.
- **Non-HTTP inputs**: UDP ingestion of ADIF and QARTest XML is governed by `docs/ham-radio.md`
  (HR-11) and the feature-001 contracts.

## 2. Testing

- Every endpoint and message type MUST have a test that checks its JSON shape (contract test).
- Error paths MUST have tests that check the status code, `Content-Type: application/json` and the
  `error` code (API-3).

## Changelog

| Date | Change |
|------|--------|
| 2026-10-09 | Initial version: rules API-1 to API-5 (constitution 1.3.0). |
