----
name: Batch Bug Fix & Patch Release
description: Retrieves open bug issues, applies fixes, runs tests, bumps version, and publishes a patch release.
version: 1.0.0
author: Sam Clark
tags: [github, automation, release, bugfix, ci/cd]
tools: [github_cli, git, npm/python, editor]
---

## Objective

Pull all issues labeled "bug" from the GitHub repository, apply necessary fixes, run tests to ensure stability, bump the version number, and publish a patch release.

## Prerequisites

- Repository is in a clean state (no uncommitted changes).
- Local environment has the toolchain installed.
- `gh` CLI installed and authenticated.
