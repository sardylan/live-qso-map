#!/usr/bin/env python3
"""Send QARTest `contactinfo` datagrams over UDP, one per callsign.

Each QSO gets its own logger id and the current UTC time, so live-qso-map
treats every callsign as a new QSO. The layout follows the reference capture
in specs/001-live-qso-map/reference/.

Usage:
    python3 scripts/simulate_qartest_qso.py [--host 127.0.0.1] [--port 12060]
        [--mycall IS0MZK] [--operator CALL] [--band 40] [--mode LSB] [--delay 0] CALL [CALL ...]

Uses only the Python standard library.
"""

import argparse
import datetime
import secrets
import socket
import time
from xml.sax.saxutils import escape


def payload(call: str, mycall: str, operator: str, band: str, mode: str, nr: int) -> bytes:
    timestamp = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d %H:%M:%S")
    fields = [
        ("logger", "QARTest 16.9.1"),
        ("contestname", "CQ-WW-SSB"),
        ("timestamp", timestamp),
        ("mycall", mycall),
        ("band", band),
        ("txfreq", "0"),
        ("operator", operator),
        ("mode", mode),
        ("call", call),
        ("countryprefix", ""),
        ("wpxprefix", ""),
        ("snt", "59"),
        ("rcv", "59"),
        ("nr", str(nr)),
        ("exch1", "14"),
        ("exch2", ""),
        ("exch3", ""),
        ("duplicate", "False"),
        ("stationname", ""),
        ("points", "1"),
        ("xqso", "0"),
        ("id", secrets.token_hex(5).upper()),
    ]
    body = "".join(f"<{name}>{escape(value)}</{name}>" for name, value in fields)
    return f'<?xml version="1.0"?><contactinfo>{body}</contactinfo>'.encode("utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description="Send simulated QARTest QSOs over UDP.")
    parser.add_argument("calls", nargs="+", metavar="CALL", help="callsign of a worked station")
    parser.add_argument("--host", default="127.0.0.1", help="destination host (default: 127.0.0.1)")
    parser.add_argument("--port", type=int, default=12060, help="UDP port (default: 12060)")
    parser.add_argument("--mycall", default="IS0MZK", help="station callsign (default: IS0MZK)")
    parser.add_argument("--operator", default="", help="operator callsign, as set by OPON (default: none)")
    parser.add_argument("--band", default="40", help="band in metres (default: 40)")
    parser.add_argument("--mode", default="LSB", help="QARTest mode (default: LSB)")
    parser.add_argument("--delay", type=float, default=0.0, help="seconds between QSOs (default: 0)")
    args = parser.parse_args()

    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as sock:
        for nr, call in enumerate(args.calls, start=1):
            sock.sendto(payload(call.upper(), args.mycall, args.operator.upper(), args.band, args.mode, nr), (args.host, args.port))
            print(f"sent {call.upper()} to {args.host}:{args.port}")
            if args.delay and nr < len(args.calls):
                time.sleep(args.delay)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
