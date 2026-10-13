# Feature Specification: Page fixes and real-browser release tests

**Feature Branch**: `feature/012-page-fixes-browser-tests`
**Created**: 2026-10-13
**Status**: Draft
**Input**: User description: "Create a new spec/feature for fixing these bugs, and adds some tests that use a real browser instance, to be run only on releases in pipelines."

<!--
  Use the amateur radio terms defined in docs/ham-radio.md §2 (QSO, entity, band, locator,
  dupe…). Requirements MUST NOT contradict its rules in §1 (e.g. UTC times, bands listed in
  frequency order).
-->

## Context

Release 3.0.0 (features 010 and 011) was published with a broken map page. Opening it in Google
Chrome on 2026-10-13 showed these bugs:

| # | Bug | Effect | Seen |
|---|-----|--------|------|
| B1 | The service doesn't serve four of the page's script files (themes, sun, settings panel, legend groups). | The page script stops before it starts: no map buttons, no settings button, no legend, no theme. The map shows, but nothing works. | 404 for each file, in Chrome's console |
| B2 | In the Dim theme, the active state of a map button (Auto-zoom on) is hard to tell from the map behind it and from the inactive state. | A viewer can't see whether auto-zoom is on. | Screenshot, Dim theme |
| B3 | The settings panel is slightly see-through. The legend and the base-map credit underneath show through it. | Ghost text behind the Legend group and the panel's lower half. | Screenshot, every theme |
| B4 | With the default bands, the Legend group offers two groups that select exactly the same bands ("HF (6 bands)" and "HF contest (6 bands)"). | Two choices that do the same thing. | Screenshot |

**Why the checks missed B1:** the page's own tests load the script files straight from the
source tree, never through the service. The server tests checked a fixed list of files, and that
list wasn't updated when the four files were added. The page was never opened in a browser
before the release (the manual checks of feature 011 were still open when it shipped).

**B1 is already fixed** on this feature's branch: the four files are served, and a server test
now fetches every page file found in the source tree, so a forgotten one fails the build. The
spec still lists it, so the release and its tests cover it.

**What is missing** is a check that opens the page in a real browser, as a viewer does, before a
release is published. Such a check is slower and needs a browser, so it runs only when a release
is built, not on every change.

## Clarifications

### Session 2026-10-13

- Q: When two band groups select the same considered bands (with the default bands, "HF" and
  "HF contest"), which one does the Legend group offer? → A: The more specific one: the group
  with the fewest bands in its full definition ("HF contest"); on a tie, the first in the group
  order.
