---
title: "Editor handoff with presets"
status: done
description: "e opens the selected file at a line in the user's editor: nvim in a reused herdr tab by default, with vim, VS Code, Emacs, and custom presets"
created: 2026-09-28
updated: 2026-09-28
tags: [milestone-3b, tui, adapter, herdr, editor]
priority: high
---

# Editor handoff with presets

## Intent

The map and the viewer are where the user decides what to read. The editor is where they write.
`e` must put the file in front of them at the right line, in their own editor, without a context switch.
Editors are adapters: nvim is the user's editor, and other editors need only a preset or a command template.

This unit takes the editor part of unit `013`. The mine layer stays in unit `013` (Milestone 6).

## Specification

- Config, in the minimap config file:

  ```toml
  [editor]
  preset = "nvim"     # nvim | vim | vscode | emacs | custom
  place = "tab"       # tab | right | down: where herdr opens a terminal editor
  command = ["zed", "{path}:{line}:{col}"]   # custom only
  terminal = false    # custom only: true runs the command in minimap's terminal until it exits
  ```

  - The default preset comes from the file name of `$VISUAL`, then `$EDITOR`: `nvim`, `vim`, `code`, `emacs` or `emacsclient`. Otherwise it is `nvim`.
  - The default place is `tab`.
  - `command` placeholders: `{path}` (absolute), `{line}`, `{col}`, `{root}`. Each argument is replaced on its own; no shell runs.
- `e` opens the selected file:
  - The line is the viewer's cursor line, else the browsed outline item's line, else 1. The column is 1.
  - On a directory, flash `Select a file to open it.` On a binary or missing file, flash why.
  - After an open, flash `Opened <path>:<line> in <editor>`. On a failure, flash the error's first line.
