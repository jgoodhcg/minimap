---
title: "Claude Code hooks adapter"
status: done
description: "Forward Claude Code Read, Grep, Edit, Write, and NotebookEdit tool events to minimap with one async PostToolUse hook"
created: 2026-09-25
updated: 2026-10-30
tags: [milestone-4, adapter, claude-code]
priority: high
---

# Claude Code hooks adapter

## Intent

The passive trail (unit `010`) needs a feed.
Claude Code hooks give one without any action by the agent.
The adapter is a subcommand of the one `minimap` binary, so the user installs nothing else.

## Specification

Facts from the Claude Code docs (checked 2026-09-29, https://code.claude.com/docs/en/hooks.md):
the PostToolUse payload has `session_id`, `cwd`, `hook_event_name`, `tool_name`, `tool_input`, and `tool_use_id`; tool paths are absolute;
command hooks take `"async": true`; edits to the settings file take effect without a restart.
The docs do not give the `tool_response` fields for each tool. Step 3 captures them.

1. `minimap hook claude-code [--log DIR]` reads one PostToolUse payload on stdin and sends events with `minimap_core::events::send` (unit `010`):
   - `Read` → `read` with `tool_input.file_path`.
   - `Grep` → `grep` with the matched paths from `tool_response`, for each `output_mode` whose response lists paths. Relative paths resolve against `tool_input.path`, else `cwd`.
     With no paths in the response, it sends nothing.
   - `Edit` (and `MultiEdit`, if a payload shows it) → `edit` with `file_path`. The lines come from a structured patch in `tool_response` when it has one.
     Otherwise they are the line range of the first `new_string` in the file after the edit. When neither gives lines, the event has no `lines`.
   - `Write` → `edit` with `file_path` and no `lines`. `NotebookEdit` → `edit` with its notebook path.
   - `Glob`, `Bash`, and other tools → nothing (M3b review, 2026-09-29: Glob lists names only).
   - Each path goes to the socket of its own repo, found from the path. So a session in one repo that reads another repo's files marks that repo's map.
   - `--log DIR` also writes the raw payload to `DIR/<tool>-<tool_use_id>.json`. This captures payloads, and it helps with debugging later.
   - It always exits 0 and prints nothing. Malformed stdin sends nothing.
2. `adapters/claude-code/settings.json`: one `PostToolUse` entry with `"matcher": "Read|Grep|Edit|MultiEdit|Write|NotebookEdit"`,
   one command hook `minimap hook claude-code` with `"async": true`.
   `adapters/claude-code/README.md`: install the binary, merge the snippet into `~/.claude/settings.json`, check it with `/hooks`, and remove it.
3. Capture real payloads. This step waits for the user:
   - Print for the user: `cargo install --path crates/tui --locked`, and the snippet with `--log <minimap repo>/.private/hook-payloads`. The user applies them at user level (`~/.claude/settings.json`).
   - After the user confirms, run in this Claude Code session: one `Read`, one `Grep` per `output_mode` (`files_with_matches`, `content`, `count`), one `Glob`,
     one `Edit`, and one `Write`. Point them at scratch files under `.private/hook-scratch/` in the minimap repo, never at the target.
   - Fix the mapping against the captured payloads. Copy them into test fixtures with fake `quayside` paths and fake text in place of private content.
   - Tell the user to drop `--log` from the installed hook.

## Validation

- [x] Unit tests: each fixture payload maps to the expected events; Glob sends nothing; an Edit with a structured patch and one without give the right lines;
  a Grep `content` response with `path:line:` rows gives each path once.
- [x] Unit test: malformed stdin, an empty object, and an unknown tool exit 0 and send nothing.
- [x] Unit test: paths in two repos go to the two repos' sockets; a path in no repo is dropped.
- [x] Integration test: with a unit `010` listener on a temporary repo, piping a fixture payload into `minimap hook claude-code` puts the event in the listener's channel.
- [x] Test: the hook with no listener exits 0 in under 50 ms.
- [ ] Manual check, with the hook installed: minimap runs on the real target repo in the visual-check tab.
  This Claude Code session reads two target files and greps the target (read only). The map, the tree, and the overview show them within 1 s.
- [ ] Manual check on the `minimap self` tab: an `Edit` of a scratch file in the minimap repo shows as an edit with its lines.
- [x] All `AGENTS.md` validation levels 1–4 pass: in every later milestone's validation, and the 12 hook tests pass on 2026-10-30.

## Result (2026-10-30)

- `minimap hook claude-code [--log DIR]` (`crates/tui/src/claude_code.rs`) and `adapters/claude-code/` shipped before Milestone G5.
  The user installed the hook at user level on 2026-10-07. Since then it feeds the TUI trail and the GUI trail, glow, and agent history.
- The tests build their payloads in code, not from captured fixtures. The captured Edit payloads carry the `structuredPatch` that the mapping reads.
- The hook logged 815 payloads to `.private/hook-payloads` from 2026-10-07 to 2026-10-30: Read, Edit, and Write only. No Grep payload arrived.
  The Claude Code session of 2026-10-30 has no Grep or Glob tool. Agents search with `grep` and `rg` through Bash, and read with `cat` and `sed -n`.
  So searches do not reach the map, and the Grep mapping is tested only with payloads built in code.
- Not run: the manual Grep check on the real target repo, and the manual Edit check on the `minimap self` tab.
  The Grep check cannot run without a Grep tool. A Bash adapter, which this unit's scope excludes, is the follow-up (`roadmap/index.md` `Current Focus`).
- The installed hook still has `--log`. Step 3 says to drop it. It stays until the user decides the Bash adapter, because that adapter needs captured Bash payloads.

## Scope

- The agent does not write to `~/.claude/` or to the target repo. It prints the install command and the snippet, and the user applies them.
- No MCP (unit `012`). No Bash parsing: a shell `rg` or `cat` does not show.
- No per-subagent marks: a subagent's tool calls show like the main agent's.

## Context

- Claude Code hooks docs: https://code.claude.com/docs/en/hooks.md. Tool list: https://code.claude.com/docs/en/tools-reference.md.
- Event API: unit `010` (`minimap_core::events`). CLI: `crates/tui/src/main.rs`.
- `AGENTS.md` `Execution Modes`: a global `cargo install` is printed for the user, not run. `Never Run`: no writes inside a target repo.
- Decided in the M3b review (2026-09-29): skip Glob. Default taken: the hook lives in the user-level `~/.claude/settings.json`,
  because `send` is silent when no minimap listens.
