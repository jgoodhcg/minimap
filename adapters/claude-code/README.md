# Claude Code adapter

This adapter shows Claude Code's reads, searches, and edits on minimap's trail (layer 1).
One async `PostToolUse` hook runs `minimap hook claude-code` after each matching tool call.
The hook reads the payload on stdin and sends trail events to the minimap that shows the file's repo.
It always exits 0 and prints nothing. When no minimap runs on a repo, it sends nothing and returns at once.

| Tool | Trail event |
|---|---|
| `Read` | `read` of `file_path` |
| `Grep` | `grep` of the matched files in the response, for each output mode that lists paths |
| `Edit` | `edit` of `file_path`, with the changed lines from the response's `structuredPatch` |
| `Write` | `edit` of `file_path`, without lines |
| `NotebookEdit` | `edit` of `notebook_path` |
| `Glob`, `Bash`, other tools | nothing |

`MultiEdit` maps like `Edit` when a Claude Code version still sends it.
Each path goes to its own repo's minimap. A session in one repo that reads another repo's files marks the other repo's map.
Each event carries the payload's `session_id`, so the window's agent history can show which session sent it.

## Install

1. Install the binary from the minimap repo. It goes to `~/.cargo/bin/minimap`.

   ```sh
   cargo install --path crates/tui --locked
   ```

2. Merge [`settings.json`](settings.json) into the user settings, `~/.claude/settings.json`.
   When the file already has a `hooks.PostToolUse` list, add the entry to that list.
   The user settings apply to every session. This is safe, because the hook does nothing where no minimap runs.
   The command runs `minimap` from `PATH`. When `~/.cargo/bin` is not on Claude Code's `PATH`, write the full path to the binary.
3. In Claude Code, run `/hooks` and find the `PostToolUse` entry with the matcher `Read|Grep|Edit|MultiEdit|Write|NotebookEdit`.
4. Run `minimap` in a repo, and let Claude Code read a file in that repo. The file shows on the trail.

## Notes for a guided review

The hook shows what Claude Code does. A note shows what Claude Code wants the person to look at (unit `052`).
Claude Code sends notes with `minimap note` through its Bash tool, and `minimap note --help` tells it how.

1. Run `minimap-gui` in the repo.
2. Ask Claude Code to guide the review, for example: "Walk me through this PR in minimap. Run `minimap note --help` first."
   It clears the old notes, then sends one note per stop in reading order:

   ```sh
   minimap note --clear
   minimap note src/retry.rs --lines 40-58 "Retries start here. The delay doubles each time."
   ```

3. Press `n` in the window to step through the notes, and `N` to go back.

To make Claude Code use notes without a reminder, add a line to the target repo's `CLAUDE.md`:

```markdown
- When you explain code to me, point at it with `minimap note` (see `minimap note --help`).
```

Claude Code asks before each Bash command that it has no permission rule for.
To let it send notes without a prompt, add this rule to `permissions.allow` in `~/.claude/settings.json`:

```json
"Bash(minimap note:*)"
```

`minimap note` exits 1 with a message when no minimap shows the repo, so Claude Code can tell the person to start the window.
The hook does not send Bash commands to the map, so a note is never also a trail event.

## Debug

Add `--log DIR` to the command, for example `minimap hook claude-code --log /tmp/minimap-hook`.
The hook then also writes each raw payload to `DIR/<tool>-<tool_use_id>.json`.
Remove `--log` again when you are done, because the log grows with every tool call.

## Remove

1. Delete the entry from `~/.claude/settings.json`.
2. Optional: remove the binary with `cargo uninstall minimap-tui`.
