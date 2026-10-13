# Contract: Band Setting Default (feature 011)

It extends feature 003's band setting. Precedence is unchanged: command line, then environment,
then file, then the default.

| Source | Value | Considered bands |
|--------|-------|------------------|
| none (no `bands`, no `--bands`, no `LQM_BANDS`) | — | **`hf-contest`**: 160m, 80m, 40m, 20m, 15m, 10m (the new default) |
| file | `bands = ["all"]` | every band |
| command line | `--bands all` | every band |
| environment | `LQM_BANDS=all` | every band |
| any | `all` together with other entries (`["all", "2m"]`, `all,-60m`) | error: `invalid setting bands: "all" can't be combined with other entries` |
| any | any other value | as in feature 003 (groups, bands, `-exclusions`) |

**Startup log** (feature 003, FR-006):
- with a band list: `bands considered: 160m, 80m, 40m, 20m, 15m, 10m`, in frequency order;
- with `all`: `bands considered: all`.

**Export** (`--save-config` / `--dump-config`, feature 008): `bands` is always written, as the
considered bands in frequency order, or `["all"]`. Loading the exported file gives the same set.

**`deploy/config.example.toml`**: the commented `bands` line says that the default is the six
contest HF bands, and that `["all"]` gives every band.
