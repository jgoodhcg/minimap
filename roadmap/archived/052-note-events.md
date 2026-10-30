---
title: "Note events and minimap note"
status: done
description: "Let an agent send a note about a file or its lines to the window with `minimap note`, through a new `note` event on the socket"
created: 2026-10-09
updated: 2026-10-30
tags: [milestone-g7, core, cli, adapter]
priority: high
---

# Note events and minimap note

## Intent

The user wants an agent to guide them through a pull request on the map, one file at a time (2026-10-09):
"add a note command, event type, add filtering to event history, display presence of notes in the right hand panel."
The socket knows only `read`, `grep`, `edit`, and `clear`, and no event carries text.
An agent also has no way to learn from the CLI how to point at code. This unit adds the event, a CLI command for it, and help text that an agent can read.
It replaces the MCP plan of unit `012`: an agent with a shell needs no MCP server, and the socket stays agent-neutral.

## Specification

The event (`minimap_core::events`):

```json
{"v":1,"type":"note","path":"src/a.clj","lines":[40,58],"text":"Retries start here.","source":"claude-code","session":"0f9e8d7c-…"}
{"v":1,"type":"clear","layer":"notes"}
```

- `Event::Note { path, lines, text, source, session }`. `path` is required and not empty. `lines` is optional, as on an `edit`: two 1-based lines, put in order.
  `text` is optional (an empty note marks lines without words). A text longer than `NOTE_TEXT_MAX` (4,000 characters) is cut to 3,999 characters and `…`.
  `source` and `session` are optional, as on trail events.
- `Event::ClearNotes`: `{"type":"clear","layer":"notes"}`. A `clear` without a layer, or with `"trail"`, stays `Event::Clear`. Another layer stays `None`.
- `Event::to_line`, `Event::normalized` (a note on a path outside the repo is `None`), and `parse` handle both.
- `Trail::record` ignores both: a note is not a read, a search, or an edit. The TUI ignores both (no change to its code).

The notes model (`minimap_core::notes`, new), for every front end:

- `Note { id, path, lines, text, source, session }`. `id` counts from 1 in arrival order and never repeats in a session.
- `Notes`: the notes in arrival order, at most `notes::MAX` (1,000; the oldest drop).
  `record(&Event)` adds a note for `Event::Note` and returns its id, and empties the list for `Event::ClearNotes`.
  `all()`, `get(id)`, `ordinal(id)` (the 1-based place in the current list), `of_path(path)`,
  `next(after: Option<u64>)` (`None`: the first note; an id that is gone: the first note), and `previous(before: Option<u64>)` (`None`: the last note).

The command (`minimap note`, in the `minimap` binary):

```text
minimap note [--lines A-B] [--repo PATH] <PATH> [TEXT]
minimap note --clear [--repo PATH]
```

- `PATH` resolves against the current directory. `TEXT` is the note; `-` reads it from stdin. Without `TEXT`, the note has no text.
- `--lines A-B`, or one line `--lines 40`, as `minimap send --edit` takes them.
- `--repo` defaults to the repo of `PATH`'s directory, else of the current directory.
- `--clear` sends `Event::ClearNotes`.
- Exit status: 0 when the window's socket took the line. 1, with one line on stderr, when no minimap listens on the repo
  (`minimap note: no minimap shows <root>. Start minimap-gui there.`), or when `PATH` is outside the repo. 2 for a usage error.
  `minimap note` reports errors, because an agent reads them; `minimap send` stays silent for hooks.
- `minimap note --help` explains notes to an agent in its long help: what the window does with a note, the guided-review recipe
  (`--clear`, then one note per stop in reading order, then tell the person to press `n`), and the exit status.
- `minimap --help` ends with: `Agents: run "minimap note --help" to point a person at code on the map.`

Docs:

- `adapters/claude-code/README.md`: a section `Notes for a guided review`: the recipe, a line for a repo's `CLAUDE.md`,
  and the permission rule `Bash(minimap note:*)`, so Claude Code does not ask before each note.
- `docs/design.md`: `Architecture` (the event), `Layers` (layer 2 is now notes), and `Vocabulary` (`note`).

## Validation

- [x] Core tests: a note parses with and without lines, text, source, and session; `lines` comes out in order; a missing or empty path is `None`;
  a 5,000-character text is cut to 4,000 characters with `…`; `to_line` and `parse` round-trip; `{"type":"clear","layer":"notes"}` is `ClearNotes`,
  and `"layer":"other"` is `None`; a note outside the repo normalizes to `None`; a listener receives a note from `events::send`.
- [x] Core tests: `Notes` ids, ordinals after a clear, `of_path`, `next` and `previous` at both ends and from a cleared id, and the cap.
- [x] Core test: `Trail::record` ignores `Note` and `ClearNotes`.
- [x] CLI tests (`crates/tui/tests/send.rs` style, the binary with a listener on a temp repo): `minimap note PATH --lines 9-4 TEXT` sends lines `(4, 9)`;
  `-` reads stdin; `--clear` sends `ClearNotes`; no listener exits 1 with the message; a path outside the repo exits 1; `--help` names the recipe.
- [x] Real-repo test: notes on 20 real-repo files, by absolute and by relative path, normalize to their index paths and round-trip through `to_line`.
- [x] All `AGENTS.md` validation levels 1–3 pass.

## Result (2026-10-09)

- Core: `events::Event::Note` and `Event::ClearNotes`, `NOTE_TEXT_MAX`, `note_text`, and `line_pair` for edits and notes; `Trail::record` ignores both.
  `minimap_core::notes`: `Note`, `Notes` (`record`, `clear`, `all`, `get`, `ordinal`, `of_path`, `next`, `previous`), `MAX` 1,000.
- CLI: `minimap note` (`NoteArgs`, `note`, `lines_arg`, `NOTE_HELP`) and the `minimap --help` line. The GUI ignores both events until unit `053`.
- Docs: `adapters/claude-code/README.md` `Notes for a guided review`; `docs/design.md` `Vocabulary`, `Layers`, `Architecture`.
- Tests: `crates/core/tests/events.rs` (5 new), `crates/core/tests/notes.rs` (5), `crates/tui/tests/note.rs` (3), and the real-repo test
  `notes_on_target_files_normalize_and_round_trip` (40 notes normalized). `claude_code.rs` matches any other event in its panic arm.
- Validation: levels 1–3 pass: 447 tests, and the real-repo suite (`--no-fail-fast`) 39 passed, 0 failed.

## Scope

- No change to the TUI's screen: it ignores notes.
- No persistence: a note sent while no window runs is lost, and the command says so.
- No MCP server. Unit `012` stays `draft` until the user drops it or asks for MCP.

## Context

- `crates/core/src/events.rs`, `crates/core/src/trail.rs`, `crates/tui/src/main.rs` (`send`, `parse_lines`), `crates/tui/tests/send.rs`.
- Unit `012` (`roadmap/archived/012-focus-mcp.md`) planned the same idea as an MCP tool for the TUI.
