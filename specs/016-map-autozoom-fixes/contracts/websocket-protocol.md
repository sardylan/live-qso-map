# Contract: WebSocket Protocol (delta to features 006 and 011)

- **Protocol version**: **3**, unchanged. The only change adds a field (`docs/http-api.md`
  API-4).

## `hello`: new field `globe_border_deg`

```json
{
  "type": "hello",
  "protocol": 3,
  "globe_border_deg": 10
}
```

- A JSON number, in degrees of arc, already validated by the service (0 to 45).
- The page uses 10 when the field is missing or not a number in that range.
- No other message changes.
