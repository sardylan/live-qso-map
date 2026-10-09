#!/usr/bin/env bash
# Send N synthetic ADIF QSOs over UDP, one second apart in QSO time (feature 001 quickstart §6,
# feature 004 quickstart §3).
#
# Usage: scripts/send_burst.sh N [host] [port] [delay] [offset]
#   delay   seconds to wait between QSOs (default 0: as fast as possible)
#   offset  index of the first QSO (default 0); calls and QSO times continue from it, so a second
#           burst with offset N sends new QSOs instead of resends
set -euo pipefail

count="${1:?usage: $0 N [host] [port] [delay] [offset]}"
host="${2:-127.0.0.1}"
port="${3:-12060}"
delay="${4:-0}"
offset="${5:-0}"
fields=(A B C D E F G H I J K L M N O P Q R)

for ((n = 0; n < count; n++)); do
    i=$((offset + n))
    call="T$((i % 10))X$(printf '%04d' "$i")"
    time=$(printf '%02d%02d%02d' $(((10 + i / 3600) % 24)) $(((i / 60) % 60)) $((i % 60)))
    grid="${fields[$((i % 18))]}${fields[$(((i / 18) % 18))]}$((i % 10))$(((i / 10) % 10))"
    printf '<CALL:%d>%s <QSO_DATE:8>20261008 <TIME_ON:6>%s <BAND:3>20m <MODE:2>CW <GRIDSQUARE:4>%s <EOR>' \
        "${#call}" "$call" "$time" "$grid" | nc -u -w0 "$host" "$port"
    if [[ "$delay" != "0" ]]; then
        sleep "$delay"
    fi
done
echo "sent $count QSOs (from $offset) to $host:$port"
