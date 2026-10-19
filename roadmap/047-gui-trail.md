---
title: "GUI trail layer"
status: done
description: "Show the agent's reads, searches, and edits in the window: tinted cards, the tree's trail mark, and the panel's trail line, from the same event socket as the TUI"
created: 2026-10-07
updated: 2026-10-07
tags: [milestone-g5, gui, core]
priority: high
---

# GUI trail layer

## Intent

On 2026-10-07 the user asked to demo "the whole thing" at 14:00 the same day: the map in the TUI and in the window, with a real Claude Code session's trail.
The TUI has the trail (unit `010`) and the hook (unit `011`). The window has neither the listener nor the layer.
The user runs one front end at a time, so the socket does not need two listeners.

## Specification

Core:

- Move the TUI's trail model (`crates/tui/src/trail.rs`: `Trail`, `Mark`, `FRESH`, `label`, `describe`, `age_phrase`) to `minimap_core::trail`.
  The TUI's `trail` module re-exports it, so the TUI's code and output do not change.
- `events::Listener::spawn_with(root, wake)`: as `spawn`, and `wake` runs on the listener's thread after each event is queued, so a window can repaint.

GUI:

- The window starts a listener on the repo's socket (not with `--from-json`). When another minimap listens, the top bar's right side shows
  `Another minimap shows the trail for this repo.` while nothing is hovered.
- Each frame takes the queued events and records them with the frame's time. A path keeps its strongest kind; the 3 newest events are fresh (the TUI's rules).
- Layer 1, on at startup: the status bar's first toggle is `1 trail`, and the key `1` toggles it. `x` clears the trail, as in the TUI.
- Cards: from the icon level, a trail card's fill is its kind's color (read `#52b3e0`, grep `#d6a63d`, edit `#e2725b`) at 45 % over the card while fresh, else 20 %.
  The hover fill wins over the trail. At the block level, a trail card is its kind's color at 90 % over the pane while fresh, else 60 %, so it shows at the fit view.
- Tree: a trail file's row has `▌` in its kind's color in the first cell, as in the TUI.
- Panel: below `N lines`, the trail line in the kind's color (`CC edited lines 40–58 · 2m ago`), 2 rows at most. While the selected file has a mark, the window repaints every second.
- The trail never moves a card.

## Validation

- [x] Core: the moved trail tests pass; `spawn_with` wakes after an event can be polled.
- [x] The TUI tests pass, and `cargo test -p minimap-tui --test render` writes no `.snap.new`.
- [x] GUI tests on `quayside`, with no window: events tint the cards (fresh and old), the hover wins, the block level uses the strong mix, `1` hides the tint,
  `x` clears it, the tree row has `▌`, the panel has the trail line, and no card's rect changes.
- [x] An integration test: a listener on a temporary repo, `events::send` of a read, and the app records it.
- [x] Real-repo check (read only): the GUI listener on the real repo receives a hook event from this session.
- [x] The headless frame probe with a trail of 50 marks: p95 under 8 ms, and the atlas does not grow on the second sweep.
- [x] GUI visual check: the user's try-out before the demo (see the Result).
- [x] All `AGENTS.md` validation levels 1–3, 4, and 4b pass.

## Result (2026-10-07)

- Core: `minimap_core::trail` holds the TUI's trail model, moved unchanged; the TUI's `trail` module re-exports it. `events::Listener::spawn_with` runs its wake after each queued event.
- GUI: `scene::Map::tint` (per file, the trail's kind and freshness), `Toggles::trail`, `trail_rgb`, and the mixes `TRAIL_FRESH_MIX` 0.45, `TRAIL_OLD_MIX` 0.20,
  `TRAIL_BLOCK_FRESH_MIX` 0.90, and `TRAIL_BLOCK_OLD_MIX` 0.60. `GuiApp`: `trail`, `listener`, `notice`, `now`, `poll_events`, `record_event`, `clear_trail`,
  `update_tint`, and `trail_line`. `tree::Look::trail` draws `▌`; `panel::Facts::trail` is the panel's line. `main.rs` starts the listener with `request_repaint` as its wake.
- The tint is set when events arrive, not per frame, so a frame reads one entry per card.
- Tests: `crates/gui/tests/trail.rs`, 5 tests with no window: fresh and old tints, the strongest kind, the hover, the block mixes, and no card moved;
  `1` and `x`; the panel's line (`CC edited lines 40–58 · 5s ago`); a headless frame with the tree's mark; and a listener on a temporary git repo.
  `crates/core/tests/events.rs` checks `spawn_with`. The TUI's trail tests pass through the re-export.
- Real repo (read only, release build): a Read payload piped into the installed `minimap hook claude-code` reached the window's listener and tinted the card within 10 ms.
- Headless frame probe with 50 read marks and an edit on the selection (release, atlas limit 8,192): p95 under 2 ms in both passes,
  for a short file and for the longest file, at 1 and 2 pixels per point. The atlas did not grow on the second sweep.
- GUI visual check: the user's try-out (2026-10-07, 11:41). A Claude Code session on a real repo read a few files; the hook delivered them,
  and the panel of one read `CC read this file · 6m ago`. The user saw no change on the map: the read color is blue like ClojureScript's type color,
  most of the marks were no longer fresh, and the files were off screen and inside closed tree directories.
  The user asked for a glow that fades and for an agent history panel (the next milestone).

## Scope

- No live index: unit `048`. No trail in an overview (the GUI has none). No second listener per repo.

## Context

- The TUI's trail: `crates/tui/src/trail.rs`, `app.rs` `poll_events`, `trail_mark`, `ui.rs` `draw_tree` (`▌`), `theme.rs` `trail_bg`.
- The socket: `crates/core/src/events.rs` (`Listener`, `send`).
