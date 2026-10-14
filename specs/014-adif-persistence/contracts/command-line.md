# Contract: Command Line and Configuration Changes (feature 014)

Extends `specs/008-platform-packaging/contracts/command-line.md` and
`specs/001-live-qso-map/contracts/configuration.md`.

## New

| Setting | CLI | Environment | TOML | Default |
|---------|-----|-------------|------|---------|
| History file | `--history-file <PATH>` | `LQM_HISTORY_FILE` | `history_file` | `qso-history.adi` next to the country data |

### `--enrich-only [FILE]`

Loads `FILE` (or the configured history file), completes every record, writes it back in place
and exits. No UDP or HTTP listener is opened.

| Situation | Exit code | File |
|-----------|-----------|------|
| completed | 0 | rewritten |
| interrupted (Ctrl+C, SIGTERM) | 0 | rewritten with what was completed |
| missing or unreadable | 1 | untouched |
| save failed | 1 | untouched (previous content) |
| invalid configuration | 1 | untouched |

- Conflicts with `--save-config` (usage error, exit code 2). `--dump-config` still prints first.
- Uses the lookup credentials, `timeout_secs`, `provider_order`, `workers` and `[station]`.
- Ignores `bind_address`, ports and `bands` (every band is completed).

## Removed

| Setting | CLI | Environment | TOML | Behaviour now |
|---------|-----|-------------|------|---------------|
| History size | `--history-size` (hidden) | `LQM_HISTORY_SIZE` | `history_size` | accepted, ignored, warning: `history_size is no longer used: the history is not limited; remove it` |

- `--save-config`/`--dump-config` no longer write `history_size`; they write `history_file` only
  when it is set explicitly (the default depends on the country data path).
- `deploy/config.example.toml`: the `history_size` line is replaced by
  `# history_file = "/var/lib/live-qso-map/qso-history.adi"`.
