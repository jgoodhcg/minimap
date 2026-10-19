---
title: "Keymap overrides"
status: done
description: "Let the config bind any key or key sequence to any action in the map, outline, and viewer modes, and show the bound keys in the help and the hints"
created: 2026-09-29
updated: 2026-09-29
tags: [milestone-3c, core, tui, keys, config]
priority: high
---

# Keymap overrides

## Intent

The user wants to change minimap's keys (M3b review, 2026-09-29).
Every key is a fixed `match` in `crates/tui/src/app.rs` and `crates/tui/src/viewer.rs` today, and the help and the hints repeat them as text.
One keymap makes the keys data: the defaults and the config merge into it, the handlers look actions up in it, and the help and hints read it.
This unit goes before Milestone 4, so the new trail keys (`1`, `x`) go into the keymap from the start.
The notation must be intuitive. The user chose VS Code-style key strings and a key-to-action table.

## Specification

Config (`minimap-core` `config`, which stores the strings; the TUI checks them, as for `[editor]`):

```toml
[keys.map]                 # the map and the file tree
"h" = "pan_left"
"a" = "none"               # removes the default binding of a

[keys.outline]             # outline browse (o)
"ctrl+n" = "down"

[keys.viewer]              # the viewer, in file mode and diff mode
"ctrl+d" = "half_page_down"
"g g" = "first_line"       # a space separates the keys of a sequence
"space f" = "find"
```

- `Config` gets `keys: BTreeMap<String, BTreeMap<String, String>>` (mode → key string → action name).
- Modes: `map`, `outline`, and `viewer`. Another table name is a config error.
- A key the config names takes that action. A key the config does not name keeps its default. `"none"` removes a key's binding.
- An action can have several keys. An action with no key left is not reachable by keys; the mouse still works.

Key strings (the TUI, a new module `crates/tui/src/keymap.rs`):

- A key is one character, or a named key, with optional modifiers joined by `+`: `a`, `G`, `/`, `ctrl+d`, `alt+x`, `shift+tab`, `ctrl+shift+left`.
- Modifiers: `ctrl`, `shift`, `alt` (aliases `option`, `opt`, `meta`). Names and modifiers ignore case. A single character is case sensitive.
- Named keys: `enter`, `esc` (alias `escape`), `tab`, `space`, `backspace`, `delete`, `insert`, `home`, `end`, `pageup`, `pagedown`, `up`, `down`, `left`, `right`, `f1`–`f12`, and `plus` for `+`.
- A character with `shift` is its shifted form: `shift+g` is `G`. On a character key, the Shift flag that the terminal reports does not count.
  `shift` with a character that is not a letter is an error: write the shifted character (`?`, not `shift+/`).
- With `ctrl`, a letter's case does not count (`ctrl+D` is `ctrl+d`), because terminals send the same bytes for both.
  `shift+tab` matches crossterm's `BackTab`.
- A space separates the keys of a sequence: `g g`, `] c`, `z t`, `space f`.
- Ctrl+C always quits and cannot be bound. The search picker and the viewer's find prompt keep their fixed keys, because typed characters go into the query.
- In the viewer, `1`–`9` start a count and cannot be bound. `0` continues a count that has started; otherwise it is a key like any other.

Config errors stop minimap at startup, before the terminal changes, with one message per error. The message names the table, the key string, and the problem:

