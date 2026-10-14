# Contract: WebSocket Protocol Changes (feature 014, protocol version 4, unchanged)

Follows `docs/http-api.md`. No message changes shape, so the version stays 4 (API-4).

## `hello.limits`

```json
{ "type": "hello", "protocol": 4, "limits": [10, 20, 50, 100, 200, 500, 1000], "default_limit": 20, … }
```

`null` (All) is no longer listed.

## `set_limit.limit`

| Value | Answer |
|-------|--------|
| one of `hello.limits` | snapshot of that many newest QSOs on the chosen bands |
| `null` (older page, "All") | treated as `1000`; the snapshot's `limit` is `1000` |
| anything else | ignored, as today |

## `snapshot`

At most 1000 QSOs, whatever the history holds. `limit` echoes the effective value (never `null`).

## Page storage (`lqm.limit`, constitution IV)

| Stored value | Read as | Written back |
|--------------|---------|--------------|
| `"10"` … `"500"`, `"1000"` | that number | unchanged |
| `"all"` | 1000 | `"1000"` |
| missing, unknown, unreadable | 20 (default) | unchanged |
