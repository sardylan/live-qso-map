# Contract: Configuration (delta to feature 001)

A new setting. Everything else in `specs/001-live-qso-map/contracts/configuration.md` is unchanged.

| TOML key | CLI flag | Env var | Default | Notes |
|----------|----------|---------|---------|-------|
| `bands` | `--bands` (comma-separated, repeatable) | `LQM_BANDS` (comma-separated) | none (all bands, no filter) | entries are ADIF bands, groups, or either prefixed with `-` to exclude; a higher-precedence layer replaces the whole list |

Groups: `lf-mf`, `hf`, `hf-contest`, `warc`, `vhf`, `uhf`, `shf`, `ehf` (see `../data-model.md`).

## Validation (startup)

The service exits with a non-zero status and `invalid setting bands: …` when:

- an entry is neither an ADIF band nor a group, for example `invalid setting bands: unknown band or
  group 'hf-dx'`;
- the effective set is empty, for example `invalid setting bands: selection 'warc,-warc' leaves no
  bands`.

## Startup log (info)

```text
bands considered: 160m 80m 40m 20m 15m 10m
bands considered: all
```

## Examples

```toml
bands = ["hf-contest"]                 # 160m 80m 40m 20m 15m 10m
bands = ["hf", "2m", "-60m"]           # 160m 80m 40m 30m 20m 17m 15m 12m 10m 2m
bands = ["-60m"]                       # every band except 60m
```
