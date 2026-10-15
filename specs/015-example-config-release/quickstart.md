# Quickstart: verifying feature 015

## On any development host

```bash
cargo test config_export      # example completeness, all-commented, placeholders, Debian live file
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

Manual check of FR-003: the example unchanged is the same as no file.

```bash
cargo run -q -- --config deploy/config.example.toml --save-config - | tail -n +2 > /tmp/with
cargo run -q -- --save-config - | tail -n +2 > /tmp/without
diff /tmp/with /tmp/without     # no output
```

Mutation check of FR-011: delete the `# password = …` line under `[lookup.hamqth]`. `cargo test`
must fail and name `lookup.hamqth.password`. Then restore the line.

## In the pipeline (release rehearsal, docs/ci.md section 7)

```bash
circleci pipeline run … --param rehearsal=true   # on the feature branch, before merge (LL-007)
```

Expect:
- `rehearsal-summary` lists `config.example.toml` among the files in `ws/release`, and
  `check-release-files.sh` prints one passing line per archive, zip and package;
- `image-test` passes its `example` step on both architectures;
- `package-deb` passes its contents check on all three architectures;
- `test-deb` still passes its install, upgrade-keeps-edits, remove and purge checks with the new
  short live file.

## After a real release

- The GitHub release lists `config.example.toml` on its own, and `SHA256SUMS` has a line for it.
- `docker run --rm --entrypoint cat sardylan/live-qso-map:<v> /usr/share/doc/live-qso-map/config.example.toml`
  prints the file.
