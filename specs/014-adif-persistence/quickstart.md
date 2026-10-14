# Quickstart: QSOs Kept in a Local ADIF File

Manual checks for feature 014, on a development host. Automated tests cover the same ground
(plan "Testing").

## 1. History survives a restart (User Story 1)

```sh
cargo run -- --history-file /tmp/lqm/qso-history.adi --station-callsign IS0MZK --station-locator JM49NH
# in another terminal, send a few QSOs:
printf '<CALL:5>DL1AB <QSO_DATE:8>20261014 <TIME_ON:4>1800 <BAND:3>20m <MODE:2>CW <GRIDSQUARE:4>JO62 <EOR>' \
  | nc -u -w1 127.0.0.1 12060
```

1. Open `http://localhost:8080`: the QSO is on the map.
2. Stop with Ctrl+C. The log ends with `history saved to /tmp/lqm/qso-history.adi: 1 QSOs`.
3. Look at the file: an ADIF header, then one record with `APP_LIVEQSOMAP_ADDED` and the derived
   fields; `GRIDSQUARE` is still `JO62`.
4. Start again: the log says `loaded 1 QSOs (1 reused, …)`, no `lookup` lines; the page shows the
   QSO at once.
5. Resend the same datagram: the log shows `dropping an identical resend`.

Reset: stop, `rm /tmp/lqm/qso-history.adi`, start — the map is empty.

## 2. An incomplete log is completed (User Story 2)

Put an exported log with some records lacking `GRIDSQUARE`/`LAT`/`LON` at the history file path
and start the service with callbook credentials. The page and the listener come up at once;
records appear as their lookups finish. After Ctrl+C every record has a position (or a rejected or
failed note), and every original field is unchanged (`diff` the two files ignoring added fields).

Records on bands outside `--bands` stay in the file unchanged and never appear on the map.

## 3. Enrich-only (User Story 3)

```sh
cargo run -- --enrich-only ./my-contest.adi --qrz-username … --qrz-password …
echo $?   # 0
```

- No port is opened (`lsof -i :8080` shows nothing while it runs).
- Every band is completed; records with a rejected-position or lookup-failed note are looked up
  again.
- `cargo run -- --enrich-only ./missing.adi; echo $?` prints an error and `1`, and creates nothing.

## 4. Unreadable file

```sh
printf 'not adif at all' > /tmp/lqm/qso-history.adi
cargo run -- --history-file /tmp/lqm/qso-history.adi
```

The service starts with an empty map and logs that the file was moved to
`/tmp/lqm/qso-history.adi.unreadable-<UTC time>`.

## 5. Page limit

- The count menu offers 10, 20, 50, 100, 200, 500, 1000.
- In the browser console: `localStorage.setItem('lqm.limit', 'all')`, reload: the menu shows 1000
  and `localStorage.getItem('lqm.limit')` is `"1000"`.

## 6. Old setting

`cargo run -- --history-size 50` starts and logs
`history_size is no longer used: the history is not limited; remove it`.
