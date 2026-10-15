---
description: "Task list for 015-example-config-release"
---

# Tasks: Example Configuration in Every Release

**Input**: Design documents from `/specs/015-example-config-release/`
**Prerequisites**: plan.md, spec.md, research.md (R1–R7), data-model.md,
contracts/example-config.md, contracts/release-deliverables.md, quickstart.md

**Tests**: MANDATORY (constitution Principle I). Write each test task before the change it covers,
run it, and confirm it fails first.
- **Rust tests**: `cargo test config_export`.
- **CI-only shell checks** (`scripts/*.sh`, `.circleci/config.yml`): confirm the failure on a
  Linux host or in a rehearsal (see each task).

**Conventions:**

- Rust tests go in the existing `#[cfg(test)] mod tests` of `src/config_export.rs`, in the section
  `// --- shipped configuration, deploy/config.example.toml ---`. Reuse its helpers:
  - `EXAMPLE` (`include_str!` of the example);
  - `uncommented_example()`, `keys()`, `rendered(&Config)`;
  - `load(args, env, file)` at line ~112.
- No inline lint suppressions (LL-001). `unwrap`/`expect` are fine in tests only.
- Shell scripts are POSIX `sh` with `set -u`. They use the `fail()`/`step=` style of
  `scripts/test-deb.sh`.
- The example's placeholders are exactly those of `contracts/example-config.md` rule 4. The
  placeholder keys are:
  - `station.callsign`, `station.locator`, `station.latitude`, `station.longitude`;
  - `lookup.qrz.username`, `lookup.qrz.password`;
  - `lookup.hamqth.username`, `lookup.hamqth.password`;
  - `lookup.qrzcq.username`, `lookup.qrzcq.password`.
- `.circleci/config.yml` changes are only proven by a release rehearsal (T029, LL-007).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story the task belongs to (US1–US3)
- **[R:TIER]**: risk tier (default GREEN)

---

## Phase 1: Setup

- [X] T001 Confirm the baseline on `feature/015-example-config-release`. `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings` and `cargo test` pass. Note the four existing
  example tests in `src/config_export.rs`:
  - `example_lists_every_exported_setting`
  - `uncommented_example_loads`
  - `example_sets_the_packaged_data_path`
  - the `keys` helper

---

## Phase 2: Foundational

No shared prerequisite: the Debian live file belongs to US1, the example content to US2, and the
completeness guard to US3.

**Checkpoint**: user story work can begin.

---

## Phase 3: User Story 1 - Find every setting in the downloaded release (Priority: P1) 🎯 MVP

**Goal**: every release file carries `config.example.toml` next to its documentation:
- the three Linux `.tar.gz`, the Windows and macOS zips;
- `/usr/share/doc/live-qso-map/` in the three `.deb` and the container image;
- a standalone `config.example.toml` on the GitHub release.

The Debian package gets its own short live `config.toml`. The example is a plain doc file,
overwritten on every upgrade. The live file is the only conffile.

**Independent Test**: a release rehearsal (T029):
- `rehearsal-summary` lists `config.example.toml` and `check-release-files.sh` passes;
- `package-deb` passes its doc-path and conffiles checks;
- `image-test` passes its `example` step.

### Tests for User Story 1 (write first, confirm they FAIL)

- [X] T002 [P] [US1] Add `const DEBIAN_LIVE: &str = include_str!("../deploy/debian/config.toml");`
  and three tests in `src/config_export.rs`. Confirm they fail: the file does not exist yet, so
  the crate does not compile.
  - `debian_live_file_loads`: `load(&[], &[], Some(DEBIAN_LIVE))` succeeds.
  - `debian_live_file_sets_the_packaged_paths`: `country_data.path` is
    `/var/lib/live-qso-map/cty.dat`, and the derived `history_file` is
    `Some("/var/lib/live-qso-map/qso-history.adi")`.
  - `debian_live_file_points_to_the_example`: the text contains
    `/usr/share/doc/live-qso-map/config.example.toml`.
