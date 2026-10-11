# Research: Platform Packages, Friendlier Command Line and Configuration Export

Feature: `008-platform-packaging` · Spec: [spec.md](spec.md) · Date: 2026-10-11

Versions checked on 2026-10-11:
- cargo-deb 3.8.0;
- clap 4.6.7;
- toml 1.1.7, already a dependency, including its writer;
- Debian 13 "Trixie".

## R1. Building the Debian package

- **Decision**: [cargo-deb](https://github.com/kornelski/cargo-deb) 3.8.0, configured in
  `[package.metadata.deb]` of `Cargo.toml`. The pipeline runs
  `cargo deb --target <triple> --no-build` on each binary that `cross` built (Principle V).
- **Rationale**:
  - The package description lives next to the crate metadata, as asked ("prepare Cargo.toml").
  - cargo-deb maps target triples to Debian architectures:
    - `x86_64…` → `amd64`;
    - `aarch64…` → `arm64`;
    - `armv7…musleabihf` → `armhf`.
  - It installs systemd units with the standard debhelper fragments, and treats files under
    `/etc` as conffiles.
  - It merges our maintainer scripts at the `#DEBHELPER#` token.
  - It is a CI tool only, not a crate dependency.
- **Alternatives**:
  - **A hand-written `debian/` folder built with `dpkg-buildpackage`**: the reference way, but it
    needs a Debian build host and repeats the version and metadata.
  - **nfpm**: works for any format, but adds a separate YAML file and a Go tool, and keeps the
    metadata away from `Cargo.toml`.

## R2. One package per architecture

- **Decision**: one `.deb` per Linux target (`amd64`, `arm64`, `armhf`), each holding the static
  musl binary. The only `Depends` is `adduser`, needed by the maintainer scripts. There is no C
  library dependency, so the same package installs on Debian 12 and 13 and on Ubuntu 22.04 and
  later.
- **Rationale**: a static binary needs nothing from the system C library, whether glibc or musl.
  A single package for all architectures (`Architecture: all`) is impossible for a native binary.
- **Alternatives**:
  - **glibc builds per distribution release**: more packages and real library dependencies, for
    no gain.

## R3. The service account

- **Decision**:
  - **Install** (`postinst configure`): create the system account and group `live-qso-map` when
    they are missing, with
    `adduser --system --group --home /var/lib/live-qso-map --no-create-home --shell /usr/sbin/nologin --gecos "live-qso-map service"`.
    An existing account is reused unchanged.
  - **Purge** (`postrm purge`), in this order:
    1. delete `/etc/live-qso-map/` and `/var/lib/live-qso-map/`;
    2. delete the account with `deluser --system`, but only if its GECOS field is still
       `live-qso-map service`, the marker of an account this package created;
    3. remove the group with `delgroup --system --only-if-empty`.
  - **Remove**: keeps the account, the configuration and the data.
- **Rationale**:
  - This is what the operator asked for: a dedicated, unprivileged account, removed on purge.
  - Debian policy discourages deleting system accounts, because a reused UID could own leftover
    files. Purge deletes the data folder first, so no file owned by the account remains.
  - The GECOS marker stops purge from deleting an account that someone else created.
- **Alternatives**:
  - **`systemd-sysusers`**: declarative, but it never removes accounts.
  - **systemd `DynamicUser=`** (today's unit): no lasting account at all, which the operator
    explicitly does not want.

## R4. The systemd unit and the "installed but disabled" state

- **Decision**:
  - **Unit file** `deploy/debian/live-qso-map.service`:
    - `User=` and `Group=live-qso-map`;
    - `ExecStart=/usr/bin/live-qso-map --config /etc/live-qso-map/config.toml`;
    - `StateDirectory=live-qso-map`;
    - the existing hardening (`NoNewPrivileges`, `ProtectSystem=strict`, `ProtectHome`,
      `PrivateTmp`…).
  - **cargo-deb**: `systemd-units = { enable = false, start = false, restart-after-upgrade = true }`.
  - **Upgrade**: `postinst` restarts the service only when it is running
    (`deb-systemd-invoke try-restart`), so a disabled service stays stopped (FR-014).
  - **First install**: at the end, `postinst` prints how to edit the configuration and run
    `systemctl enable --now live-qso-map` (FR-011).
- **Rationale**: the UDP port has no authentication, so nothing listens until the operator has
  chosen the configuration (clarification Q1).
- **To check during implementation**: the exact fragments cargo-deb generates with
  `start = false`. Our own `try-restart` call doesn't depend on them.

## R5. The configuration file in the package

- **Decision**:
  - **Source**: `deploy/config.example.toml` ships as `/etc/live-qso-map/config.toml`. Every
    setting is commented out with its default value, and the country data path is
    `/var/lib/live-qso-map/cty.dat`.
  - **Conffile**: the file is under `/etc`, so dpkg treats it as a conffile. Upgrades keep the
    operator's changes, and dpkg asks before replacing an edited file.
  - **Permissions**: `postinst` sets the file to `root:live-qso-map`, mode `0640`, unless an
    operator has registered a `dpkg-statoverride` for it.
  - **Sync test**: a unit test checks that every key of the exported configuration (R9) appears
    in the example. It also checks that the example, with comments removed, loads as the default
    configuration.
- **Rationale**: the hand-written comments explain each setting, which an export can't do. The
  test keeps the example from drifting away from the code (FR-010).
- **Alternatives**:
  - **Generate the conffile with `--save-config` at build time**: always in sync, but it has no
    explanations, and the target binary would have to run on the build machine.

## R6. Container image

- **Decision**:
  - **Base**: `debian:trixie-slim` at runtime.
  - **Release image** (`deploy/docker/Dockerfile.release`):
    - copies the pipeline-built static binary for each `TARGETARCH`, and runs nothing on the
      target platform;
    - a stage on `$BUILDPLATFORM` installs `ca-certificates` and creates `/etc/live-qso-map` and
      `/var/lib/live-qso-map`; the final stage copies them in;
    - the process runs as the numeric unprivileged user `10001:10001`, which needs no passwd
      entry;
    - `VOLUME /var/lib/live-qso-map`, with `LQM_CTY_PATH` pointing into it;
    - exposes ports `12060/udp` and `8080/tcp`;
    - the entry point reads `/etc/live-qso-map/config.toml` when present.
  - **Publishing**: `linux/amd64` and `linux/arm64`, tagged `sardylan/live-qso-map:<version>` and
    `:latest` on Docker Hub (clarification Q2).
  - **Local builds**: the repository `Dockerfile` stays, for building from source on any host. Its
    build stage moves to `rust:<pinned>-trixie`, with the same runtime base.
- **Rationale**:
  - The official Rust image is a build environment of more than 1 GB, not a runtime base.
  - Copying pipeline binaries keeps cross-compiling in one place (Principle V) and avoids emulating
    ARM during the build.
  - The TLS stack uses the system certificate store (`rustls-platform-verifier`), so the image
    needs `ca-certificates`.
- **Alternatives**:
  - **Keep distroless**: the operator asked for Debian Trixie.
  - **`RUN useradd` on each target platform**: needs QEMU emulation in CI.

## R7. Release pipeline (CircleCI, tags only)

- **Decision**: on `vX.Y.Z` tags, after the existing quality gates:
  1. `check-version`: the tag must equal `v` plus the `Cargo.toml` version, otherwise the
     workflow stops (FR-002).
  2. **Builds**: unchanged (`cross` for the three Linux targets, macOS, Windows).
  3. `package-deb`: cargo-deb on the three Linux binaries.
  4. `test-deb`: runs `scripts/test-deb.sh` in `debian:12`, `debian:13`, `ubuntu:22.04` and
     `ubuntu:24.04` containers (amd64). The script installs, upgrades (to the same package),
     removes, reinstalls and purges, and checks files, owners, modes and the account after each
     step.
  5. `package-archives`: the Windows and macOS zips.
  6. `docker-image`: `docker buildx build --platform linux/amd64,linux/arm64 --push`.
  7. `publish-github-release`: every file plus `SHA256SUMS` (FR-003). It runs only after every job
     above has succeeded (SC-008).

  New CircleCI project variables: `DOCKERHUB_USERNAME` and `DOCKERHUB_TOKEN` (a Docker Hub access
  token), next to `GITHUB_TOKEN`.
- **Rationale**: one job per artifact keeps failures easy to read, and publishing only after every
  job succeeds makes the release all-or-nothing.
- **Accepted risk**: the image is pushed before the GitHub release is published, so a later
  failure could leave an image without a release. Running `docker-image` in parallel saves time,
  because it is the slowest job. If this ever happens, the image push moves to the end.

## R8. Command-line help (clap)

- **Decision**:
  - **Features**: enable clap's `wrap_help` (wrap to the terminal width) and `unicode` (correct
    width for non-ASCII text). The default features stay on: `color`, `suggestions`,
    `error-context`, `help` and `usage`.
  - **Colours**: set `Command::styles` to a coloured palette for headers, literals and
    placeholders. clap's output layer (`anstream`) already honours `NO_COLOR` and plain output
    when not writing to a terminal.
  - **Short and long help**: each option's doc comment becomes a one-line summary, followed by a
    paragraph `Environment: LQM_X. Default: Y.`. clap shows the summary in `-h` and the full text
    in `--help` (FR-026).
  - **Tests**:
    - `Cli::command().term_width(80).render_long_help()` has no line longer than 80 columns;
    - `-h` contains no "Environment:" text;
    - `try_parse_from(["live-qso-map", "--udp-prot", "1"])` suggests `--udp-port`.
- **Rationale**: these are clap's own features, so no new crate is needed. The `[env: …]` and
  `[default: …]` text moves out of the one-line summaries and into the long help.
- **Alternatives**:
  - **clap's `env` feature**: it would also make clap read the variables, duplicating the
    precedence logic in `Config::load` (command line > environment > file > defaults).

## R9. Configuration export ("save" and "dump")

- **Decision**:
  - **Options**: `--save-config <PATH>` writes the file and exits with 0; `--dump-config <PATH>`
    writes it and then runs. They can't be used together, and `-` means standard output.
  - **Command line only**: neither has an `LQM_*` variable, since a variable would make every
    service start exit or rewrite its file.
  - **Order**: `main` loads and checks the configuration as today, then writes, then exits or
    continues. Nothing is bound before the write (FR-023, FR-025).
  - **Format**:
    - the file-format structs (`FileConfig` and the rest) gain `Serialize`, and are built from the
      effective `Config` (`From<&Config>`);
    - `toml::to_string_pretty` renders them;
    - a header comment gives the program version and the UTC date. When credentials are present,
      it also notes that the file contains passwords.
  - **Content**:
    - every setting, including defaults;
    - passwords, always (clarification Q3);
    - the band filter as the explicit list of considered bands, in frequency order (HR-2);
    - the full colour table under `[map.bands]`.
  - **Atomic write**: write to a temporary file in the same folder, then rename it over the
    target. On Unix the temporary file is created with mode `0600` before any byte is written. On
    Windows, `std::fs::rename` replaces an existing file.
  - **Round-trip test**: for a set of combinations (command line, environment, file),
    `load(export(load(x))) == load(x)` (SC-004).
- **Rationale**: reusing the file format guarantees that the export is a valid configuration
  file. Writing after the check means an invalid configuration never produces a file.
- **Alternatives**:
  - **A `config save` subcommand**: a breaking change for a command line that has no
    subcommands today.
  - **Leave defaults out of the export**: shorter, but then the file doesn't list every setting,
    which the operator asked for.

## R10. Configuration file next to the program; data paths on Windows and macOS

- **Decision**:
  - **Configuration file lookup**: after `--config` and `LQM_CONFIG`, `config_path` gains a last
    fallback: `live-qso-map.toml` in the folder of `std::env::current_exe()`, when that file
    exists. This applies on every platform; on Linux the packaged service always passes
    `--config`.
  - **Data paths**: on Windows and macOS (`cfg(any(windows, target_os = "macos"))`), a relative
    `country_data.path` resolves against the program's folder, not the current folder. Linux
    keeps today's behaviour, relative to the current folder.
  - **Fallback**: if `current_exe()` fails, the current folder is used, with a warning.
- **Rationale**: Finder starts a double-clicked program in the home folder, and a Windows
  shortcut can change the working folder. The program's folder is the one place that doesn't
  move (FR-018).
- **Alternatives**:
  - **Per-user folders** (`%APPDATA%`, `~/Library/Application Support`): the operator asked for
    everything in the program's folder, for now.

## R11. Keeping the Windows window open after an error

- **Decision**: on Windows only (`cfg(windows)` in `main.rs`), when the program exits with an
  error and `std::io::stdin().is_terminal()` is true, it prints
  `Press Enter to close this window.` and waits for one line on standard input. With input
  redirected or piped, it exits at once (FR-019, as clarified during planning).
- **Rationale**:
  - Uses only the standard library: no `unsafe`, no new crate, no lint exception.
  - Interactive input is true both for a double-click and for a terminal start, so terminal
    users press Enter once after an error. The operator accepted that cost.
- **Alternatives**:
  - **Ask Windows whether the console is shared** (`GetConsoleProcessList`), to wait only after
    a double-click: exact, but it needs an `unsafe` call. The project forbids inline lint
    exceptions, so it would need a separate helper crate; rejected by the operator.
  - **A `.cmd` launcher with `pause`**: operators double-click the `.exe`, so the error would
    still flash and vanish.
  - **Guessing from environment variables** (`PROMPT`, `WT_SESSION`): wrong in PowerShell and in
    other terminals.

## R12. Windows and macOS archives

- **Decision**: one zip per platform, `live-qso-map-<version>-windows-x64.zip` and
  `live-qso-map-<version>-macos-arm64.zip`. Each contains:
  - the program;
  - `LICENSE.md`;
  - a `README.txt` from `deploy/windows/` or `deploy/macos/`, which covers:
    - starting by double-click;
    - `live-qso-map.toml` next to the program;
    - `--save-config live-qso-map.toml`;
    - the map's address.

  The macOS read-me also explains the first-start confirmation for an unsigned program (System
  Settings → Privacy & Security → Open Anyway).
- **Rationale**:
  - The GPL requires the license text alongside the binary, so a bare `.exe` isn't enough
    (FR-004).
  - Both systems open zip files without extra software.
  - `zip` keeps the program's executable bit for macOS.
- **For later (macOS)**:
  - **A Homebrew tap**: `brew install sardylan/tap/live-qso-map`, which also handles updates.
  - **A signed and notarised build**: needs a paid Apple developer account.

## R13. Other release files

- **Decision**: the GitHub release also carries:
  - the three static Linux binaries, as today;
  - a `SHA256SUMS` file covering every downloadable file (FR-003).
- **Rationale**: the static binaries cost nothing extra and serve distributions other than
  Debian.
