#!/usr/bin/env bash
# Send N synthetic ADIF QSOs over UDP, one second apart in QSO time (quickstart §6).
#
# Usage: scripts/send_burst.sh N [host] [port]
set -euo pipefail

count="${1:?usage: $0 N [host] [port]}"
host="${2:-127.0.0.1}"
port="${3:-12060}"
fields=(A B C D E F G H I J K L M N O P Q R)

for ((i = 0; i < count; i++)); do
    call="T$((i % 10))X$(printf '%04d' "$i")"
    time=$(printf '%02d%02d%02d' $((10 + i / 3600)) $(((i / 60) % 60)) $((i % 60)))
    grid="${fields[$((i % 18))]}${fields[$(((i / 18) % 18))]}$((i % 10))$(((i / 10) % 10))"
    printf '<CALL:%d>%s <QSO_DATE:8>20261008 <TIME_ON:6>%s <BAND:3>20m <MODE:2>CW <GRIDSQUARE:4>%s <EOR>' \
        "${#call}" "$call" "$time" "$grid" | nc -u -w0 "$host" "$port"
done
echo "sent $count QSOs to $host:$port"