- [X] T003 [P] [US1] [R:YELLOW] Create `scripts/check-release-files.sh` exactly as
  `contracts/release-deliverables.md` "`scripts/check-release-files.sh`" describes.
  - Usage: `<release-dir> <example>`.
  - Checks: standalone `cmp`; at least one each of `*.tar.gz`, `*.zip` and `*.deb`; then
    `tar -xzOf`, `unzip -p` and `dpkg-deb --fsys-tarfile | tar -xOf -
    ./usr/share/doc/live-qso-map/config.example.toml`, each compared with `cmp`.
  - Output: one `ok <file>` line per file; `FAIL [<file>]: <reason>` and exit 1 on the first
    failure.

  Confirm it fails on a Linux host with a scratch release dir holding today's layout:
  - a `.tar.gz` of `live-qso-map` + `LICENSE.md`;
  - a `.zip` of `LICENSE.md` + `README.txt`;
  - no standalone file.

  Expect `FAIL [config.example.toml]`, and after adding the standalone copy,
  `FAIL [<the tar.gz>]`.

  Where to run what:
  - The standalone, `.tar.gz` and `.zip` checks need only `tar`, `unzip` and `cmp`. Run them
    locally on any host, macOS included, against the scratch dir.
  - The `.deb` check needs `dpkg-deb`, which a macOS development host lacks. Prove it on a
    Linux host if one is available. Otherwise, start an early rehearsal right after T006, before
    T008. In it, `package-deb` and the script must fail on the missing doc path. The final
    rehearsal (T029) then shows them passing.
- [X] T004 [P] [US1] [R:YELLOW] Add a step `example` to `scripts/test-image.sh`, after `user`.
  `docker run --rm --entrypoint test "$image" -f /usr/share/doc/live-qso-map/config.example.toml`
  must succeed, else `fail "the example is not in /usr/share/doc/live-qso-map"`. It fails against
  today's image: confirm in the rehearsal (T029), or locally with `docker build -f Dockerfile .`
  where Docker is available.
- [X] T005 [US1] [R:YELLOW] Extend the "Check architecture and contents" step of `package-deb` in
  `.circleci/config.yml`:
  - Add `./usr/share/doc/live-qso-map/config.example.toml` to the `for path in …` list.
  - Add a conffiles check:
    `conffiles=$(dpkg-deb --ctrl-tarfile "$deb" | tar -xOf - ./conffiles)` must equal exactly
    `/etc/live-qso-map/config.toml`, else
    `echo "conffiles: $conffiles"; exit 1`.

  It fails today, because the doc path is missing.
- [X] T006 [US1] [R:YELLOW] In `.circleci/config.yml`, run
  `sh scripts/check-release-files.sh ws/release deploy/config.example.toml` as the first command
  of two jobs. It fails today, because there is no standalone file.
  - `publish-github-release`: needs a new step after the gh install, before "Checksums".
  - `rehearsal-summary`: add a `checkout` step; it currently only attaches the workspace.

### Implementation for User Story 1

- [X] T007 [US1] Create `deploy/debian/config.toml` (data-model "Debian live configuration file").
  - A header comment:
    - this is the live configuration of the Debian package;
    - every setting, commented out, is in `/usr/share/doc/live-qso-map/config.example.toml`;
    - a complete file can be written with
      `sudo live-qso-map --config /etc/live-qso-map/config.toml … --save-config /etc/live-qso-map/config.toml`;
    - it may hold provider passwords and is readable by root and the service only.
  - Then `[country_data]` with `path = "/var/lib/live-qso-map/cty.dat"`.

  T002 passes.
- [X] T008 [US1] [R:YELLOW] In `Cargo.toml` `[package.metadata.deb] assets`:
  - replace `["deploy/config.example.toml", "etc/live-qso-map/config.toml", "640"]` with
    `["deploy/debian/config.toml", "etc/live-qso-map/config.toml", "640"]`;
  - add `["deploy/config.example.toml", "usr/share/doc/live-qso-map/config.example.toml", "644"]`
    after the README line.

  Do not add a `conf-files` key: cargo-deb marks only `etc/` files as conffiles (research R3).
  T005 now holds.
- [X] T009 [P] [US1] Add
  `COPY deploy/config.example.toml /usr/share/doc/live-qso-map/config.example.toml` after the
  `LICENSE.md` COPY in two files. T004 passes.
  - `deploy/docker/Dockerfile.release` (line ~26);
  - `Dockerfile` (line ~35), for local build parity.
- [X] T010 [US1] [R:YELLOW] In `.circleci/config.yml` `build-linux`, step "Archive with the
  license":
  - add `cp deploy/config.example.toml stage/`;
  - add `config.example.toml` to the `tar -czf … -C stage …` file list;
  - rename the step "Archive with the license and the example".
