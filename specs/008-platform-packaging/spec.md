# Feature Specification: Platform Packages, Friendlier Command Line and Configuration Export

**Feature Branch**: `feature/008-platform-packaging`
**Created**: 2026-10-11
**Status**: Draft
**Input**: User description: "Add new spec for release in the various platforms. Prepare Cargo.toml
and other files for packaging in the various platforms. Linux: Ubuntu/Debian DEB package, if
possible a single package (if it has no OS dependencies and is compatible between libc and musl);
Docker image, based on Debian Trixie (or the official rust image, if any, and if it is a best
practice); Windows single exe file (or a zip file in case of multiple files); MacOS default release
for a command line application, but Keep It Simple, same as Windows. Ensure also that the clap
library has all the useful features enabled (like colors, long help, suggestion, and text
wrapping). Add also a feature to dump the config file to disk with the actual configuration
parameters, with also the possibility to just save the config file without running the whole
program. This feature is useful for generating a config file starting from a clean installation.
About packaging on Linux, follow the LSB principles for folder and files. Binary should go into
/usr/bin/, config files into /etc/live-qso-map/ and variable data (downloaded files or generated
ones) to /var/lib/live-qso-map/. Remember also to create a dedicated unprivileged user for running
the binary, and ensure that the control scripts (pre-inst, post-inst, pre-rm, post-rm, etc) manage
the user the right way, deleting it on purge, as per the best practices of Debian packages. On
Windows, everything should be done in the same folder in which the exe file is kept, because it is
mainly intended for double-click execution (which simply starts a terminal window with the
program), so no folder complication, for the moment. On MacOS follow the same principle of
Windows, for the moment, or suggest a better setup."

## Clarifications

### Session 2026-10-11

- Q: After installing the Debian package, should the service start at once? → A: No. It is
  installed but disabled and stopped; the operator configures it and enables it (FR-011).
- Q: In which registry is the container image published? → A: Docker Hub,
  `sardylan/live-qso-map` (FR-015).
- Q: Does the exported configuration include provider passwords? → A: Yes, always; the file is
  readable and writable by its owner only (FR-022).
- Q: (during planning) On Windows, how does the window stay open after an error without unsafe
  code? → A: The program waits for Enter after an error whenever its input is an interactive
  console, also when started from a terminal (FR-019).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Install and run as a service on Debian or Ubuntu (Priority: P1)

An operator with a Debian or Ubuntu machine (a shack PC, a Raspberry Pi, a small server) downloads
one package for the machine's architecture and installs it with the system's package tools. The
map service is installed in the standard system locations, runs under its own unprivileged
account, keeps its configuration in `/etc/live-qso-map/` and its downloaded data in
`/var/lib/live-qso-map/`, and is managed like any other system service. Upgrading keeps the
operator's configuration; removing stops the service; purging leaves nothing behind.

**Why this priority**: a Linux service is the most common way to run the map during a contest,
and today it takes manual copying, a hand-made account and a hand-edited service file.

**Independent Test**: on a clean Debian 13 and a clean Ubuntu 24.04 machine, install the package,
edit the configuration, restart the service, open the map page, send a QSO; then upgrade, remove
and purge, checking files, account and service state after each step.

**Acceptance Scenarios**:

1. **Given** a clean Debian or Ubuntu machine, **When** the operator installs the package for its
   architecture, **Then** the program is at `/usr/bin/live-qso-map`, a configuration file exists in
   `/etc/live-qso-map/`, `/var/lib/live-qso-map/` exists and belongs to a dedicated system account,
   and the service is registered with the system's service manager but neither enabled nor
   running, so no port is open.
2. **Given** the operator has edited the configuration, **When** they enable and start the service
   with the service manager, **Then** it starts now and at every boot.
3. **Given** the package is installed, **When** the service runs, **Then** it runs as the dedicated
   account (never as root), reads its configuration from `/etc/live-qso-map/`, and stores the
   downloaded country data in `/var/lib/live-qso-map/`.
4. **Given** the operator changed the configuration file, **When** a newer package is installed,
   **Then** the changed configuration is kept and a running service runs the new version; a
   disabled service stays disabled.
5. **Given** the package is installed, **When** the operator removes it (without purging),
   **Then** the service is stopped and the program is gone, while the configuration, the data
   directory and the dedicated account are kept.
6. **Given** the package is installed or removed, **When** the operator purges it, **Then** the
   configuration, `/var/lib/live-qso-map/` and the dedicated account are deleted.
7. **Given** the dedicated account already exists (for example from an earlier installation),
   **When** the package is installed again, **Then** installation succeeds and reuses the account.

---

### User Story 2 - Generate a configuration file from the current settings (Priority: P1)