- Q: Where does the browser come from in the pipeline (Debian's Chromium package, or an image
  with a browser already installed)? → A: CircleCI's own browser image (`cimg/node`, the
  `-browsers` variant), pinned to an exact version. (Planning found that the image holds
  Chrome's libraries but not Chrome; the job adds a pinned, checksum-verified Chrome build.)

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The map page works again (Priority: P1)

An operator upgrades to the fixed release and opens the map. The map buttons, the settings
button, the legend and the theme all work, as they did before 3.0.0, plus the feature 011
additions.

**Why this priority**: in 3.0.0 the page is unusable. Everything else depends on this.

**Independent Test**: start the service, open the page in a browser, and check that the browser
reports no failed request to the service and no script error, and that the map buttons, the
settings button and the legend are shown.

**Acceptance Scenarios**:

1. **Given** the service is running, **When** a viewer opens the page, **Then** every file the
   page asks the service for is delivered, and the page shows the zoom, Globe/Flat, Auto-zoom and
   settings buttons and the legend.
2. **Given** a new page file is added to the source tree but not served, **When** the automated
   checks run, **Then** they fail and name the file.

---

### User Story 2 - A release can't ship a broken page (Priority: P1)

The maintainer tags a release. Before anything is published, the release pipeline starts the
built program, opens its page in a real browser and plays a short viewer session. If the page
fails, the release stops and nothing is published.

**Why this priority**: it prevents a repeat of 3.0.0. Without it, the next missing file or
broken script ships again unnoticed.

**Independent Test**: in a rehearsal run (feature 009), break the page on purpose (for example,
leave a file unserved) and check that the browser tests fail and block publishing. Then restore
it and check that the release goes through.

**Acceptance Scenarios**:

1. **Given** a release tag, **When** the pipeline runs, **Then** the browser tests run against
   the program built for that release, before the GitHub release and the container image are
   published.
2. **Given** the browser tests fail, **When** the pipeline continues, **Then** the GitHub release
   is not created, the container image is not promoted, and the failing step names what failed.
3. **Given** a pull request or a push to `main`, **When** the pipeline runs, **Then** the browser
   tests don't run.
4. **Given** a browser test run, **When** it finishes (pass or fail), **Then** the pipeline keeps
   the browser's console log and a screenshot of each tested state, for the maintainer to look
   at.
5. **Given** a developer's machine with the browser installed, **When** they run the browser
   tests by hand, **Then** the same tests run locally against a local build.

---

### User Story 3 - The tested viewer session (Priority: P1)

The browser tests act as a viewer would: they open the page, send QSOs to the service, and use
the controls added in feature 011.

**Why this priority**: the tests are only as good as what they cover. These are the paths that
broke, or could break, unnoticed.

**Independent Test**: run the browser tests; each step below passes or names what went wrong.

**Acceptance Scenarios**:

1. **Given** the page is open, **When** the test sends QSOs on several bands (some considered,
   some not), **Then** the considered ones appear on the map and in the recent box within
   5 seconds, and the others never appear.
2. **Given** the page is open, **When** the test opens the settings panel with the settings
   button, **Then** the panel shows its four groups and focus is on its first control. **When**
   it presses Escape, **Then** the panel closes and focus is back on the settings button.
3. **Given** the panel is open, **When** the test picks each theme (Light, Dim, Dark), **Then**
   the page shows that theme, the status line names it, and auto-zoom, the view and the QSOs are
   unchanged.
4. **Given** the device prefers dark, **When** the page opens with the automatic (device) theme,
   **Then** it shows Dark.
5. **Given** two groups with different bands are offered, **When** the test ticks one group,
   **Then** the legend lists only that group's bands, and the map and recent box show only QSOs on
   them. **When** it unticks it, **Then** everything is shown again.
6. **Given** a viewer setting was changed, **When** the page is reloaded, **Then** the setting is
   kept.
7. **Given** the service is stopped and started again, **When** the page reconnects, **Then** the
   reconnection banner shows and then goes away, and QSOs sent after the restart appear.

---

### User Story 4 - The smaller page bugs are fixed (Priority: P2)

A viewer using the Dim theme can tell whether a map button is on. The settings panel hides what
is under it. The Legend group offers no two groups that do the same thing.

**Why this priority**: B2–B4 are visible but don't stop the page working.

**Independent Test**: open the page in each theme with Auto-zoom on and off, open the panel over
the legend, and look at the offered groups with the default bands.

**Acceptance Scenarios**:

1. **Given** any theme, **When** a toggle button (Auto-zoom) is on, **Then** it is marked by more
   than a background shade, and its on state can be told apart from its off state and from the
   map.
2. **Given** the panel is open, **When** it covers the legend or the map credit, **Then** nothing
   underneath shows through it.
3. **Given** the default bands, **When** the Legend group is shown, **Then** it offers
   "HF contest (6 bands)" and not "HF". **Given** a server that also considers 30 m, **Then** it
   offers both "HF" and "HF contest", with their own counts.

---

### Edge Cases

- **The base map can't be reached from the pipeline**: the browser tests don't depend on base-map
  tiles. A page that shows its "base map unavailable" banner still passes, as long as the rest
  works. The banner is recorded in the log.
- **The page's map library can't be loaded from its CDN**: the page can't work without it. The
  test fails with a message that says so, so it isn't mistaken for a product bug. Re-running the
  workflow is the fix.
- **The browser has no graphics hardware in the pipeline**: the page needs WebGL. The tests use
  the browser's software renderer. If WebGL is still missing, the test fails with a clear
  message, not a timeout.
- **Flaky timing**: every wait has a limit, and a missed limit names what was being waited for.
  No test relies on fixed sleeps alone.
- **A rehearsal run**: the browser tests run there too (feature 009), so the whole
  release path can be tried without publishing.
- **A release re-run after a failure**: re-running the workflow from failed re-runs the browser
  tests. It doesn't skip them.

## Requirements *(mandatory)*

### Functional Requirements

**Page fixes**

- **FR-001**: The service MUST serve every script file and stylesheet the page loads. (B1)
- **FR-002**: An automated check that runs on every change MUST fail when a page file in the
  source tree isn't served, or isn't served unchanged, and MUST name the file. (B1)
- **FR-003**: In every theme, the on state of a toggle map button MUST be shown by more than a
  background shade (for example an outline or a marker in the accent colour). Its text MUST keep
  a 4.5:1 contrast, and the on state MUST stand out against both the off state and the map.
  (B2)
- **FR-004**: The open settings panel MUST be opaque: nothing under it may show through. (B3)
- **FR-005**: The Legend group MUST NOT offer two groups whose considered bands are the same.
  When two or more groups select the same bands, only the most specific one is offered: the
  group with the fewest bands in its full definition, and on a tie, the first in the group
  order. With the default bands, "HF contest" is offered and "HF" isn't. "HF" comes back as soon
  as the server also considers a band that only "HF" has (60 m or a WARC band). (B4)

**Browser tests**

- **FR-006**: There MUST be automated tests that run a real browser (Google Chrome, without a
  window) against the running program, and drive the page as a viewer would. They cover
  User Story 3's scenarios 1–7. In the pipeline, the tests run in CircleCI's browser image
  (`cimg/node`, `-browsers` variant), pinned to an exact version tag. That image ships Chrome's
  system libraries but not Chrome itself, so the job adds a pinned Chrome build, checked against
  its SHA-256 like every other CI download (feature 009).
- **FR-007**: Each browser test run MUST fail when the page:
  - asks the service for a file and doesn't get it;
  - reports a script error;
  - misses an expected control;
  - doesn't reach an expected state within its time limit.

  Warnings from the map library and failed base-map tile requests MUST NOT fail it.
- **FR-008**: The browser tests MUST run in the release pipeline only: on release tags and in
  the rehearsal run (feature 009, started by hand). They MUST NOT run on pull requests, on pushes to `main`, or on the weekly
  schedule.
- **FR-009**: The browser tests MUST run against the program built for the release (the Linux
  x86_64 build), not a separate debug build.
- **FR-010**: The GitHub release and the container image promotion MUST wait for the browser
  tests, and MUST NOT happen when they fail.
- **FR-011**: Every run MUST keep, as pipeline artifacts, the browser's console log, the list of
  failed requests, and one screenshot per tested state: load, panel open, each theme, and the
  legend restricted to a group.
- **FR-012**: The browser tests MUST be runnable by hand on a developer machine with the browser
  installed, with one documented command, against a local build.
- **FR-013**: The browser tests MUST NOT need any account, secret or online callbook. They feed
  QSOs to the program directly, and use the country data bundled with the tests.
- **FR-014**: The release documentation (`docs/releasing.md`, `docs/ci.md`) MUST describe the
  browser tests: when they run, what they block, where to find their artifacts, and how to run
  them locally.

**Release**

- **FR-015**: The fixes MUST ship as release 3.0.1, made with the existing release process
  (feature 009). Its changelog MUST say that 3.0.0's page was broken and that 3.0.1 fixes it.

### Key Entities

- **Browser test run**: one session of the browser tests against one build. It holds the steps
  and their results, the console log, the failed requests and the screenshots.
- **Tested state**: a page state the run checks and photographs, for example "loaded",
  "panel open", "Dark theme" or "legend restricted to one group".

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Opening the page of the fixed release gives 0 failed requests to the service and
  0 script errors, in 100% of runs.
- **SC-002**: A release whose page leaves any page file unserved, or throws a script error on
  load, is stopped before publishing, in 100% of tried cases (checked in a rehearsal run).
- **SC-003**: The browser tests add at most 5 minutes to a release pipeline, and 0 minutes to a
  pull-request or `main` pipeline.
- **SC-004**: A failed browser test run leaves enough evidence (log, failed requests,
  screenshots) for the maintainer to find the cause without re-running it, in 100% of failures.
- **SC-005**: In every theme, a viewer can tell at a glance whether Auto-zoom is on (on/off
  difference visible beyond a shade change; text contrast at least 4.5:1).
- **SC-006**: With the default bands, the Legend group offers no two choices with the same bands:
  only "HF contest".
- **SC-007**: Release 3.0.1 is published with every gate and the browser tests passing.

## Assumptions

- **Browser:** Google Chrome, without a window: a pinned Chrome build inside CircleCI's
  browser image in the pipeline (clarified), and Chrome or Chromium on a developer machine. Other browsers are out of
  scope for now. Debian's Chromium package and the Playwright image were considered; the
  maintainer chose the CircleCI image.
- **Platform:** the pipeline runs the browser tests on Linux x86_64 only. The other builds
  (ARM, macOS, Windows) share the same page files, so testing one build covers the page.
- **Network:** the pipeline can reach the CDN that serves the map library, as viewers do. The
  base map is optional for the tests.
- **Dependencies:** the browser and anything that drives it are CI-only and developer-only
  tools. The browser image is pinned to an exact version, and any other tool is pinned and
  checksum-verified like the other CI tools (feature 009). None of it reaches the shipped
  program (constitution III).
- **Scope:** visual comparison of screenshots against reference images is out of scope. The
  screenshots are kept for people to look at.
- **Manual checks of feature 011** (T008, T015, T019, T030): the browser tests cover what they
  can automate. Anything left (the 5-minute auto-close, screen readers) stays a manual check.
