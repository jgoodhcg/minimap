---
title: "Session ids on events"
status: done
description: "Carry the agent session's id on trail events: an optional `session` field in the event line, sent by the Claude Code hook"
created: 2026-10-07
updated: 2026-10-07
tags: [milestone-g6, core, adapter, claude-code]
priority: high
---

# Session ids on events

## Intent

The agent history (unit `051`) shows which session sent each event. The hook's payload has `session_id` (a UUID), but the event line (unit `010`) has only `source`.

## Specification

- `events::Event::Trail` gets `session: Option<String>`. `to_line` writes `"session"` when it has one, and `parse` reads it. The version stays `"v":1`:
  `parse` reads only the fields it knows, so an older listener ignores `session`, and an event without it parses with `None`.
- `minimap hook claude-code` (unit `011`) sets the session from the payload's `session_id` on each routed event. A payload without it sends no session.
- `minimap send` (the TUI's CLI) sends no session.
- The trail model does not change: `trail::Mark` has no session.
- The installed hook binary must be built again: the user runs `cargo install --path crates/tui --locked`. The agent prints the command and does not run it.

## Validation

- [x] Core: an event with a session survives `to_line` and `parse`; a line without `session` parses with `None`; a line with an unknown field parses.
- [x] Hook: a fixture payload with `session_id` routes events with that session; a payload without it routes events with none.
- [x] All `AGENTS.md` validation levels 1–3 pass, and `cargo test -p minimap-tui --test render` writes no `.snap.new`.

## Result (2026-10-07)

- Core: `Event::Trail::session`; `to_line` writes it, `parse` reads it, and `normalized` keeps it. `Event::trail` has no session. `Trail::record` ignores it.
- Hook: `route` sets each event's session from the payload's `session_id`. `minimap send` sends none.
- Tests: `crates/core/tests/events.rs` `a_session_survives_the_line_and_an_unknown_field_is_ignored`; `crates/tui/tests/claude_code.rs`
  `routed_events_carry_the_payloads_session`, and the route and hook tests expect the fixture's session `s`.
- Real-repo tests of core and the TUI (read only): 30 passed, 0 failed.
- The installed hook (`~/.cargo/bin/minimap`, from 94e6be4) sends no session until the user runs `cargo install --path crates/tui --locked`.
  Until then the window shows `—` for the session.

## Scope

- No session in the TUI's trail line. Unit `011` stays `ready` for its Grep payloads.

## Context

- `crates/core/src/events.rs` (`Event`, `parse`, `to_line`), `crates/tui/src/claude_code.rs` (`route`).
- Real payloads in `.private/hook-payloads/` have `session_id`, a 36-character UUID. Do not copy them into tracked files.
