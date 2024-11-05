#!/usr/bin/python3

import socket

if __name__ == "__main__":
    payload: bytes = (
        b'<?xml version="1.0"?>'
        b'<contactinfo>'
        b'<logger>QARTest 14.9.1</logger>'
        b'<contestname>CQ-WW-SSB</contestname>'
        b'<timestamp>2024-10-24 09:00:00</timestamp>'
        b'<mycall>XXXXXX</mycall>'
        b'<band>40</band>'
        b'<txfreq>0</txfreq>'
        b'<operator>YYYYYY</operator>'
        b'<mode>SSB</mode>'
        b'<call>N1CALL</call>'
        b'<countryprefix>N</countryprefix>'
        b'<wpxprefix>N0</wpxprefix>'
        b'<snt>59</snt>'
        b'<rcv>59</rcv>'
        b'<nr>1234</nr>'
        b'<exch1>41</exch1>'
        b'<exch2></exch2>'
        b'<exch3></exch3>'
        b'<duplicate>True</duplicate>'
        b'<stationname></stationname>'
        b'<points>0</points>'
        b'<id>123456789</id>'
        b'</contactinfo>'
    )

    sck = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sck.sendto(payload, ("127.0.0.1", 12060))
    sck.close()
