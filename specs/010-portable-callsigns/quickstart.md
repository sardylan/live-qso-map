# Quickstart: verify feature 010

Everything below runs on any development host, and needs no online account. The behaviour table
in `contracts/enrichment.md` is covered by tests with injected lookup sources.

## 1. Automated checks

```bash
cargo test --locked enrich::callsign      # base callsign and retry rules (research R2)
cargo test --locked enrich::acceptance    # placeholders, nesting, 180° wrap (R4, R5)
cargo test --locked --test portable_callsigns  # the behaviour table, rows 1–18
cargo test --locked                       # nothing else changed (SC-006)
```

## 2. Replay by hand (optional)

Start the service with the country data and no providers. With no provider configured, only cty
answers, so the entity rules are visible:

```bash
cargo run -- --station-callsign IS0MZK --station-locator JM49NH
```

Send these test records with `scripts/send_burst.sh` (or any UDP sender), then look at the map
and at each QSO's fields:

| `CALL` | Fields sent | Expected on the map | `APP_LIVEQSOMAP_POS_REJECTED` |
|--------|-------------|---------------------|-------------------------------|
| `IS0/DL0AAA` | — | Sardinia reference point, precision "country" | — |
| `DL0AAA` | `LAT` `N000 00.000`, `LON` `E000 00.000` | Germany reference point | `logger:placeholder` |
| `DL0AAA` | `GRIDSQUARE` `AA00AA` | Germany reference point | `logger:placeholder` |
| `LU1ZAA` | `GRIDSQUARE` `AA00AA` | the South Pole (the locator) | — |
| `DL0AAA` | `GRIDSQUARE` `JN18` (France) | France: the logger locator is trusted | — |

## 3. With a real provider (optional)

With QRZ or HamQTH credentials configured, log `IS0/<a known German call>`. The marker should
sit in Sardinia, and the QSO's fields should show `APP_LIVEQSOMAP_POS_REJECTED` with
`qrz:outside-entity`. They should also show no `QTH` or `COUNTRY` from the German record, while
`NAME` is still present.
