# live-qso-map

[![CircleCI](https://dl.circleci.com/status-badge/img/gh/sardylan/live-qso-map/tree/main.svg?style=shield)](https://dl.circleci.com/status-badge/redirect/gh/sardylan/live-qso-map/tree/main)

Small tool for displaying QSO in world map in real time

## Usage

```
Usage: live-qso-map [OPTIONS] --qrzcom-user <QRZCOM_USER> --qrzcom-password <QRZCOM_PASSWORD> --home-latitude <HOME_LATITUDE> --home-longitude <HOME_LONGITUDE>

Options:
  -l, --log-level <LOG_LEVEL>
          Set the level of logging messages
          
          [default: WARN]

  -H, --http-host <HTTP_HOST>
          Binding address for the HTTP server
          
          [default: ::]

  -P, --http-port <HTTP_PORT>
          Port for the HTTP server
          
          [default: 8641]

  -I, --bind-host <BIND_HOST>
          Binding address for the QARTest UDP socket receiver
          
          [default: ::]

  -Q, --bind-port <BIND_PORT>
          Port for the QARTest UDP socket receiver
          
          [default: 12060]

  -u, --qrzcom-user <QRZCOM_USER>
          Username for the QRZ.com XML APIs

  -p, --qrzcom-password <QRZCOM_PASSWORD>
          Password for the QRZ.com XML APIs

  -a, --home-latitude <HOME_LATITUDE>
          Latitude of the home station

  -b, --home-longitude <HOME_LONGITUDE>
          Longitude of the home station

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```