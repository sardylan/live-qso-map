#!/usr/bin/env python3
"""Capture every UDP datagram received on a port into a text file.

Each datagram is written between explicit markers, with its byte length, so
packet boundaries are unambiguous even if the payload contains newlines:

    ===== BEGIN PACKET #1 | 2026-10-08T14:03:12.123456+00:00 | from 192.168.1.10:51234 | 312 bytes =====
    <raw payload bytes, exactly as received>
    ===== END PACKET #1 =====

Usage:
    python3 udp_capture.py [--bind 0.0.0.0] [--port 12060] [--out capture.txt]

Stop with Ctrl+C. Uses only the Python standard library.
"""

import argparse
import datetime
import socket
import sys


def main() -> int:
    parser = argparse.ArgumentParser(description="Capture UDP datagrams with packet markers.")
    parser.add_argument("--bind", default="0.0.0.0", help="address to bind (default: 0.0.0.0)")
    parser.add_argument("--port", type=int, default=12060, help="UDP port (default: 12060)")
    parser.add_argument("--out", default="udp_capture.txt", help="output file (default: udp_capture.txt)")
    args = parser.parse_args()

    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    if hasattr(socket, "SO_REUSEPORT"):
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEPORT, 1)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_BROADCAST, 1)
    sock.bind((args.bind, args.port))

    print(f"Listening on UDP {args.bind}:{args.port}, writing to {args.out} (Ctrl+C to stop)")

    count = 0
    # Append in binary mode: payload bytes are stored exactly as received.
    with open(args.out, "ab") as out:
        try:
            while True:
                data, (host, port) = sock.recvfrom(65535)
                count += 1
                ts = datetime.datetime.now(datetime.timezone.utc).isoformat()
                header = (
                    f"===== BEGIN PACKET #{count} | {ts} | from {host}:{port} | {len(data)} bytes =====\n"
                )
                out.write(header.encode("ascii"))
                out.write(data)
                out.write(f"\n===== END PACKET #{count} =====\n\n".encode("ascii"))
                out.flush()
                print(f"#{count} {ts} {host}:{port} {len(data)} bytes")
        except KeyboardInterrupt:
            print(f"\nStopped. {count} packet(s) captured in {args.out}")
        finally:
            sock.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
