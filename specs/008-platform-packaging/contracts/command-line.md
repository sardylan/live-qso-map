# Contract: Command Line (feature 008)

Extends the command line of feature 001 (`contracts/configuration.md`). Option names, `LQM_*`
variables and precedence are unchanged.

## New options

| Option | Argument | Behaviour | Environment variable |
|--------|----------|-----------|----------------------|
| `--save-config <PATH>` | a file path, or `-` for standard output | load and validate the configuration, write it (contracts/configuration.md), exit with status 0 | none (command-line only) |
| `--dump-config <PATH>` | a file path, or `-` for standard output | load and validate the configuration, write it, then run normally | none (command-line only) |

- The two options exclude each other. Giving both is a usage error (exit status 2).
- Neither option binds a port or starts background work before the file is written.
- `--save-config` never binds a port.

## Exit status

| Situation | Status | Output |
|-----------|--------|--------|
| `--save-config` written | 0 | `configuration written to <PATH>` on standard error (nothing when `-`); `note: <PATH> contains provider passwords` when it holds any |
| invalid configuration | 1 | the same message as at start-up; nothing written |
| destination not writable | 1 | `cannot write configuration file <PATH>: <reason>` |
| usage error (unknown option, both options, missing argument) | 2 | clap's error with a suggestion when one exists |
| run ends with an error | 1 | the error; on Windows with interactive input, then `Press Enter to close this window.` |

## Help

| Request | Content |
|---------|---------|
| `-h` | usage line and one line per option: its summary only |
| `--help` | usage, the program description, and for each option its summary plus a paragraph `Environment: LQM_X. Default: Y.` (when it has a variable and a default) |
| `--version` | `live-qso-map <version>` |

- **Width**: lines wrap at word boundaries to the terminal width. When the width is unknown, they
  wrap at 100 columns.
- **Colour**: headers, option names and placeholders are coloured only when the stream is a
  terminal that supports colour. `NO_COLOR` (any value) and redirected output disable colour.
  This also applies to errors.
- **Suggestions**: an unknown option within a small edit distance of a valid one ends with
  `tip: a similar argument exists: '--udp-port'`.

## Windows pause

On Windows only: when the program exits with an error and its standard input is an interactive
console, it prints `Press Enter to close this window.` and waits for one line. With redirected or
piped input it exits at once. Other platforms never wait.
