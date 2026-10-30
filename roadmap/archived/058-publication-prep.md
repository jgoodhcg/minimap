---
title: "Publication prep: a fake repo of its own and a one-commit history"
status: done
description: "Prepare the repo for a public GitHub repo: a new Polylith layout for the fake quayside repo, no measurements of the private target repo in tracked files, and one commit with the earlier commit messages kept in docs/git-history.md"
created: 2026-10-19
updated: 2026-10-19
tags: [publication]
priority: high
---

# Publication prep

## Intent

The user publishes `minimap` on their GitHub account. Nothing in the repo may show the private target repo:
not its name, its paths, its structure, or its measurements.
The repo starts over as one commit, so no earlier commit carries those facts. Its commit messages stay readable in a file.

## Specification

- `mockups/fakegen.py` lays out `quayside` as a Polylith workspace: `bases/`, `components/`, `projects/`, `development/`,
  and beside them `migrations/`, `assets/`, `deploy/`, `insights/`, `acceptance/`, `docs/`, and `tools/`. It has 2,585 files.
  The berth component (`components/berth/src/quayside/berth/`) keeps the files that tests and mockup demos pin.
  Commit subjects use the `area: Subject` style, and the agent file is `AGENTS.md`.
- The quayside fixtures, goldens, and snapshots come from the new fake: `mockups/export-layout.mjs` (tiles and chars),
  `scripts/quayside-details.py` (the berth directory and one short test file), and `scripts/fzf-golden.py`.
- Tests that named old quayside paths use the new equivalents, and tests that hard-coded counts read them from the data where they can.
- `docs/`, `roadmap/`, `AGENTS.md`, and code comments keep no counts, sizes, timings, paths, or names of the private target repo.
  Results on a real repo read as qualitative statements. The kickoff mockup links left `docs/design.md`.
- `docs/git-history.md` keeps the 70 earlier commits: date, old short hash, subject, body, and model, with the private measurements removed.
- The full history stays in a git bundle under `.private/` (git-ignored).

## Validation

- [x] `AGENTS.md` validation levels 1–3 and 4b's tests pass. The real-repo tests pass with `--no-fail-fast`.
- [x] `git grep` finds no name of the private repo, no old quayside path, and no old quayside count in tracked files.
- [x] The bundle verifies and records the complete history.
- [x] After the commit, the repo has one commit and no other ref.

## Scope

- No change to the TUI's or the GUI's behavior.
- No push: the user creates the GitHub repo and pushes.
