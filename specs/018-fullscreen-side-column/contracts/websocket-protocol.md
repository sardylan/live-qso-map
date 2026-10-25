# Contract: WebSocket Protocol and Page (delta to feature 017)

- **Protocol version**: **4**, unchanged. This feature adds one field to `hello`; nothing that
  exists changes (`docs/http-api.md` API-4, research R7).
- Pages built for protocol 4 already ignore fields they don't know.

## Changed server message: `hello`

One new field, after the existing ones:

```json
{
  "type": "hello",
  "protocol": 4,
  "version": "3.4.0",
  "...": "every field of feature 017, unchanged"
}
```

| Field | Type | Meaning |
|-------|------|---------|
| `version` | string | the service's software version, `MAJOR.MINOR.PATCH`, the same it logs at startup |

A service older than this feature sends no `version`.

## Changed static file: `/` (`index.html`)

Not a payload (API-5), recorded here because the page depends on it:

```html
<meta name="lqm-version" content="3.4.0">
```

The service fills `content` with the same string as `hello.version`. The file shipped in the
source tree carries the token `__LQM_VERSION__`, which never reaches a browser through the service.

## Page behaviour on `hello`

1. Read the page version from the meta (missing, empty or `__LQM_VERSION__` → no version).
2. Read the tab's reload flag `sessionStorage['lqm.reloaded-for']`; when session storage is
   unavailable, skip to step 4 (no reload without a loop guard).
3. If both versions are present, they differ, and the flag is not the service version: write the
   service version to the flag and reload the page. Nothing else from this `hello` is handled.
4. Otherwise: show `v<version>` in `#version` (empty when the service sent none) and handle `hello`
   as in feature 017.

This runs on the first connection and on every reconnection.

## Unchanged

`snapshot`, `upsert`, `remove`, `score`, `station_status` and `set_limit` are as in features 001 to
017.
