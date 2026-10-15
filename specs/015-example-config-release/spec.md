# Feature Specification: Example Configuration in Every Release

**Feature Branch**: `feature/015-example-config-release`
**Created**: 2026-10-15
**Status**: Draft
**Input**: User description: "Ensure that every release contains the example config file with every
parameter, including secrets and passwords (commented out, with a placeholder as value). Example
config file must have a different name (with the world "example" in the name) and must be included
with the documentation."

## Problem Summary

The project already keeps a commented example configuration in its source tree, but only the
Debian package carries it, and there it is installed as the live configuration file, not as a
reference. The Linux archives, the Windows and macOS zips and the container image contain no
example at all: an operator who downloads one of them has to find the source repository, or run
the program to write a file, to learn which settings exist.

The example is also incomplete where it matters most: the lookup provider passwords are shown as
empty values rather than as recognisable placeholders, and nothing checks that the credentials
stay listed when settings change.

After this feature, every release deliverable carries the same example configuration file, under a
name that contains "example" so it can never be mistaken for (or overwrite) a live configuration,
placed next to the documentation that ships with that deliverable. It lists every setting of the
configuration file, each one commented out, secrets included with an obvious placeholder value.
The same file is also published on its own, as a release download named `config.example.toml`.

## Clarifications

### Session 2026-10-15

- Q: Should the example also be a standalone download on the GitHub release page? → A: Yes:
  inside every deliverable and also attached to the GitHub release as its own deliverable, named
  `config.example.toml`.
- Q: What does the Debian package install as the live `/etc/live-qso-map/config.toml`? → A: A
  short live configuration with only the package-specific settings (country data location), and a
  comment pointing to the example in the package documentation folder; the example itself is no
  longer installed as the live file.
- Q (user direction, after planning): how do installed packages treat the two files? → A: Every
  deliverable carries the example, with "example" in its file name. In installed packages the
  example is always overwritten by the new version, with no question asked. The live
  configuration is a normal configuration file and follows the package manager's standard flow:
  replaced when the operator never changed it, and when both the operator and the new version
  changed it, the installation asks the operator what to do.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Find every setting in the downloaded release (Priority: P1)

An operator downloads any release deliverable (Linux archive, Windows zip, macOS zip, Debian
package, container image). Next to the license and readme they find the example configuration
file. They copy it to their configuration file name, uncomment the lines they need, replace the
placeholders with their own values, and start the program with it.

**Why this priority**: This is the whole request: the example must reach the operator in every
release, whatever the platform.

**Independent Test**: Unpack (or install, or inspect) each deliverable of a release build and check
that the example file is present, next to the documentation, with the expected name.

**Acceptance Scenarios**:

1. **Given** a Linux, Windows or macOS release archive, **When** the operator unpacks it, **Then**
   the example configuration file is in the same folder as the license (and the readme, where the
   archive has one).
2. **Given** the Debian package is installed, **When** the operator lists the package's
   documentation folder, **Then** the example configuration file is there, next to the readme and
   the copyright file.
3. **Given** the container image, **When** the operator lists its documentation folder, **Then**
   the example configuration file is there, next to the license.
4. **Given** any deliverable, **When** the operator compares its example file with the one of
   another deliverable of the same release, **Then** the contents are identical.
5. **Given** the GitHub release page, **When** the operator looks at its downloads, **Then**
   `config.example.toml` is listed on its own and can be read without downloading any package.

---

### User Story 2 - Secrets shown with placeholders (Priority: P1)

An operator wants to enable a callsign lookup provider. In the example file they find the
provider's username and password lines, commented out, each with a placeholder value that is
obviously not a real credential. They uncomment both lines and type their own credentials over
the placeholders.

**Why this priority**: The user explicitly asked for secrets and passwords to be listed; without
them the operator cannot discover how to enable the providers.

**Independent Test**: Read the example file and check that every credential setting of every
lookup provider appears commented out with a placeholder value.

**Acceptance Scenarios**:

1. **Given** the example file, **When** the operator searches for a provider's credentials,
   **Then** both its username and its password lines are present, commented out, with a
   placeholder value.
2. **Given** the example file copied unchanged as a live configuration, **When** the program
   starts with it, **Then** it starts with the built-in defaults and no lookup provider is
   enabled (nothing is active until uncommented).
3. **Given** the example file, **When** the operator reads any placeholder, **Then** it is clear
   from the value itself that it must be replaced (for example a value naming what goes there).

---

### User Story 3 - The example cannot fall behind (Priority: P2)

A developer adds a new setting, or a new lookup provider, to the configuration. The automated
checks fail until the example file lists the new setting, so no release can ship an example that
is missing a setting.

**Why this priority**: "Every parameter" has to remain true release after release; without a guard
the example silently drifts.

**Independent Test**: Remove one setting (a password, for instance) from the example file and run
the project's checks: they fail and name the missing setting.

**Acceptance Scenarios**:

1. **Given** a setting missing from the example file, **When** the project's checks run, **Then**
   they fail and name the missing setting, credentials included.
2. **Given** a release build, **When** a deliverable lacks the example file, **Then** the release
   pipeline fails before anything is published.

---

### Edge Cases

- An operator unpacks a release archive into a folder that already holds their live configuration:
  nothing is overwritten, because the example file has a different name from any live
  configuration file.
- An operator upgrades the Debian package after editing the live configuration: if the new
  version ships the same live configuration as before, the operator's file is kept with no
  question; if the new version changed it too, the installation asks the operator whether to keep
  their file or take the new one (and offers to show the difference). The example in the
  documentation folder is always replaced by the new release's one, with no question.
- An operator edited the installed example: the upgrade overwrites it without asking, because the
  example is documentation, not configuration.
