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


if __name__ == "__main__":
    unittest.main()