- [X] T011 [US1] [R:YELLOW] In `.circleci/config.yml` `package-archives`, step "Zip":
  - copy `deploy/config.example.toml` into `win/` and `mac/`;
  - add `config.example.toml` to both `zip -X …` file lists;
  - add `cp deploy/config.example.toml ws/release/config.example.toml` (the standalone download).

  Update the job comment to "program, license, read-me, example; plus the standalone example".
  The existing `persist_to_workspace` path `release/*.zip` must also persist the new file: add
  `release/config.example.toml` to its `paths`. T006 now holds for the archives.
- [X] T012 [P] [US1] In `deploy/debian/postinst`, change the first-install message step 1. Point
  to `/usr/share/doc/live-qso-map/config.example.toml` as the list of every setting, keeping the
  `--save-config` alternative. The script still never writes the content of `$CONFIG`.
- [X] T013 [P] [US1] In `deploy/windows/README.txt` section 3 "Settings", add before the
  `--save-config` paragraph:
  - `config.example.toml` in this folder lists every setting, commented out;
  - copy it to `live-qso-map.toml`, uncomment what you change, and replace the `<…>` placeholders.
- [X] T014 [P] [US1] Same change in `deploy/macos/README.txt`.
- [X] T015 [P] [US1] Update `README.md`.
  - "Configuration" (line ~183): describe `deploy/config.example.toml` as the complete commented
    list. Say it ships as `config.example.toml` in every release file and as its own release
    download.
  - "Deployment":
    - container: `/usr/share/doc/live-qso-map/config.example.toml` in the image, and how to start
      a mounted `config.toml` from it;
    - Debian **Layout**: the short live file, plus the example in `/usr/share/doc/live-qso-map/`;
    - Debian upgrades: the example is always replaced; the live file follows dpkg's
      configuration-file prompt when both sides changed it, so the upgrade to this version asks
      once if you edited it;
    - Windows/macOS **One folder**: start `live-qso-map.toml` from the bundled
      `config.example.toml`.
- [X] T016 [P] [US1] Update `docs/releasing.md` "The release chain" step 3. It now reads: the
  GitHub release, with the changelog notes, `SHA256SUMS` and the standalone
  `config.example.toml`, after `check-release-files.sh` has confirmed the example in every archive
  and package.

**Checkpoint**: US1 is complete; T029 verifies it in CI.

---

## Phase 4: User Story 2 - Secrets shown with placeholders (Priority: P1)

**Goal**: the example has every line commented out, shows the defaults, and gives every setting
without a default a `"<…>"` placeholder, provider credentials included. Used unchanged, it is the
same as no file.

**Independent Test**: `cargo test config_export` passes the four tests below. The quickstart
`diff` of `--save-config -` with and without `--config deploy/config.example.toml` is empty.

**Depends on**: T007–T008. The Debian live file must carry the packaged path before the example
stops setting it.

### Tests for User Story 2 (write first, confirm they FAIL)

- [X] T017 [P] [US2] In `src/config_export.rs`, add `example_unchanged_sets_nothing`. Confirm it
  fails today, because of the uncommented `country_data.path`.
  - `toml::from_str::<toml::Table>(EXAMPLE)` is an empty table.
  - `rendered(&load(&[], &[], Some(EXAMPLE)))` equals `rendered(&load(&[], &[], None))`.
- [X] T018 [P] [US2] In `src/config_export.rs`, add `example_uses_placeholders_where_there_is_no_default`.
  Confirm it fails today, because the values are `""` and `"IS0MZK"`.
  - Parse `uncommented_example()`.
  - For each placeholder key (Conventions), walk the table by its dotted path. The value must be
    a string that starts with `<` and ends with `>`.
  - The message names the key.
- [X] T019 [US2] In `src/config_export.rs`, add `const PLACEHOLDER_SAMPLES: &[(&str, &str)]`. It
  maps each placeholder key's last segment to a valid TOML value:
  - `callsign` → `"IS0MZK"`, `locator` → `"JM49NH"`;
  - `latitude` → `39.3125`, `longitude` → `9.125`;
  - `username` → `"user"`, `password` → `"secret"`.

  Add `fn sampled_example() -> String`, which does two things:
  - It takes `uncommented_example()` and replaces the value of every line whose value is a
    `"<…>"` string with the sample for its key.
  - It drops the raster-tile alternative lines (`tile_url`, `tile_attribution`).

  Change `uncommented_example_loads` to load `sampled_example()`, keeping its current filter. It
  still passes today (no placeholders yet). It must keep passing after T022.