- An operator who already has a live configuration installed by an earlier package (a copy of the
  old example) upgrades: if they never edited it, it is replaced by the new short file with no
  question; if they edited it, the installation asks them, because the new version changed it too.
  Whatever they answer, the service keeps the same country data location and keeps working.
- An operator uncomments a credential line but forgets to replace the placeholder: the provider is
  enabled with the placeholder, the lookup fails as with any wrong credential, and the program
  reports it the way it reports any failed provider login; it does not crash.
- A setting whose default depends on the platform or the installation (for example where the
  country data and the QSO history are stored): the example shows it commented out, with a comment
  explaining what the default is, rather than one platform's path presented as the only value.
- Options that exist only on the command line (writing or dumping the configuration, enrichment
  only, the configuration file path itself) and options kept only for compatibility and ignored:
  they are not configuration file settings and are not listed.
- Per-band colours, whose keys are band names chosen by the operator: the example shows at least
  one commented entry illustrating the form, not one line per band.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The project MUST keep exactly one example configuration file, whose name contains the
  word "example" and differs from the live configuration file name used by any deliverable.
- **FR-002**: The example file MUST list every setting the configuration file accepts, in every
  section, including the username and password of every lookup provider.
- **FR-003**: Every setting in the example file MUST be commented out, so that the example used
  unchanged as a configuration behaves exactly like having no configuration file.
- **FR-004**: Every secret (provider passwords) and every other setting without a meaningful
  default (provider usernames, station identity) MUST be shown with a placeholder value that is
  recognisably not real and says what goes there; settings with a default MUST show that default.
- **FR-005**: The example file MUST explain, in its comments, how to use it: copy it to a live
  configuration file, uncomment, replace placeholders; and where secrets can be supplied instead
  (environment variables).
- **FR-006**: Every release deliverable MUST contain the example file: each Linux release archive,
  the Windows zip, the macOS zip, the Debian package and the container image.
- **FR-006a**: Every GitHub release MUST also offer the example file as a deliverable of its own,
  named exactly `config.example.toml` (no version or platform in the name).
- **FR-007**: In each deliverable the example file MUST sit with the documentation that deliverable
  ships (the license, and the readme where there is one): the archive root for the archives and
  zips, the package documentation folder for the Debian package and the container image.
- **FR-008**: The example file MUST be identical in every deliverable of the same release,
  the standalone `config.example.toml` included.
- **FR-009**: The readmes shipped with the deliverables and the project readme MUST mention the
  example file and how to start from it.
- **FR-010**: The Debian package MUST install a working live configuration file on first install.
  On upgrade the live configuration MUST follow the package manager's standard
  configuration-file flow: replaced without a question when the operator never changed it; kept
  without a question when the operator changed it and the new version did not; and, when both
  changed it, the installation MUST ask the operator (keep theirs, take the new one, or see the
  difference). The package MUST NOT rewrite the live configuration by any other means.
- **FR-010a**: The Debian package's live configuration MUST be a short file, distinct from the
  example, holding only the settings the packaged service needs (the country data location under
  the package's data folder) plus a comment pointing to the example in the package documentation
  folder. The example MUST NOT be installed as, or copied into, the live configuration.
- **FR-010b**: In an installed package the example file MUST be an ordinary packaged file, not a
  configuration file: every install and upgrade overwrites it with the new version's example
  without asking, whatever the operator did to it.
- **FR-011**: The project's automated checks MUST fail, naming the setting, when the example file
  lacks any configuration file setting, credentials included, or when the example used unchanged
  does not load as a valid configuration.
- **FR-012**: The release pipeline MUST verify, for every deliverable, that the example file is
  present at its documentation location, and that the standalone `config.example.toml` is among
  the files to publish, before publishing; a missing file MUST stop the release.

### Key Entities

- **Example configuration file**: the reference document listing every configuration file setting,
  commented out, with defaults or placeholders and explanatory comments; one per release, the same
  in every deliverable.
- **Live configuration file**: the file the program actually reads; written by the operator (or by
  the Debian package on first install); never replaced by the example.
- **Release deliverable**: a downloadable or installable output of a release — Linux archive,
  Windows zip, macOS zip, Debian package, container image, and the standalone
  `config.example.toml`.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a release build, 100% of the deliverables (every Linux archive, both zips, the
  Debian package, the container image) contain the example file at their documentation location,
  and the GitHub release lists `config.example.toml` as a download of its own.
- **SC-002**: 100% of the configuration file settings, including every provider's username and
  password, appear in the example file; removing any one of them makes the project's checks fail.
- **SC-003**: The example file used unchanged as a configuration starts the program with exactly
  the built-in defaults and zero lookup providers enabled.
- **SC-004**: An operator can go from a freshly unpacked release to a configuration with one lookup
  provider enabled by editing only the example file copy (no other source consulted) in under 5
  minutes.
- **SC-005**: No release deliverable overwrites or replaces an existing live configuration file.

## Assumptions

- The existing example (`config.example.toml` in the deployment folder) is the file to complete and
  ship; its name already satisfies the "example" requirement, so it is kept.
- "Every parameter" means every setting of the configuration file. Command-line-only options and
  options accepted only for compatibility are out of scope.
- "Included with the documentation" means inside each deliverable, next to the license and readme
  it already carries; in addition the file is published on the release page on its own
  (Clarifications, 2026-10-15).
- Placeholders are written in a single recognisable style (for example `"<your QRZ.com password>"`)
  across the file.
- The Debian package's first-install live configuration is a separate short file, not derived from
  the example (Clarifications, 2026-10-15); the service's behaviour (country data and history under
  the package's data folder) is unchanged.
- No change to the program's behaviour, configuration format or settings is part of this feature.
