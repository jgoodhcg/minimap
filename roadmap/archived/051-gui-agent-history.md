---
title: "GUI agent history"
status: done
description: "List every agent event that this window received in a collapsible strip below the map: time, kind, file, and session; a click selects the file and zooms to it"
created: 2026-10-07
updated: 2026-10-07
tags: [milestone-g6, gui]
priority: high
---

# GUI agent history

## Intent

The user asked (2026-10-07, 11:50): "We should also have an agents history panel that shows any events this instance of minimap has picked up
for this repo that we are targeting. The events should list files with some metadata about time and some identifiers for the session.
If I click on an event I want to focus the related file. If the other panels are open, or when they are open, I would see that file selected."
The glow (unit `049`) fades in seconds, and a file can be off screen or inside a closed tree directory. The list keeps every event.

## Specification

Place and size:

- A strip at the bottom of the middle column, below the map region and between the tree and the panel, on `SIDE`, with a 1-point rule in `RULE` along its top.
  It is 9 rows of 20 points (180 points): a head row and 8 event rows. The map region keeps at least 160 points of height; in a short window the strip gets less.
- `r` shows or hides it; it shows at startup. Hiding or showing it never moves a card, and the map content keeps its place on the screen (`Motion::keep_on_screen`).
- Text at 13 points, as in the tree and the panel, so the zoom never changes it.

Content:

- Head row: `AGENT HISTORY` (`LABEL`, bold), then at the right `N events · K sessions` (`FAINT`).
- One row per path of each trail event, newest first: the local clock time (`11:41:07`), the kind (`read`, `search`, `edit`) in its trail color,
  the repo-relative path (cut at the left with `…` when it does not fit), an edit's lines (`40–58`, `DIM`), and at the right the session's first 8 characters
  (`INK2`), or `—` (`FAINT`) for an event without a session (unit `050`).
- Without events: `No agent events yet. Claude Code's reads, searches, and edits show here.` When another minimap listens:
  `Another minimap shows the trail for this repo.`
- The list keeps every event of this window, at most 5,000 rows (the oldest drop). `x` does not clear it. A `clear` event does not clear it.
- A row of the selected file is `CURSOR` with a white path. A row under the pointer is `TILE_HI`, and its file's card is hovered on the map.
- A row for a path with no card (deleted, excluded, or ignored) shows its path in `FAINT` and does not click or hover.

Input:

- A click on a row selects its file through `GuiApp::select` (the one selection), so the tree opens to the file and the panel shows it,
  and a hidden tree or panel shows it when it shows again. The map zooms to the card, as `f` does: the user asked to "focus the related file".
  Changed after the try-out: see below.
- A press records the row's event, not its screen place, so an event that arrives between the press and the release does not change what the click selects.
- The wheel over the strip scrolls its rows (3 per notch, trackpad points by the row height) and never moves the map. A press on it starts no pan.

## Validation

- [x] App-state tests on `quayside` with `record_event` and `on_event`, no window: rows newest first with the kind, the path, and the session (`—` without one);
  a grep of 3 paths gives 3 rows; a click selects the file, opens the tree to it, and zooms to the card; a row for an unknown path does not click;
  a press, then a new event, then the release selects the pressed row's file; `r` hides and shows the strip, and no card's screen place changes;
  the wheel scrolls the rows and does not move the camera; `x` keeps the rows.
- [x] Unit test: the clock format for a UNIX time and an offset (`11:41:07`).
- [x] A headless frame draws the head row and the event rows.
- [x] The headless frame probe on the real repo with the strip showing 50 events: p95 under 8 ms, and the atlas does not grow on the second sweep.
- [ ] GUI visual check: the user's try-out after 13:00. No window opens before then.
- [x] All `AGENTS.md` validation levels 1–3 and 4b pass.

## Result (2026-10-07)

- GUI: `crates/gui/src/activity.rs`: `Record`, `ActivityView`, `Facts`, `Strip`, `build`, `paint`, `file_of`, `max_top`, `kind_word`, `short_session`, `fit_left`,
  and `ACTIVITY_ROWS` 8, `ACTIVITY_H` 180, `REGION_MIN_H` 160, `ACTIVITY_MAX` 5,000, `SESSION_CHARS` 8. `panel::paint_draws` paints both the panel's and the strip's items.
  `GuiApp`: `activity`, `sessions`, `activity_on`, `activity_top`, `activity_view`, `activity_hover`, `set_activity`, `strip`, `click_activity`, and `scroll_activity`;
  `Screen::activity`; `PressOn::Activity`; `r`; the status bar's hint ends `r history`. `record_event` adds one row per path with the local time.
- Core: `history::clock` and `history::local_clock` (`gix::date::Time::now_local_or_utc`; no new crate). At 12:10:20 it read the shell's local time.
- Tests: `crates/gui/tests/activity.rs`, 6 tests with no window: rows newest first with the time, kind, path, lines, and session (`—` without one), a grep of 3 paths as 3 rows,
  `x` keeps the rows; a click on a deep file's row selects it, opens the tree to it, zooms to its card, and the panel names it, and an unknown path's row does not click;
  a press keeps its row when an event arrives before the release; `r` hides and shows the strip and the card keeps its screen place; the wheel scrolls 9 rows,
  stops at the last full page, keeps a scrolled strip's rows when an event arrives, and never moves the camera; a headless frame draws the strip, and a row hovers its card.
  `crates/core/tests/history.rs` `clock_times_of_day_at_an_offset`.
- Changed tests: `panel.rs` (the panel ends at the strip's bottom, not the region's) and `outline_clicks.rs` (hides the strip, so its short card fits the region of unit `046`).
- Headless frame probe on the real repo (release), with the strip showing 151 rows and 50 running glows: p95 under 2 ms in both passes,
  for a short file and for the longest file. The atlas did not grow on the second sweep.
- Real-repo suite (`--no-fail-fast`, read only): 37 passed, 1 failed. `a_hook_payload_reaches_the_windows_trail_on_the_target` failed with `AlreadyListening`:
  the user's own window (started 11:39) listens on the repo's socket. It passed in unit `047`, and units `049`–`051` do not change the listener.
- GUI visual check: not run. No window opens before 13:00; the user's try-out with a real session is the check.

## Change after the try-out (2026-10-07, 13:05)

- The user: "can we not change zoom when focusing on click of an agent history". A click now acts as a click on the file's tree row (`GuiApp::click_row`):
  it selects the file, and the map moves only as far as needed to show the card, with no zoom change. A double-click zooms to it.
- Test: `a_click_selects_the_file_opens_the_tree_and_shows_it_without_a_zoom` (the zoom stays, the map shows the card, and a second click zooms).

## Scope

- No filter or search in the list. No persistence: a new window starts empty. No grouping by session.
- No change to the TUI.

## Context

- The selection: `GuiApp::select`, `zoom_to`. The panel's clickable rows: `panel::Hit`, `PressOn::Panel`. The region: `GuiApp::screen_layout`, `set_screen`.
- Local time: `gix::date::Time::now_local_or_utc` (already a dependency of `minimap-core`), so no new crate.