- [X] T020 [US2] In `src/config_export.rs`, add `example_shows_the_defaults`. Confirm it fails
  today: `bands = ["all"]` differs from the default, and `country_data.path` is the Debian path.
  - Take `sampled_example()` without the `[map.bands]` entries (labelled illustrations), and
    without `style_url` and `style_attribution`. Setting them explicitly to the defaults gives the
    same base map, but `Config::default()` leaves both `None`, so `rendered` would differ.
  - Load it with `load(&[], &[], Some(&text))`.
  - Compare `rendered(…)` with `rendered(&expected)` after dropping the first (timestamp) line.
    `expected` is `Config::default()` with the samples set:
    - station: callsign IS0MZK, locator JM49NH, 39.3125 / 9.125;
    - `user`/`secret` credentials for all three providers.

  Build `expected` through `load(&[...flags...], &[], None)` with the matching `--station-*` and
  `--<provider>-username/--<provider>-password` flags, so defaults are derived the same way.

  In the same test, also check the style defaults. Parse `uncommented_example()`: `map.style_url`
  must equal `DEFAULT_STYLE_URL` and `map.style_attribution` must equal
  `DEFAULT_STYLE_ATTRIBUTION`. This fails today, because the example shows `"..."`.

### Implementation for User Story 2

- [X] T021 [US2] Rewrite `deploy/config.example.toml` per `contracts/example-config.md`.
  - Header: illustrative wording in the contract.
  - Sections in exported order: top level, `[station]`, `[lookup]`, `[lookup.qrz]`,
    `[lookup.hamqth]`, `[lookup.qrzcq]`, `[country_data]`, `[map]`, `[map.bands]`.
  - Every line is `# key = value`.
  - Defaults:
    - `bands = ["hf-contest"]`, with the group table and `["all"]` / `["hf", "-60m"]` kept as
      commented examples;
    - `history_file` commented with "default: qso-history.adi next to the country data;
      /var/lib/live-qso-map/qso-history.adi in the Debian package and the container image";
    - `country_data.path = "./cty.dat"`, with a comment on the Windows/macOS (program folder),
      Debian and container (`/var/lib/live-qso-map/cty.dat`) locations.
  - Placeholders exactly as contract rule 4, including the `[lookup]` comment naming the six
    `LQM_*` credential variables.
  - `style_url` and `style_attribution` show their real defaults, `DEFAULT_STYLE_URL` and
    `DEFAULT_STYLE_ATTRIBUTION` from `src/config.rs:38-41`. Write the credit as a TOML literal
    string (`'…'`) so its HTML double quotes need no escaping.
  - Raster tiles and two `[map.bands]` colours stay, labelled as examples.
  - Command-line-only options and `history_size` stay out.
  - ADIF band names, groups in frequency order (`docs/ham-radio.md` §1).
- [X] T022 [US2] Remove the test `example_sets_the_packaged_data_path` from
  `src/config_export.rs`; T002 covers it for the Debian file. Run `cargo test config_export`:
  T017–T020, `uncommented_example_loads` and `example_lists_every_exported_setting` all pass.

**Checkpoint**: US2 is complete; the example is safe to use unchanged and self-explanatory.

---

## Phase 5: User Story 3 - The example cannot fall behind (Priority: P2)

**Goal**: removing any setting, credentials included, from the example fails `cargo test` and
names the setting. A new lookup provider cannot be added without the test noticing.

**Independent Test**: delete `# password = …` under `[lookup.hamqth]`. `cargo test` fails with
`deploy/config.example.toml lacks lookup.hamqth.password`. Restore the line.

### Tests for User Story 3 (write first, confirm they FAIL)

- [X] T023 [US3] In `src/config_export.rs` `example_lists_every_exported_setting`, set credentials
  for every provider in `Config::default().lookup.provider_order`. Use an exhaustive `match` on
  `ProviderKind` (`Qrz` → `config.lookup.qrz`, `HamQth` → `hamqth`, `QrzCq` → `qrzcq`), with
  `Credentials { username: "u".into(), password: Secret::new("p") }` (the existing
  constructor, `src/config.rs:274`). A new variant then fails to compile here until the test, and so the example, are
  updated.

  Confirm the guard works by mutation: delete the `[lookup.hamqth]` password line from
  `deploy/config.example.toml`, run `cargo test config_export`, and see it fail naming
  `lookup.hamqth.password`. Restore the line and see it pass.