- a key string that does not parse;
- an action name that is not an action of that mode (the message lists the mode's actions);
- two key strings that name the same key, for example `"G"` and `"shift+g"`;
- a key that is both a binding and the first key of a sequence in the same mode (bind the prefix to `"none"` to use the sequence);
- a count digit bound in the viewer.

Actions have snake_case names, and one name means the same kind of action in each mode.
The default keys are today's keys. The full default table, with each action's description, goes into `docs/design.md` `Keys`.
The actions of each mode, with their default keys:

- `map`: `quit` (`q`), `help` (`?`), `search` (`/`), `next_result` (`n`), `previous_result` (`N`), `clear_search` (`esc`),
  `toggle_hidden` (`.`), `toggle_tree` (`t`), `narrow_tree` (`[`), `widen_tree` (`]`), `copy_path` (`y`), `copy_namespace` (`Y`),
  `copy_commit` (`H`), `toggle_age_heat` (`4`), `toggle_churn_heat` (`5`), `browse_outline` (`o`), `open_editor` (`e`),
  `toggle_pulse` (`m`), `toggle_panel` (`p`), `toggle_overview` (`v`), `first_row` (`g`, `home`), `last_row` (`G`, `end`),
  `down` (`j`), `up` (`k`), `page_down` (`pagedown`), `page_up` (`pageup`), `tree_in` (`l`), `tree_out` (`h`), `open` (`enter`),
  `select_left`, `select_right`, `select_up`, `select_down` (the arrows), `pan_up`, `pan_left`, `pan_down`, `pan_right` (`w` `a` `s` `d`),
  and `pan_up_far`, `pan_left_far`, `pan_down_far`, `pan_right_far` (`W` `A` `S` `D`).
- `outline`: `down` (`j`, `down`), `up` (`k`, `up`), `open` (`enter`), `open_editor` (`e`), `close` (`esc`, `o`), `quit` (`q`).
- `viewer`: `down` (`j`, `down`), `up` (`k`, `up`), `half_page_down` (`ctrl+d`), `half_page_up` (`ctrl+u`),
  `page_down` (`ctrl+f`, `pagedown`), `page_up` (`ctrl+b`, `pageup`), `first_line` (`g g`, `home`), `last_line` (`G`, `end`),
  `center_cursor` (`z z`), `cursor_to_top` (`z t`), `cursor_to_bottom` (`z b`), `next_definition` (`] ]`), `previous_definition` (`[ [`),
  `next_change` (`] c`), `previous_change` (`[ c`), `next_blank` (`}`), `previous_blank` (`{`), `scroll_left` (`h`, `left`),
  `scroll_right` (`l`, `right`), `scroll_home` (`0`), `find` (`/`), `next_match` (`n`), `previous_match` (`N`), `toggle_diff` (`=`),
  `copy_location` (`y`), `open_editor` (`e`), `toggle_tree` (`t`), `toggle_panel` (`p`), `help` (`?`), `close` (`esc`, `q`).
- A count applies to the viewer actions that take one today: `down`, `up`, `next_blank`, `previous_blank`, `scroll_left`, `scroll_right`,
  `next_definition`, `previous_definition`, `next_change`, `previous_change`. With a count, `first_line` and `last_line` go to that line.

While the help shows, `esc` and the keys of `help` close it, and the keys of `quit` quit. Other keys do nothing, as now.

Sequences:

- After the first key of a sequence, minimap waits for the next key, with no timeout, as the viewer does now.
- A key that completes no sequence ends the wait and does nothing. `esc` ends the wait.
- A count typed before a sequence applies to it (`3]]`).

Showing the keys:

- The help overlay, the status bar hints, the file tree hint, and the panel hints show the first key bound to each action they name.
  An action with no key drops out of a hint. The status bar layer toggles show no key letter for an action with no key.
- Display form: `ctrl+d` shows as `^d`, `enter` as `⏎`, the arrows as `←` `→` `↑` `↓`, and a sequence without its spaces (`gg`, `]c`).
  `esc`, `tab`, `space`, and the other names show as written.
- The help overlay gets a last line: `minimap keys prints the keymap`.
- `minimap keys [--config PATH]`: prints the effective keymap as TOML that the config accepts, one table per mode, one line per binding,
  with the action's description as a comment. Actions with no key follow as comments (`# no key: copy_commit`). It exits 0.

Architecture:

- `keymap.rs` owns the key parser, the default bindings, the merge, and the lookup (a key sequence → an action, or "wait for more").
- `App::on_key` and `Viewer::key` take actions from the keymap. Their behavior does not change for the default keymap.
- The keymap lives in `App`. The binary builds it from the config before it starts the terminal.

`docs/design.md`: a `Keymap` section with the config shape and the rules above, and an `Action` column in `Keys`.
The `crates/core/src/config.rs` doc comment shows a `[keys.map]` example.

## Validation

- [x] Unit tests of the key parser: `a`, `G`, `shift+g` (same key as `G`), `ctrl+d`, `CTRL+D`, `alt+x`, `option+x`, `shift+tab` (`BackTab`),
  `enter`, `esc`, `escape`, `space`, `f5`, `plus`, `/`, and the sequences `g g` and `] c`. Errors: an empty string, `ctrl+`, `hyper+x`, `foo`.
- [x] Unit tests of the merge: a key gets a new action; `"none"` removes a default; an action with two keys; an unknown mode;
  an action of another mode; `"G"` with `"shift+g"`; a prefix conflict (`"g"` bound with `"g g"`), and the same with `"g" = "none"` (valid);
  a count digit in the viewer. Each error message names the table and the key string.
- [x] Unit test: the default keymap has no conflicts, and every action of each mode has at least one default key.
- [x] All existing key, render, viewer, outline, search, and editor tests pass without change to their inputs or snapshots.
- [x] App tests on quayside: with `"h" = "pan_left"`, `h` pans left and `a` still pans left; with `"a" = "none"`, `a` does nothing;
  with `[keys.viewer] "J" = "half_page_down"`, `J` moves half a page; with `"space f" = "find"`, the two keys open the find prompt;
  `5` then a remapped `down` key moves 5 lines; `3` then `] ]` moves three definitions.
- [x] Snapshot tests at 150 × 45 with a config that remaps `?` to `F1`, `/` to `ctrl+f`, and `4` to `none`: the help overlay, the map's status bar, and the viewer's status bar show the remapped keys.
- [x] Unit test: `minimap keys` output parses as a config, and the parsed keymap equals the one printed, with the defaults and with a remapping config.
- [x] Real-repo check: `minimap keys --config .private/minimap.toml` prints the default keymap, and its output parses back.
- [x] Visual check (see `AGENTS.md` `Visual check`) on the real target repo with a private config in `.private/` that adds `"h" = "pan_left"`
  and `[keys.viewer] "space f" = "find"`: `h` pans, `?` shows `h` next to the pan row, and in the viewer, `space f` opens the find prompt.
- [x] Config error check: run the binary with a config that has an unknown action. It exits non-zero before the screen changes, and the message names the key.
- [x] All `AGENTS.md` validation levels 1–4 pass.

## Result (2026-09-29)

- `crates/tui/src/keymap.rs` holds the parser, the defaults, the merge, and the lookup. `App` and `Viewer` share one `Arc<Keymap>`.
  With the defaults, every existing test and snapshot passed unchanged except the help overlay.
- The help overlay changed in two places: its new `keymap` row, and `esc or q back to the map`, because `esc` is the viewer's first `close` key.
  `esc` comes first so the status bar keeps `esc map`.
- The help title (`esc or ? close`) also comes from the keymap.
- Help rows that name two keys show what is bound: with `"h" = "pan_left"`, the rows read `whsd pan the map`, `WASD pan the map 4 times as far`,
  and `l open or close a directory`, because `h` left `tree_out`.
- An unknown action gets a "Did you mean" suggestion before the list of the mode's actions. On the real repo, the bad config printed
  `Did you mean pan_left?` and exited 1 before the screen changed.
- Visual check on the real repo with `.private/minimap-keys.toml`: `h` panned left by 11 columns, the help showed `h`, and in the viewer `space f` opened the find prompt
  and the status bar showed `space f find`. Captures: `.private/captures/visual-033-*.txt`.
- Real-repo test: `minimap keys --config .private/minimap.toml` printed 91 bindings, and they parsed back to the same keymap.
- A count before Home or End in the viewer now goes to that line, as `12G` does, because Home and End are `first_line` and `last_line`.
- Not checked: other terminals' reports of Shift and Alt; only crossterm on this terminal (herdr) was used.

## Scope

- No mouse remapping. No per-repo keymaps (the `[keys]` tables apply to every repo).
- No leader key, no sequence timeout, and no key-to-command macros (one key or sequence → one action).
- The search picker's and the find prompt's keys stay fixed.
- No new actions. Unit `010` adds `toggle_trail` and `clear_trail` to the `map` mode.

## Context

- Key handlers: `crates/tui/src/app.rs` `on_key`, `on_outline_key`, `on_search_key`; `crates/tui/src/viewer.rs` `key`, `prefixed`, `prompt_key`.
- Hints and help: `crates/tui/src/ui.rs` `TREE_HINT`, `HINT`, `HELP`, `draw_status`, `draw_viewer_status`, `draw_help`, the panel hints in `draw_file_info` and `draw_dir_info`.
- Config: `crates/core/src/config.rs` (`[editor]` is the pattern: the core stores strings, the TUI checks them). CLI: `crates/tui/src/main.rs` (`clap` subcommands).
- Decided in the M3b review (2026-09-29): all modes with sequences; key-to-action tables; VS Code-style key strings, because the user asked for an intuitive notation.