An operator starting from a clean installation runs the program once with the options they want
(callsign, locator, ports, bands…) and asks it to write those settings to a configuration file,
without starting the map. The file contains every setting with its effective value, so it can be
edited later and used as the program's configuration. The operator can also ask the program to
write the file and then go on running normally.

**Why this priority**: it is the quickest way to get a correct, complete configuration file on
every platform, and the Linux package relies on it to describe every setting.

**Independent Test**: run the program with a few options and the "save only" request; check that
it exits at once without opening any network port, and that starting it again with only the
written file gives the same effective configuration.

**Acceptance Scenarios**:

1. **Given** options on the command line, in the environment and in an existing configuration file,
   **When** the operator asks to save the configuration to a file and exit, **Then** the file holds
   every setting with the value the program would use, and the program exits with success without
   listening on any port.
2. **Given** a configuration file written this way, **When** the program starts with only that file,
   **Then** its effective configuration is identical to the one that was saved.
3. **Given** the operator asks to write the configuration and keep running, **Then** the file is
   written first and the map then starts normally.
4. **Given** the operator names `-` as the destination, **Then** the configuration is printed to the
   standard output instead of a file.
5. **Given** the configuration is invalid (for example an unknown band), **When** the operator asks
   to save it, **Then** nothing is written and the program reports the same error it reports at
   start-up.
6. **Given** the destination cannot be written (missing directory, no permission), **Then** the
   program reports the path and the reason and exits with failure, without starting the map.

---

### User Story 3 - Run the map on Windows or macOS by double-clicking (Priority: P2)

An operator on Windows or macOS downloads one archive, extracts it to a folder of their choice and
double-clicks the program. A terminal window opens and the map starts. Everything the program
reads or writes (its configuration file, the downloaded country data) lives in that same folder,
whatever the current directory of the terminal.

**Why this priority**: many contest stations run Windows; a no-install, double-click start is the
simplest possible setup for them. macOS follows the same model for now.

**Independent Test**: on a clean Windows 11 machine and a clean macOS machine, extract the archive
to a new folder, double-click the program, open the map page, send a QSO, and check that the
country data was downloaded into that folder.

**Acceptance Scenarios**:

1. **Given** the extracted folder, **When** the operator double-clicks the program, **Then** a
   terminal window opens, the map starts with the built-in defaults and the page is reachable.
2. **Given** a configuration file with the documented name in the program's folder, **When** the
   program starts without a configuration option, **Then** it uses that file.
3. **Given** the program starts from any current directory, **When** it downloads the country data,
   **Then** the file is stored in the program's folder (unless the configuration names another
   location).
4. **Given** the program stops with an error on Windows (for example the port is already in use),
   **Then** the error stays readable in the window until the operator presses Enter.
5. **Given** the program runs on Windows with its input redirected (a script or a service
   wrapper), **When** it stops with an error, **Then** it exits at once without waiting.

---

### User Story 4 - Run the map as a container (Priority: P2)

An operator who runs containers starts the map from a published image, giving it a configuration
file and a persistent volume for its data. The container runs as an unprivileged user and uses the
same configuration and data locations as the Linux package.

**Why this priority**: containers are a common way to run services on home servers and cloud
machines, and an image already exists informally (the repository's container recipe).

**Independent Test**: start the image on an x86-64 and on an ARM64 host with a mounted
configuration file and a named volume; open the map, send a QSO, restart the container and check
the country data was kept.

**Acceptance Scenarios**:

1. **Given** the published image, **When** the operator starts it with the UDP and HTTP ports
   exposed, **Then** the map works with the built-in defaults.
2. **Given** a configuration file mounted at `/etc/live-qso-map/config.toml`, **Then** the program
   uses it; given a volume at `/var/lib/live-qso-map`, **Then** the country data survives a restart.
3. **Given** the container runs, **Then** its process does not run as root.

---

### User Story 5 - A command line that helps (Priority: P3)

An operator types the program's options in a terminal. The short help gives a one-screen summary;
the long help explains every option with its environment variable and default. Help text fits the
terminal width, uses colour when the terminal supports it, and a mistyped option gets a suggestion
of the closest valid one.

**Why this priority**: the program has more than 30 options; good help reduces configuration
mistakes before a contest, but the program works without it.

**Independent Test**: run the short and long help in a narrow and a wide terminal, with and without
colour, and mistype an option.

**Acceptance Scenarios**:

1. **Given** a terminal, **When** the operator asks for the short help, **Then** each option has a
   one-line description; **When** they ask for the long help, **Then** each option also shows its
   environment variable and default value.
2. **Given** a terminal 80 columns wide, **Then** no help line is longer than the terminal and text
   wraps at word boundaries.
3. **Given** a terminal that supports colour, **Then** help and errors use colour; **Given** output
   redirected to a file or the `NO_COLOR` environment variable set, **Then** no colour codes appear.
4. **Given** a mistyped option (for example `--udp-prot`), **Then** the error suggests the closest
   valid option (`--udp-port`).

---

### Edge Cases

- The dedicated account exists but was created by someone else with a login shell or a different
  home: installation must not change or break it, and purge must only delete an account the package
  created.
- The operator purges while the service is running: the service is stopped before files and the
  account are removed.
- A package upgrade while the service runs: the service is restarted on the new version without
  losing the configuration.
- `/var/lib/live-qso-map/` contains files the operator added: remove keeps them; purge deletes the
  whole directory, as documented.
- Installing the package on an architecture it was not built for: the package tools refuse it.
- Saving the configuration to the same file it was loaded from: the file is replaced only after the
  new content is complete, so an interruption never leaves a half-written file.
- Saving a configuration that holds provider passwords over the packaged
  `/etc/live-qso-map/config.toml`: the file keeps its owner, group and mode, so the service can
  still read it and other users cannot (FR-022).
- On Windows or macOS, the program's folder is read-only (for example extracted into a protected
  location): the country data download fails with a message naming the folder, as today for an
  unwritable path.
