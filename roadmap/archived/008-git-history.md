---
title: "Git history and heat layers"
status: done
description: "Load per-file git history in the background, show a git group in the panel, and add age and churn heat layers to the map"
created: 2026-09-25
updated: 2026-09-25
tags: [milestone-3, core, git, layers]
priority: medium
---

# Git history and heat layers

## Intent

Age and churn tell the user which parts of the repo are alive and which are settled.
Recent commit descriptions give the "why" behind a file.

## Specification

In `minimap-core`, module `history`:

- Walk every commit reachable from `HEAD` with `gix` `rev_walk`, newest commit time first.
  Skip merge commits. Diff each other commit's tree against its parent's, and the root commit's tree against the empty tree (`gix::diff::tree`).
  This counts what `git rev-list --no-merges --full-history HEAD -- <path>` counts.
  No new `gix` features are needed: `rev_walk` and `gix::diff::tree` have no feature gate in `gix` 0.87.
- Record, for each path in the index: the commit count, the newest and oldest commit times, and the 5 newest commits as `(hash, time, subject)`.
- Use the committer time. Compute ages in whole days from a `now` parameter, so tests can fix it.
- Do not follow renames. A renamed file's history starts at the rename.
- Run the walk on a background thread after the map shows. The panel shows `loading` in the GIT group until it finishes, then redraws.
- Measure the walk on the real target repo and record the time. If it is over 5 s, report it; a cache is a Quick Idea.
- `--from-json` mode reads the `a0`, `a1`, `c`, `r` fields and the `commits` map (ages in days) from the JSON, matched by path.

In the TUI:

- Selection panel GIT group, between the outline and the key hint (`mockups/v5-template.html` `drawSelection`):
  `GIT` with `N commits` right-aligned, `changed X ago · created Y ago`, then up to 5 rows of age plus description.
  Ages use the format in `docs/design.md` `Selection panel`. Strip the description prefix with `^[a-z]+(\([^)]*\))?!?:\s*` (case-insensitive).
  A file with no commits shows `No commits yet.`
- `H` copies the newest commit hash. If there is none, flash `No commits for this file.` While the walk runs, flash that git is still loading.
- Layers, on the map only (the overview shows no layers):
  - `4` toggles age heat: by days since the last change, ramp steps at ≤14, ≤60, and ≤365 days.
  - `5` toggles churn heat: by commit count, ramp steps at ≥60, ≥25, and ≥10.
  - `4` and `5` are exclusive, so they share one state color: `#e0529c`.
    The cell background mixes it over the pane at 45 %, 30 %, and 18 % for the three steps. Files past the last step get no heat.
  - Heat is the lowest cell state (`docs/design.md` `Color`). The glyph keeps its type color.
  - A heat layer can turn on while the walk runs. Its colors show when the history arrives.
  - The status bar starts with the `4 age` and `5 churn` toggles and, while a heat layer is on, its ramp legend: two cells of each step color and `<2w <2mo <1y` or `60+ 25+ 10+`.
- The help overlay and `docs/design.md` `Keys` list `H`, `4`, and `5`.

## Validation

- [x] Unit tests on a temporary repo with scripted commits and fixed dates (`GIT_AUTHOR_DATE`, `GIT_COMMITTER_DATE`): counts, oldest and newest ages, recent subjects in order, and a file changed on a merged branch that counts once, with the merge commit in no file's recent commits.
- [x] Unit test: prefix stripping on `feat(#123): add x`, `fix!: y`, `build(deps): Bump z`, `Merge branch z`.
- [x] Unit test: ramp steps at their edges.
- [x] Unit test: toggling a heat layer changes no `Layout` tile position and no camera position.
- [x] Snapshot tests at 150 × 45: panel with the git group; map with age heat on.
- [x] Real-repo test: for 20 files, the counts, newest and oldest times, and the recent commits match `git rev-list --no-merges --full-history`. Print the walk time.
- [x] Visual check (see `AGENTS.md` `Visual check`) on the real target repo: the git group fills in after loading, and `4` and `5` recolor the map.
- [x] All `AGENTS.md` validation levels 1–4 pass.

## Scope

- No rename tracking, no blame, no cache.
- No heat in the overview, and no age or commit text on tiles (a v5 tile is one cell).

## Context

- The selection panel is the right column from unit `018`. Unit `007` adds the shape and outline above the GIT group.
- `docs/design.md` `Selection panel`, `Color`, `Layers`.
- The target repo's commit and merge counts, and the time `git log` takes over them, are in `.private/targets.md`.
- Milestone 3 exit: after this unit, stop and ask the user to review.
