"""Regression tests for the CircleCI configuration (feature 009, T008).

The pipeline can't be unit-tested, but the two bugs fixed in feature 009 show up as text in
`.circleci/config.yml` (and the README). These tests read both files as text, so they need no
YAML library and run on any host.

Run with: python3 -m unittest tools/test_ci_config.py
"""

import os
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

PAGE_TESTS = "node --test 'web/*.test.js'"


def read(path):
    with open(os.path.join(ROOT, path), encoding="utf-8") as f:
        return f.read()


class PageTestCommand(unittest.TestCase):
    """R16: `node --test web/` loads `web` as a module on Node 24 and runs no test."""

    def test_page_tests_use_portable_glob(self):
        config = read(".circleci/config.yml")
        self.assertIn(PAGE_TESTS, config)
        self.assertNotIn("node --test web/", config)

    def test_readme_documents_the_same_command(self):
        readme = read("README.md")
        self.assertIn(PAGE_TESTS, readme)
        self.assertNotIn("node --test web/", readme)


class ReleaseRerun(unittest.TestCase):
    """R2: `ghr -delete` deleted and recreated an existing release, losing its notes."""

    def setUp(self):
        self.config = read(".circleci/config.yml")

    def test_release_rerun_keeps_notes(self):
        self.assertNotIn("ghr ", self.config)
        self.assertNotIn(" -delete", self.config)
        view = self.config.find("gh release view")
        create = self.config.find("gh release create")
        self.assertNotEqual(view, -1, "no existence check before creating the release")
        self.assertNotEqual(create, -1, "no gh release create")
        self.assertLess(view, create)
        create_line = self.config[create:self.config.find("\n", create)]
        self.assertIn("--verify-tag", create_line)
        self.assertIn("--notes-file", create_line)
        self.assertRegex(self.config, r"gh release upload [^\n]*--clobber")


if __name__ == "__main__":
    unittest.main()


CHROME_URL = (
    "https://storage.googleapis.com/chrome-for-testing-public/154.0.8037.92/linux64/chrome-linux64.zip"
)
CHROME_SHA256 = "ff43322f335e436b2f4dcdfeeec5db032299e335a7e8c1c618b326e100ce8732"


def section(text, header, indent):
    """The lines from `header` (at `indent` spaces) up to the next line at the same or lower indent."""
    lines = text.splitlines()
    start = lines.index(" " * indent + header)
    body = [lines[start]]
    for line in lines[start + 1:]:
        stripped = line.lstrip(" ")
        if stripped and not stripped.startswith("#") and len(line) - len(stripped) <= indent:
            break
        body.append(line)
    return "\n".join(body)


def workflow_entry(workflow, job):
    """The `- job:` entry of a workflow (or `- job` with no options), up to the next entry."""
    lines = workflow.splitlines()
    for i, line in enumerate(lines):
        if line.strip() in (f"- {job}:", f"- {job}"):
            indent = len(line) - len(line.lstrip(" "))
            entry = [line]
            for next_line in lines[i + 1:]:
                stripped = next_line.lstrip(" ")
                if stripped and len(next_line) - len(stripped) <= indent:
                    break
                entry.append(next_line)
            return "\n".join(entry)
    return None


class BrowserTests(unittest.TestCase):
    """Feature 012 (contracts/pipeline.md): real-browser tests gate the release, and only it."""

    def setUp(self):
        self.config = read(".circleci/config.yml")
        jobs = section(self.config, "jobs:", 0)
        self.job = section(jobs, "browser-test:", 2)
        workflows = section(self.config, "workflows:", 0)
        self.release = section(workflows, "test_and_build:", 2)
        self.rehearsal = section(workflows, "rehearsal:", 2)
        self.weekly = section(workflows, "newest_toolchain:", 2)

    def test_job_uses_the_pinned_image_and_chrome(self):
        self.assertIn("image: cimg/node:24.21.0-browsers", self.job)
        self.assertIn(f"url: {CHROME_URL}", self.job)
        self.assertIn(f"sha256: {CHROME_SHA256}", self.job)
        self.assertIn("fetch-verified:", self.job)

    def test_job_runs_the_release_binary_and_keeps_artifacts(self):
        self.assertRegex(
            self.job,
            r"node tests/browser/run\.mjs(?:[^\n]|\\\n)*--binary ws/bin/x86_64-unknown-linux-musl/live-qso-map",
        )
        # The artifacts are kept whatever the result: the test step records the exit code
        # instead of failing, the artifacts are stored, and only then does a step fail the job.
        run = self.job.index("node tests/browser/run.mjs")
        store = self.job.index("store_artifacts:")
        verdict = self.job.index("exit \"$(cat")
        self.assertLess(run, store)
        self.assertLess(store, verdict)
        self.assertIn("path: target/browser-test", self.job[store:verdict])

    def test_release_runs_it_on_tags_after_the_x86_64_build(self):
        entry = workflow_entry(self.release, "browser-test")
        self.assertIsNotNone(entry, "browser-test is not in the release workflow")
        self.assertIn("filters: *release-tags", entry)
        self.assertIn("build-linux-x86_64-unknown-linux-musl", entry)
        self.assertNotIn("context:", entry, "no secret reaches the browser tests (FR-013)")

    def test_publishing_waits_for_it(self):
        publish = workflow_entry(self.release, "publish-github-release")
        self.assertRegex(publish, r"\n\s*- browser-test\n")

    def test_rehearsal_runs_it_and_its_summary_waits(self):
        entry = workflow_entry(self.rehearsal, "browser-test")
        self.assertIsNotNone(entry, "browser-test is not in the rehearsal")
        self.assertNotIn("context:", entry)
        summary = workflow_entry(self.rehearsal, "rehearsal-summary")
        self.assertRegex(summary, r"\n\s*- browser-test\n")

    def test_never_on_pull_requests_main_or_the_weekly_run(self):
        self.assertIsNone(workflow_entry(self.weekly, "browser-test"))
        entry = workflow_entry(self.release, "browser-test")
        # In the gates workflow it exists only behind the release-tag filter (checked above), which
        # ignores every branch: pull requests and main pushes never run it.
        self.assertIsNotNone(entry)
        self.assertIn("filters: *release-tags", entry)