- A release tag whose version differs from the program's own version: no package is published.

## Requirements *(mandatory)*

### Functional Requirements

**Release artifacts**

- **FR-001**: Each release MUST provide, for every configured target, a ready-to-use artifact: a
  Debian package for each Linux architecture (x86-64, ARM64, 32-bit ARM), a container image, an
  archive for Windows (x64) and an archive for macOS (Apple silicon).
- **FR-002**: Every artifact MUST carry the program's version, which MUST equal the release tag's
  version (tag `v2.0.0` → version `2.0.0`; Debian packages add their revision, `2.0.0-1`). A
  mismatch MUST stop the release with nothing published.
- **FR-003**: Each release MUST publish a SHA-256 checksum for every downloadable artifact.
- **FR-004**: Every artifact MUST include the license text, as the GPL requires for binary
  distribution. Downloadable programs are therefore always inside a package or an archive, never
  a bare binary.

**Debian package**

- **FR-005**: There MUST be one Debian package per architecture that installs on every supported
  Debian (12, 13) and Ubuntu (22.04, 24.04 and later) release, with no dependency on the system's C
  library or other system packages beyond the base system and the service manager.
- **FR-006**: The package MUST install the program as `/usr/bin/live-qso-map`, its configuration in
  `/etc/live-qso-map/`, and use `/var/lib/live-qso-map/` for downloaded and generated data, following
  the Filesystem Hierarchy Standard.
- **FR-007**: The package MUST create a dedicated system account and group named `live-qso-map`, with
  no login shell and `/var/lib/live-qso-map` as home, when it does not already exist.
- **FR-008**: The service MUST run as that account, never as root, with the operating-system
  hardening the current service file already applies (no new privileges, read-only system, private
  temporary files, …).
- **FR-009**: The configuration file MUST be a package configuration file: upgrades keep the
  operator's changes, and the package tools ask before replacing a changed file.
- **FR-010**: The shipped configuration file MUST list every setting with its default value,
  commented, with the data location set to `/var/lib/live-qso-map/`. It MUST be readable by the
  service account and not by other users, because it may hold provider passwords.
- **FR-011**: Installation MUST register the service but leave it disabled and stopped, so nothing
  listens until the operator has configured it and enabled it with the service manager (the map's
  UDP port has no authentication, FR-035 of feature 001). After installation the package MUST
  print how to edit the configuration and enable the service.
- **FR-012**: Removing the package MUST stop and disable the service and remove the program, keeping
  the configuration, `/var/lib/live-qso-map/` and the account.
- **FR-013**: Purging MUST stop the service if running and delete `/etc/live-qso-map/`,
  `/var/lib/live-qso-map/` and the dedicated account and group, but only an account the package
  created.
- **FR-014**: Upgrading MUST restart a running service on the new version, and MUST NOT enable or
  start a service the operator left disabled.

**Container image**

- **FR-015**: The image MUST be based on Debian 13 "Trixie" (slim variant) and published for x86-64
  and ARM64 under the release version and `latest`, on Docker Hub as `sardylan/live-qso-map`.
- **FR-016**: The container MUST run as an unprivileged user, read `/etc/live-qso-map/config.toml`
  when present, keep its data in `/var/lib/live-qso-map/` (declared as a volume), and expose the UDP
  and HTTP ports.

**Windows and macOS**

- **FR-017**: The Windows and macOS releases MUST each be a single archive containing the program,
  the license and a short read-me that explains the double-click start and the configuration file.
- **FR-018**: When no configuration file is given, the program MUST use a file named
  `live-qso-map.toml` in its own folder if one exists. Relative data paths in the defaults MUST
  resolve against the program's folder on Windows and macOS, not the current directory.
