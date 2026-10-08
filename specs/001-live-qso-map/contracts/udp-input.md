# Contract: UDP Input

**Port**: `udp_port` (default 12060) on `bind_address` (default `0.0.0.0`). Open to any sender (FR-035).
**Unit**: one datagram, up to 65,507 bytes. The format is detected per datagram (FR-002).

## Detection (in order)

| # | Condition (after skipping leading whitespace) | Format |
|---|-----------------------------------------------|--------|
| 1 | Starts with `<`. Parses as XML. Root element is `contactinfo`, `contactreplace`, `contactdelete` or `dynamicresults`. The `<logger>` or `<soft>` text starts with `QARTest`. | QARTest |
| 2 | Contains an ADIF data specifier `<NAME:LEN>` (case-insensitive), or `<EOH>`/`<EOR>` | ADIF |
| 3 | Anything else, including ADX (`<ADX>` root) and XML from other loggers | Unrecognised: logged at **warn** and dropped |

## ADIF datagrams (FR-003)

- Optional header: everything up to and including `<EOH>` is skipped.
- One or more records, each ending in `<EOR>`. Each record becomes one `QsoEvent{kind: New}`.
- Trailing data without an `<EOR>` is treated as a final record if it contains at least `CALL`.
- A malformed record is logged at warn and skipped. The other records in the same datagram are still
  processed (SC-006).
- A record without `CALL`, `QSO_DATE` or `TIME_ON` is rejected and logged at warn.

Example:

```text
<ADIF_VER:5>3.1.4 <EOH>
<CALL:5>DL5AB <QSO_DATE:8>20261008 <TIME_ON:6>150404 <BAND:3>40m <MODE:3>SSB
<GRIDSQUARE:6>JO62QM <MY_GRIDSQUARE:6>JM49NH <STATION_CALLSIGN:6>IS0MZK <EOR>
```

## QARTest datagrams (FR-004, FR-004a)

| Root | Event | Log level |
|------|-------|-----------|
| `contactinfo` | `New` | debug |
| `contactreplace` | `Correction`, with `previous = (oldcall, oldtimestamp)` | debug |
| `contactdelete` | none (out of scope) | debug |
| `dynamicresults` | none (score) | debug |

The field mapping is FR-004a in the spec. Reference input: `../reference/qartest-16.9.1-cqww-ssb.txt`.

Expected conversion of packet #1 (DL5QFN):

```text
CALL=DL5QFN  STATION_CALLSIGN=IS0MZK  QSO_DATE=20261008  TIME_ON=150404
FREQ=7.000000  BAND=40m  MODE=SSB  SUBMODE=LSB  RST_SENT=59  RST_RCVD=59
SRX_STRING=14  PFX=DL5  CONTEST_ID=CQ-WW-SSB  PROGRAMID=QARTest  PROGRAMVERSION=16.9.1
APP_LIVEQSOMAP_LOGGER_ID=3807B771C  APP_LIVEQSOMAP_QARTEST_COUNTRYPREFIX=DL
APP_LIVEQSOMAP_QARTEST_NR=4  APP_LIVEQSOMAP_QARTEST_POINTS=1  APP_LIVEQSOMAP_QARTEST_DUPLICATE=False
APP_LIVEQSOMAP_QARTEST_XQSO=0
(OPERATOR, STATIONNAME: empty → absent)
```

## Guarantees

- No datagram can crash the service. A parse failure affects only that datagram, or only that record.
- Processing is non-blocking for the sender. When channel 1 is full, the ingest task waits
  (backpressure), and the kernel's receive buffer absorbs short bursts (SC-004).
