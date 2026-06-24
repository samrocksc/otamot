---
name: GitHub Issues Workflow
description: End-to-end workflow for picking up, implementing, and closing a GitHub issue. Covers issue selection, branch creation, implementation, verification, and PR creation.
version: 1.0.0
author: Sam Clark
tags: [github, workflow, feature, bugfix, issues]
tools: [gh, git, cargo, editor]
---

## Objective

Take a GitHub issue from open → implemented → PR. This is the primary development loop for otamot.

## Prerequisites

- Clean working tree (`git status` shows nothing uncommitted)
- Toolchain verified (`cargo check` passes)
- `gh` CLI authenticated

---

## Step 1 — Pick an Issue

If no issue number was provided, list open issues and choose one:

```bash
gh issue list --repo samrocksc/otamot --state open
```

Read the full issue body before starting:

```bash
gh issue view <number> --repo samrocksc/otamot
```

Note the issue number and title. Extract a short kebab-case slug from the title (e.g. issue #56 "Overview in Drop Down" → `overview-in-drop-down`).

---

## Step 2 — Create a Feature Branch

```bash
git checkout -b feature/<number>-<slug>
```

Example: `feature/56-overview-in-drop-down`

---

## Step 3 — Baseline Check

```bash
cargo check
cargo test
```

All tests must pass before touching any code. If they don't, stop and report.

---

## Step 4 — Implement

Follow the patterns in `AGENTS.md`:

- New module → `src/<module>.rs` with `impl Default`, add to `lib.rs`, import in `app.rs`
- New config field → update `impl Default for Config`, search `Config {` in tests, run `cargo test`
- Business logic → `src/` lib modules only (never in `src/ui/`)
- Visuals → `src/ui/` only

Reference `.agents/docs/ARCHITECTURE.md` for layer boundaries and `.agents/docs/CODE_STYLE.md` for style.

Implement the minimum required to satisfy the issue's acceptance criteria. No scope creep.

---

## Step 5 — Verify

```bash
cargo fmt && cargo clippy && cargo test
```

All three must pass clean. Fix any clippy warnings before continuing — do not suppress them.

---

## Step 6 — Commit

```bash
git add <relevant files>
git commit -m "feat: <short description>

Closes #<number>"
```

Use `feat:` for new features, `fix:` for bugs, `chore:` for maintenance.

---

## Step 7 — Push and Open PR

```bash
git push -u origin feature/<number>-<slug>
gh pr create \
  --title "<issue title>" \
  --body "Closes #<number>

## Changes
- <bullet summary of what changed>

## Testing
- [ ] cargo test passes
- [ ] cargo clippy clean
- [ ] Manual smoke test of the feature"
```

---

## Notes

- The issue number in the commit message (`Closes #N`) auto-closes the issue when the PR merges.
- If the issue has no acceptance criteria, clarify with the user before implementing.
- Reference `.agents/SOUL.md` for the agent's character — keep the vibe cowabunga. 🍕
