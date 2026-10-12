# Contract: CI Service and Code Host Setup (feature 009)

Settings that live outside the repository. `docs/ci.md` records them for whoever maintains the
project (FR-008). IDs are those of 2026-10-12.

- **Project:** `gh/sardylan/live-qso-map`, id `e4ace6e1-9360-4882-9faf-21f77e871b0e`.
- **Organisation:** `sardylan`, id `332144a0-85cc-43d6-b55d-496fed496e29`.
- **Repository:** `sardylan/live-qso-map`, external id `883686784`.

## Repository connection (done on 2026-10-12)

| Item | Value |
|------|-------|
| CircleCI GitHub App | installed on `sardylan` through Org → VCS Connections, with access to `live-qso-map`; authorised by the maintainer |
| GitHub App pipeline | `f5014549-bbc3-4712-901f-e2bd315c9e6e` "live-qso-map (GitHub App)", config and checkout `github_app`, `.circleci/config.yml` |
| OAuth pipeline | `baf57313-9196-5daa-8992-aed6f4629331`: can't be deleted; MUST have no trigger (its trigger `72e306b5…` was deleted) |
| Checkout | HTTPS (`Cloning git repository - https://github.com/…`); no deploy key, no SSH key |

**Diagnosis:**
- **No run starts:** check that the App still has access to the repository (GitHub → Settings
  → Applications → CircleCI → Repository access) and that the trigger exists
  (`circleci project trigger list --pipeline-definition-id f5014549-…`).
- **Two runs per event:** an OAuth trigger was recreated; delete it.
- **Checkout over `git@github.com`:** the run came from the OAuth pipeline.

## Triggers

| Trigger | Provider | Preset or schedule | Parameters |
|---------|----------|--------------------|------------|
| events | `github_app`, repository `883686784` | `only-build-prs` | none |
| weekly | `schedule` | `0 4 * * 1` (Monday 04:00 UTC), `main` | `{"newest-toolchain": true}` |

**Replacing the event trigger** (the old `all-pushes` trigger is `2f62efc0…`):
```
circleci project trigger create --pipeline-definition-id f5014549-bbc3-4712-901f-e2bd315c9e6e \
  --provider github_app --repo-id 883686784 --event-preset only-build-prs
circleci api -X DELETE api/v2/projects/e4ace6e1-9360-4882-9faf-21f77e871b0e/triggers/2f62efc0-51a0-461d-96c2-f5750aea445d
```

**Creating the schedule** (if the API refuses it for GitHub App pipelines: Project Setup →
Schedule, same values):
```
circleci api -X POST api/v2/projects/e4ace6e1-9360-4882-9faf-21f77e871b0e/pipeline-definitions/f5014549-bbc3-4712-901f-e2bd315c9e6e/triggers -d '{
  "event_name": "weekly newest toolchain",
  "event_source": {"provider": "schedule", "schedule": {"cron_expression": "0 4 * * 1", "attribution_actor": "system"}},
  "config_ref": "main", "checkout_ref": "main",
  "parameters": {"newest-toolchain": true}
}'
```

## Context `live-qso-map-release`

| Item | Value |
|------|-------|
| Variables | `GITHUB_TOKEN` (fine-grained, this repository only, `contents: write`), `DOCKERHUB_USERNAME`, `DOCKERHUB_TOKEN` (personal access token, Read/Write/Delete on `sardylan/live-qso-map`) |
| Restriction 1 | project `e4ace6e1-9360-4882-9faf-21f77e871b0e` |
| Restriction 2 | expression `pipeline.git.tag starts-with "v"` |
| Project-level variables | none with these names (deleted if present) |

```
circleci context create live-qso-map-release --org gh/sardylan
circleci context restriction create live-qso-map-release --type project --value e4ace6e1-9360-4882-9faf-21f77e871b0e --org gh/sardylan
circleci context restriction create live-qso-map-release --type expression --value 'pipeline.git.tag starts-with "v"' --org gh/sardylan
circleci context env-var list live-qso-map-release --org gh/sardylan   # names only
```

The variable values are entered by the maintainer (CLI prompt or UI), never in a file or a
command line.

## Ruleset on `main` (`deploy/github/main-ruleset.json`)

```json
{
  "name": "main",
  "target": "branch",
  "enforcement": "active",
  "conditions": { "ref_name": { "include": ["~DEFAULT_BRANCH"], "exclude": [] } },
  "bypass_actors": [ { "actor_type": "RepositoryRole", "actor_id": 5, "bypass_mode": "always" } ],
  "rules": [
    { "type": "deletion" },
    { "type": "non_fast_forward" },
    { "type": "pull_request", "parameters": {
        "required_approving_review_count": 0, "dismiss_stale_reviews_on_push": false,
        "require_code_owner_review": false, "require_last_push_approval": false,
        "required_review_thread_resolution": false } },
    { "type": "required_status_checks", "parameters": {
        "strict_required_status_checks_policy": false,
        "required_status_checks": [
          { "context": "ci/circleci: lint", "integration_id": 302869 },
          { "context": "ci/circleci: test-rust", "integration_id": 302869 },
          { "context": "ci/circleci: test-web", "integration_id": 302869 },
          { "context": "ci/circleci: test-tools", "integration_id": 302869 },
          { "context": "ci/circleci: audit", "integration_id": 302869 } ] } }
  ]
}
```

**Applying it:** `gh api -X POST repos/sardylan/live-qso-map/rulesets --input deploy/github/main-ruleset.json`.
Later changes use `PUT repos/sardylan/live-qso-map/rulesets/<id>`.

**Renaming a gate job:** the JSON MUST change in the same pull request.

## Pull requests from forks

They never start a run (a GitHub App limit). The procedure:
1. review the code;
2. `gh pr checkout <n>`;
3. `git push origin HEAD:refs/heads/contrib/<n>`;
4. `gh pr create --head contrib/<n> --base main --title "<title> (from #<n>)"`;
5. close `#<n>` with a link to the new pull request.

The commits keep their author.
