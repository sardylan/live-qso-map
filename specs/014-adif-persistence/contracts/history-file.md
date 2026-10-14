# Contract: The History File (feature 014)

The service's QSO history between runs. ADIF 3.1.4, ASCII/UTF-8, one record per line.

## Location

- Setting `history_file`: `--history-file <PATH>`, `LQM_HISTORY_FILE`, TOML top-level
  `history_file = "…"`. Precedence: command line > environment > file > default.
- Default: `qso-history.adi` in the folder of `country_data.path` once that path is resolved:

  | Install | Default file |
  |---------|--------------|
  | Debian package (conffile sets the country data in `/var/lib/live-qso-map/`) | `/var/lib/live-qso-map/qso-history.adi` |
  | Container (country data in the `/var/lib/live-qso-map` volume) | `/var/lib/live-qso-map/qso-history.adi` |
  | Windows, macOS (program folder) | `<program folder>/qso-history.adi` |
  | Linux from source, default `./cty.dat` | `./qso-history.adi` |

- A relative `history_file` is resolved like `country_data.path` (program folder on Windows and
  macOS).

## Layout written by the service

```text
live-qso-map QSO history, written 2026-10-14T18:03:12Z
<ADIF_VER:5>3.1.4 <PROGRAMID:12>live-qso-map <PROGRAMVERSION:5>0.9.0 <CREATED_TIMESTAMP:15>20261014 180312 <EOH>
<APP_LIVEQSOMAP_ADDED:…>… <APP_LIVEQSOMAP_POS_PRECISION:8>locator6 <APP_LIVEQSOMAP_POS_SOURCE:3>qrz <BAND:3>20m <CALL:6>DL1ABC … <EOR>
…
```

- Records in QSO time order; fields in name order (the record's iteration order).
- Field lengths in bytes.

## Fields the service adds

| Field | Meaning |
|-------|---------|
| `APP_LIVEQSOMAP_ADDED` | comma-separated names of every field the service added to the record (including the `APP_LIVEQSOMAP_*` fields below and merged lookup fields such as `QTH`, `GRIDSQUARE`, `MY_GRIDSQUARE`) |
| `APP_LIVEQSOMAP_POS_SOURCE`, `APP_LIVEQSOMAP_POS_PRECISION` | source and precision of `LAT`/`LON` (HR-7, unchanged) |
| `APP_LIVEQSOMAP_POS_REJECTED` | rejected positions (feature 010, unchanged) |
| `APP_LIVEQSOMAP_LOOKUP_FAILED` | new: `source:kind[,…]`, `kind` ∈ `error`, `timeout`; present when every online source failed on the logged call |
| `APP_LIVEQSOMAP_STATION_POS_SOURCE` | station position source (unchanged) |

Fields not listed in `APP_LIVEQSOMAP_ADDED` are the logger's (or the external file's) and are
written back unchanged (HR-11).

## Reading

- Any ADIF file: header optional, records in any order, `<EOR>`-terminated.
- A record without a valid `CALL`, `QSO_DATE` and `TIME_ON` is skipped and counted.
- Unreadable (I/O error, or non-empty with neither `<EOH>` nor `<EOR>`):
  - service: renamed to `<name>.unreadable-YYYYMMDDTHHMMSSZ`, warning logged, empty start; if the
    rename fails, startup fails with exit code 1;
  - enrich-only: exit code 1, nothing written.
- A missing file: the service starts empty; enrich-only exits with code 1.

## Writing

- At a graceful stop (Ctrl+C, SIGTERM) and at the end of an enrich-only run, always — even if the
  file was deleted while the service ran (FR-003).
- To `<name>.tmp` in the same folder, synced, then renamed over `<name>`. On failure the temporary
  file is removed, an error is logged, and the previous file is unchanged. The exit code is
  unaffected for the service; enrich-only exits with code 1.

## Log lines (info unless noted)

- `history file <path>: loaded N QSOs (R reused, E to enrich, P on other bands), skipped M malformed`
- `history file <path> is unreadable (<reason>); moved to <new path>` (warn)
- `history saved to <path>: N QSOs` / `cannot save the history to <path>: <reason>` (error)
- enrich-only, at the end: `enriched <path>: N QSOs, X placed from the record, Y by lookup,
  F with a failed lookup, Z without a position`
