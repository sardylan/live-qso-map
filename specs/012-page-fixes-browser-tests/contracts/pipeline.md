# Contract: the `browser-test` pipeline job

```yaml
browser-test:
  docker:
    - image: cimg/node:24.21.0-browsers
  steps:
    - checkout
    - attach_workspace: { at: ws }
    - fetch-verified:
        url: https://storage.googleapis.com/chrome-for-testing-public/154.0.8037.92/linux64/chrome-linux64.zip
        sha256: ff43322f335e436b2f4dcdfeeec5db032299e335a7e8c1c618b326e100ce8732
        dest: chrome-linux64.zip
    - run: unzip -q chrome-linux64.zip
    - run: node tests/browser/run.mjs --binary ws/bin/x86_64-unknown-linux-musl/live-qso-map --chrome chrome-linux64/chrome || status=$?   # exit code saved, not failed
    - store_artifacts: { path: target/browser-test, destination: browser-test }   # so it always runs
    - run: exit "$(cat browser-test-status)"   # the job fails here when the tests failed
```

**Workflows:**

| Workflow | Runs it? | Requires | Required by |
|----------|----------|----------|-------------|
| gates (pull requests, `main` pushes) | no | — | — |
| weekly schedule | no | — | — |
| release (tags `vX.Y.Z`) | yes, `filters: *release-tags` | `build-linux-x86_64-unknown-linux-musl` | `publish-github-release` (and so `docker-promote`) |
| rehearsal (`--param rehearsal=true`) | yes | `build-linux-x86_64-unknown-linux-musl` | `rehearsal-summary` |

**Context:** none. The job uses no secret (FR-013).

**Re-run from failed:** re-runs `browser-test`. Its result is never cached.
