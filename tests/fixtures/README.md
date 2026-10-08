# Test fixtures

| Path | Content | Source |
|------|---------|--------|
| `cty-excerpt.dat` | AD1C cty.dat entity blocks, copied verbatim except for a few exact-call entries replaced with fictitious calls: China, Fed. Rep. of Germany, Switzerland, Italy, Sardinia, Sicily, United States, Hawaii (T046) | `https://www.country-files.com/cty/cty.dat`, Last-Modified 2026-09-15 |
| `cty-overrides.dat` | **synthetic** entities exercising `<lat/lon>`, `{cont}`, `~utc~`, `(cq)`, `[itu]` and `=EXACT` overrides, which the current AD1C file does not use | hand-made from the CTY format description |
| `qrz/`, `hamqth/`, `qrzcq/` | provider XML responses (T047) | reconstructed from the providers' published API documentation, with fictitious data |
| `malformed/` | malformed-input corpus (T085): truncated/overflowing/negative/huge ADIF lengths, binary noise, empty datagram, half and invalid QARTest XML, 65,000 `<`, invalid UTF-8, bad date | hand-made; `binary_noise.bin` from /dev/urandom |