- **FR-019**: On Windows, when the program exits with an error and its input is an interactive
  console, it MUST keep the message visible and wait until the operator presses Enter. This also
  applies when started from a terminal. When its input is not interactive (redirected, piped, a
  service wrapper), it MUST exit at once.

**Configuration export**

- **FR-020**: The program MUST offer two requests: write the effective configuration to a given path
  and exit ("save"), and write it then keep running ("dump"). The path `-` MUST mean the standard
  output.
- **FR-021**: The written file MUST contain every setting with the value the program would use,
  after combining command line, environment, configuration file and defaults, in the configuration
  file format, and loading it back MUST give an identical effective configuration.
- **FR-022**: The written file MUST include the provider usernames and passwords, whatever their
  source (command line, environment or file), so it is a complete, working configuration. On Linux
  and macOS, a new file MUST be readable and writable by its owner only; when an existing file is
  replaced, its owner, group and mode MUST be kept, as far as the writer is allowed to set them.
  The program MUST print a note that the file contains passwords when it holds any. Writing to the
  standard output (`-`) prints them too.
- **FR-023**: The configuration MUST be fully validated before anything is written; on an invalid
  configuration nothing is written and the error is the same as at start-up.
- **FR-024**: Writing MUST be atomic: an existing file at the path is replaced only once the new
  content is complete. Writing over an existing file MUST be allowed (the path is the operator's
  explicit choice).
- **FR-025**: The "save" request MUST NOT open any network port or start any background work.

**Command line**

- **FR-026**: The command line MUST offer a short help (`-h`) and a long help (`--help`); the long
  help MUST show each option's environment variable and default value.
- **FR-027**: Help and error output MUST wrap to the terminal width, use colour only when the output
  is a colour terminal and `NO_COLOR` is not set, and suggest the closest valid option for a
  mistyped one.

### Key Entities

- **Release artifact**: a downloadable or pullable output of a release — Debian package, container
  image, Windows archive, macOS archive — identified by target and version, with its checksum.
- **Dedicated account**: the system account and group `live-qso-map` that owns the data directory
  and runs the service on Linux.
- **Effective configuration**: the complete set of settings the program will use, after combining
  command line, environment, configuration file and defaults; what the export writes.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: On clean Debian 13 and Ubuntu 24.04 machines, an operator goes from download to a
  running map with their callsign in under 5 minutes, using only the package tools, a text editor
  and one service-manager command; until that command, no port is open.
- **SC-002**: After a purge, no file, directory, service or account created by the package remains
  on the system (0 leftovers).
- **SC-003**: Across an upgrade, 100% of the operator's configuration changes are kept.
- **SC-004**: A configuration file saved from any combination of options, loaded back, gives an
  identical effective configuration in 100% of the tested combinations.
- **SC-005**: On clean Windows 11 and macOS machines, an operator goes from download to a running map
  in under 2 minutes with no installer and no administrator rights.
- **SC-006**: The container starts and serves the map on both x86-64 and ARM64 hosts, and the
  country data survives a container restart.
- **SC-007**: In an 80-column terminal, no help line exceeds 80 columns; a single-letter typo in any
  option name gets a suggestion of the intended option.
- **SC-008**: Every release publishes all artifacts of FR-001 with checksums, or none of them.

## Assumptions

- **One package per architecture**: the Linux program is a static binary with no C-library
  dependency, so a single package per architecture works on every Debian and Ubuntu release;
  "single package" means no per-distribution variants, not one package for all architectures.
- **Container base**: the official Rust image is a build environment, not a runtime base; best
  practice is a small runtime image. The image therefore uses Debian 13 slim as requested, and
  contains the pipeline-built static binary (constitution Principle V: cross-compiling only in the
  pipeline). The current distroless recipe is replaced.
- **macOS**: kept as simple as Windows, as requested: an archive with the program for Apple
  silicon (the configured target), run from its folder. The program is not signed or notarised, so
  the first start needs the usual "Open anyway" confirmation, which the read-me explains. Better
  options for later are a Homebrew tap (`brew install`, which also handles updates) or a signed and
  notarised build, which needs a paid Apple developer account. Intel Macs are not a release target.
- **Windows**: x64 only, matching the configured target; no installer, no service, no registry use.
- **Service manager**: Linux packages target systemd, the default on all supported Debian and Ubuntu
  releases.
- **Configuration file name**: `config.toml` on Linux and in the container, `live-qso-map.toml` next
  to the program on Windows and macOS, so it is recognisable among the user's files.
- **Existing behaviour is unchanged otherwise**: option names, environment variables, precedence
  (command line > environment > file > defaults) and the configuration format stay as they are; the
  export adds requests, it does not rename settings.
- **Releases**: built only by the pipeline on release tags (constitution Principle V); developers
  build for their own host and do not produce packages locally.