- [X] T024 [US3] Same mutation check for a non-credential setting: delete `# recent_header = …`,
  see `map.recent_header` named, restore. Record both mutation runs (key named, restored) in the
  PR description.

### Implementation for User Story 3

None: US3 is a test-only story. The example content it guards comes from T021, and no runtime or
test-support code is needed (`Secret::new` already exists).

**Checkpoint**: the completeness guard covers every setting and every provider.

---

## Phase 6: Polish & Quality Gates

- [X] T025 [P] Add entries under `## [Unreleased]` in `CHANGELOG.md`.
  - `### Added`: `config.example.toml` (every setting commented out, placeholders for credentials
    and station) is in every release archive, package and image, and is a release download of
    its own.
  - `### Changed`:
    - the Debian package installs a short `/etc/live-qso-map/config.toml`; the full example is in
      `/usr/share/doc/live-qso-map/`;
    - upgrading asks once about the configuration file if you edited it (dpkg's standard
      prompt).
- [X] T026 [P] Update `specs/008-platform-packaging/contracts/release-artifacts.md` and
  `debian-package.md`. Add a one-line note at the top of each: "Superseded for the example and
  the Debian live configuration by `specs/015-example-config-release/contracts/release-deliverables.md`."
- [X] T027 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and
  the frontend and tool tests (`node --test 'web/*.test.js'`, the `tools/` tests as in
  `.circleci/config.yml` `test-tools`). All pass.
- [X] T028 Run the quickstart's development-host checks (`quickstart.md`, "On any development
  host"): the `--save-config -` diff with and without the example is empty.
- [ ] T029 [R:YELLOW] Push the branch and run a release rehearsal
  (`circleci pipeline run … --param rehearsal=true`, docs/ci.md section 7), as LL-007 and
  research R7 require. Expect:
  - `check-release-files.sh` prints `ok` for 3 `.tar.gz`, 2 `.zip`, 3 `.deb` and the standalone
    file;
  - `package-deb` passes its doc and conffiles checks (×3);
  - `test-deb` passes on every image;
  - `image-test` passes `example` (×2);
  - `rehearsal-summary` stores `config.example.toml`.

  Fix and re-run until green before opening the pull request.

---

## Dependencies & Execution Order

- **Setup (T001)** → everything.
- **US1**:
  - T002–T006 (tests/checks) before T007–T011.
  - T007 before T008.
  - T012–T016 (docs) are independent and can go any time in US1.
- **US2** depends on T007–T008: the example may stop setting the Debian path only once the
  Debian live file does.
  - T017, T018 [P] → T019 → T020 → T021 → T022.
- **US3**: T023–T024 can start after T001. The mutation checks are most meaningful after T021,
  so run them then.
- **Polish**: T025, T026 [P] any time. T027 → T028 → T029 last.

```text
T001 ─┬─ US1: T002 T003 T004 T005 T006 ─→ T007 → T008 → T009 T010 T011 ─┐
      │        T012 T013 T014 T015 T016 (docs, parallel)                  │
      ├─ US2 (after T008): T017 T018 → T019 → T020 → T021 → T022 ─────────┤
      └─ US3: T023 → T024 (mutation after T021) ───────────────────┴→ T027 → T028 → T029
```

## Parallel Examples

- **US1 tests**: T002 (`src/config_export.rs`), T003 (`scripts/check-release-files.sh`) and T004
  (`scripts/test-image.sh`) touch different files and run together. T005 and T006 both edit
  `.circleci/config.yml`: do them in sequence.
- **US1 docs**: T012 (`postinst`), T013 (`windows/README.txt`), T014 (`macos/README.txt`), T015
  (`README.md`) and T016 (`docs/releasing.md`) run together.
- **US2 tests**: T017 and T018 are independent new tests. T019 and T020 share the samples helper,
  so they run in sequence.
- **Polish**: T025 (`CHANGELOG.md`) and T026 (feature 008 contracts).

## Implementation Strategy

1. **MVP = US1** (T001–T016): every deliverable ships the example, and Debian has its own live
   file. It can ship on its own; the example's content is then today's, improved by US2.
2. **US2** (T017–T022): placeholders, everything commented, defaults shown. This is the user's
   explicit secrets requirement, so US1 and US2 together are the minimum for a release.
3. **US3** (T023–T024): the guard that keeps "every parameter" true.
4. **Polish** (T025–T029): changelog, superseded contracts, gates, rehearsal. No release tag
   until T029 is green.
