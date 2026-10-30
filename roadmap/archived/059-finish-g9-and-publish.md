---
title: "Finish the G9 exit and publish the repo"
status: done
description: "Run the open G9 visual check with the user's consent, decide the open G9 choices, set the commit author email, add a README and license files, and push the one-commit repo to the user's GitHub account"
created: 2026-10-20
updated: 2026-10-30
tags: [milestone-g9, publication]
priority: high
---

# Finish the G9 exit and publish

## Intent

Unit `057` (Milestone G9) and unit `058` (publication prep) are done, but their exits have open steps that need the user.
This unit collects those steps, so a new session can finish them in order.

## Specification

1. Install the new GUI. The user runs `cargo install --path crates/gui --locked`. The agent prints the command and does not run it.
2. GUI visual check of unit `057`, only with the user's consent: it opens a window on the user's screen that floats above other windows and takes the focus.
   Check these on the real target repo (`AGENTS.md` `GUI visual check`):
   - The reader's selection fill (`CURSOR` at 25 %, `SELECT_MIX`) is visible on the pane, on the noted-lines tint, and in the note box. The fill is `#17212a` on `#10161c`, so it can be too faint.
   - The session colors and the head row's swatches read on `SIDE`, and the sort chip sits at the head row's right.
   - The status bar hints `n N notes` and `drag select · ⌘c copy` show, and `⌘` draws.
3. Ask the user to decide two G9 choices, and record the answers in unit `057` and `docs/design.md`.
   The user moved both to `roadmap/index.md` `Quick Ideas` as candidates (2026-10-29):
   - Oldest first, the strip does not follow new rows at its end (the unit's rule). Should a strip scrolled to its end follow them?
   - A row without a session now draws `—` in `DIM`; it was `FAINT`. Keep `DIM`?
4. Optional: open `mockups/minimap-v5.html` (after `python3 mockups/build.py`) and check that the new Polylith `quayside` layout reads well as the design reference.
5. Add the files that a public repo needs. The user chose one license, MIT (2026-10-29); `Cargo.toml` declares `MIT`.
   - `README.md` at the root: what minimap is, the two binaries, build and install, run, the config, the agent adapters, and the docs.
   - `LICENSE`: the MIT License, copyright Justin Good, 2026.
6. Publish. The repo is one commit (`Publish minimap`), and it has no remote. The full old history is in `.private/minimap-full-history-2026-10-19.bundle`.
   - Ask the user which author email to publish: `jgoodhcg@gmail.com` (the current author of the commit) or the GitHub noreply address.
     To change it, amend the one commit with `--reset-author` after `git config user.email <address>`, with the user's confirmation.
   - Commit this unit and any answers from steps 2–5 first, with the `AGENTS.md` trailer, after the user confirms.
   - Run `git grep -nIi -f .private/sweep-patterns.txt` and the unit `058` sweep again before the push; both must find nothing.
     The pattern file is git-ignored, because the private names must not enter tracked files.
   - The user creates the repo and pushes: `gh repo create minimap --public --source . --push`. The agent prints the command and does not run it.
     The name can be taken on the user's account; then the user picks another.

## Validation

Decisions (2026-10-28): the commit keeps the author `jgoodhcg@gmail.com`. The user skipped the visual check. The user asked for a README and license files.

- [ ] The user installed the GUI, or chose not to. Open: the installed binaries date from 2026-10-13, before units `056` and `057`. The install moved to `roadmap/index.md` `Current Focus`.
- [x] The visual check ran with the user's consent, and its findings are recorded in unit `057`, or the user skipped it.
- [x] The two G9 choices are decided and recorded: both wait in `Quick Ideas`, and the behavior stays as unit `057` built it.
- [x] `README.md` and `LICENSE` are at the root.
- [x] The sweep finds no private name before the push. On 2026-10-30 `git grep -nIi -f .private/sweep-patterns.txt` finds nothing. The run before the push is not recorded.
- [x] The repo is on GitHub with one or more commits, and the old history exists only in the local bundle. `main` tracks `origin/main`, and the history starts at `Publish minimap`.

## Result (2026-10-30)

The repo is public at `https://github.com/jgoodhcg/minimap`. Milestone G9 is closed. Only the install is open, and it is a user step.

## Scope

- No new features. A finding from the visual check that needs work becomes its own unit.
- Do not push, create a GitHub repo, or change git identity without the user's confirmation.
- Never write inside the target repo.

## Context

- `roadmap/archived/057-usage-quick-fixes.md` (Result), `roadmap/archived/058-publication-prep.md`, `docs/design.md` (`Agent history`, `Note reader`).
- `AGENTS.md` `Visual check` and `Commit Trailer Template`.