- Presets:
  - `nvim` under herdr (`HERDR_ENV=1`, with `HERDR_WORKSPACE_ID` and `HERDR_PANE_ID` set):
    - The first open creates the place.
      For `tab`: `herdr tab create --workspace $HERDR_WORKSPACE_ID --cwd <root> --label "minimap editor" --focus`.
      For `right` or `down`: `herdr pane split $HERDR_PANE_ID --direction <right|down> --cwd <root>`.
      Read the new pane and tab IDs from the JSON output.
      Then `herdr pane run <pane> "nvim --listen <socket> +<line> -c 'normal! zt' <path>"`.
    - Later opens, when `nvim --server <socket> --remote-expr 1` succeeds:
      `nvim --server <socket> --remote-send "<C-\><C-N>:drop <path> | call cursor(<line>, 1) | normal! zt<CR>"`.
      Then focus the place: `herdr tab focus <tab>` for `tab`, `herdr pane focus --pane $HERDR_PANE_ID --direction <right|down>` for a split.
    - When the socket does not answer but the pane exists, run nvim in the pane again. When the pane is gone, create the place again.
    - The socket is `$TMPDIR/minimap-$USER-<16 hex digits of the FNV-1a hash of the repo root>.sock`.
  - `vim` under herdr: the first open runs `vim +<line> -c 'normal! zt' <path>` in the place.
    Later opens, while `herdr pane process-info --pane <pane>` shows `vim` in the foreground:
    `herdr pane send-keys <pane> Escape`, then `herdr pane send-text <pane> ":drop <path> | <line> | normal! zt\r"`.
    (`/usr/bin/vim` on the user's machine is `-clientserver`, so `vim --remote` is not available.)
  - `nvim` or `vim` without herdr: suspend the TUI, run `<editor> +<line> -c 'normal! zt' <path>`, wait for it to exit, then restore the TUI and redraw.
  - `vscode`: run `code --goto <path>:<line>:<col>` detached. It reuses the open window.
  - `emacs`: run `emacsclient -n +<line>:<col> <path>` detached. It needs `(server-start)` in the running Emacs.
  - `custom`: `terminal = false` runs `command` detached. `terminal = true` suspends the TUI and runs it, as for nvim without herdr.
- Architecture:
  - `crates/tui/src/editor.rs` builds each preset's steps as argument vectors, with no side effects, so tests check them.
  - The binary installs the runner, as it installs the clipboard: `App` gets an `open` function that takes the path, line, and column.
    A suspend request goes back to the main loop, which leaves the alternate screen, runs the editor, and restores the screen.
  - The editor state (the place's pane and tab, the socket) lives for the session.
- Keys: `e` in the map, in outline browse mode, and in the viewer (in diff mode, a removed row uses the next working-tree line, unit `030`).
  The help overlay and `docs/design.md` `Keys` list it. `docs/design.md` gets an `Editor handoff` section, and `Architecture` lists editors as adapters.
- Unit `013` keeps only the mine layer and names this unit as its opener.

## Validation

- [x] Unit tests: the steps of every preset and place for a first open and a reuse, with a path that contains a space.
- [x] Unit tests: placeholder replacement; the default preset from `$VISUAL` and `$EDITOR`; the line order (viewer, outline, 1); the socket path is under 100 bytes.
- [x] Unit test: a failed step flashes its error, and the app keeps running.
- [x] Real check with nvim and herdr, on the real target repo in the visual-check tab: press `e` on a file in the viewer at a line, then on another file from the map.
  `nvim --server <socket> --remote-expr "expand('%:p') . ':' . line('.')"` shows each file at its line. `herdr tab list` shows one `minimap editor` tab.
  Close the editor tab and the check tab afterwards.
- [x] Real check without herdr: run minimap in a pane with `HERDR_ENV` unset, press `e`. nvim takes the pane at the line, and `:q` returns to the same map.
- [x] VS Code, Emacs, and vim are not installed as servers here. They get unit tests only; report that.
- [x] All `AGENTS.md` validation levels 1–4 pass.

## Result (2026-09-28)

- `serde_json` (already in the lockfile through `minimap-core`) parses herdr's JSON in `minimap-tui`. `tempfile` became a TUI dev-dependency.
- nvim under herdr on the target: `e` in the viewer at line 30 created one `minimap editor` tab and started nvim on
  `$TMPDIR/minimap-$USER-<hash>.sock`. nvim answered with the file's path and line 30.
  `e` on the third outline item of another file went through the socket (line 26), still with one tab.
  After `:qa!` in nvim, `e` restarted nvim in the same pane.
- `zt` keeps the user's `scrolloff = 10`, so the line sits near the top with context above it.
- The tab opens with `--focus`, so each check moved the herdr focus. The user's own workspace and tab were focused again right after each `e`.
- Without herdr (`HERDR_ENV` unset), `e` gave nvim the pane at `61:1`. `:q` returned to the viewer at line 61 and the same map.
- Fixed during the check: the "Back from nvim" flash expired at once, because the app clock was stale after the editor ran.
- vim, VS Code, and Emacs were not checked for real: vim here is `-clientserver`, and `code` and `emacsclient` are not installed. Unit tests cover their steps.

## Scope

- No mine layer (unit `013`) and no nvim-side adapter (Quick Ideas).
- No "open in a new place" key: one reused place per session.
- No editor for directories, and no column other than 1.

## Context

- herdr on 2026-09-28: `tab create --workspace --cwd --label --focus`, `tab focus <tab>`, `pane split [PANE_ID] --direction right|down --cwd`,
  `pane run <pane> <command>`, `pane send-keys`, `pane send-text`, `pane process-info --pane <pane>` (JSON with the foreground processes' names),
  `pane focus --pane <pane> --direction <d>`. The environment has `HERDR_ENV`, `HERDR_WORKSPACE_ID`, `HERDR_TAB_ID`, and `HERDR_PANE_ID`.
- nvim 0.12.4 is installed; `$EDITOR` is `nvim`. nvim's `hidden` is on by default, so `:drop` does not fail on a modified buffer.
- Create herdr tabs in the current workspace (`--workspace`): without it, `herdr tab create` used another workspace (2026-09-25).
- Decided in the M3 review (2026-09-28): the editor handoff moves to 3b with presets, and nvim opens in a new focused tab.
