---
title: "Event socket and trail layer"
status: done
description: "Accept JSON-line events on a local socket and show agent reads, searches, and edits on the map, the file tree, the overview, and the panel"
created: 2026-09-25
updated: 2026-09-30
tags: [milestone-4, core, events, layers]
priority: high
---

# Event socket and trail layer

## Intent

The trail is the core of the idea: the user sees where the agent works while it works.
The socket is agent-neutral, so Claude Code is one adapter among possible others (unit `011`).
This revision fits the trail to the v5 map: state is a cell background, the overview shows the trail as colored dots (M3 review, 2026-09-28),
and the tree marks trail files with `▌`.

## Specification

Protocol, one JSON object per line:

```json
{"v":1,"type":"read","path":"src/a.clj","source":"claude-code"}
{"v":1,"type":"grep","paths":["src/a.clj","src/b.clj"],"source":"claude-code"}
{"v":1,"type":"edit","path":"src/a.clj","lines":[40,58],"source":"claude-code"}
{"v":1,"type":"clear","layer":"trail"}
```

- `v` must be 1; a line with another `v` is dropped. An unknown `type` is dropped. `source` and `lines` are optional.
- A path is absolute or repo-relative. An absolute path under the repo root becomes repo-relative; the root matches in its given and its canonical form
  (macOS `/var` is `/private/var`). `.` and `..` segments resolve by text. A path outside the repo is dropped.
- The trail keeps events by path. A path that the index does not have yet is kept, and shows when a re-index (unit `034`) adds it.

In `minimap-core`, module `events`:

- `Event` and `parse(line) -> Option<Event>`.
- `socket_path(root)`: `$TMPDIR/minimap-$USER-<16 hex digits of the FNV-1a hash of the canonical repo root>-events.sock`.
  When that path is longer than 103 bytes, `/tmp` takes the place of `$TMPDIR`. The hash function moves from `crates/tui/src/editor.rs` to the core,
  and the nvim socket path of unit `031` does not change.
- `Listener::spawn(root)`: binds the socket with mode 0600 and reads lines on a thread. Each parsed event goes to a channel.
  - When the socket file exists and a connect succeeds, another minimap listens: return `AlreadyListening`, and bind nothing.
  - When the connect fails, the file is stale: remove it and bind.
  - Dropping the listener removes the socket file.
- `send(root, &Event)`: connects and writes one line, with a 50 ms write timeout. It returns an error; callers decide to be silent.

CLI:

- `minimap send [--repo PATH] '<json>'`, `minimap send --read <path>`, `--grep <path>...`, `--edit <path> [--lines A-B]`, `--clear`.
  Relative paths resolve against the current directory. The repo root comes from `--repo`, else the current directory.
- `send` exits 0 with no output in every case: no repo, no listener, a bad path. It finishes in under 50 ms, so an adapter never slows the agent.

Trail state (the TUI):

- Per path: the strongest kind seen (edit, then read, then grep), the sequence number and time of its newest event, and the lines of its newest edit.
- A path is fresh while one of its events is among the 3 newest events.
- `clear` and `x` empty the trail. The trail lives for the session and has no size limit.

Drawing, while layer 1 is on (`docs/design.md` `Layers`, `Color`):

- State colors: read `#52b3e0`, grep `#d6a63d`, edit `#e2725b`. The first two are the v3 hues (`#6cb6d9`, `#c9a14a`) raised to the M2 rule's HSL saturation of at least 60 %.
- Map: a trail tile's cell background is its kind's color at 45 % over the pane when fresh, else at 20 %.
  The background priority becomes: selection pulse, hover, search match, CC focus, CC edit, mine, CC read, CC grep, git status, heat.
- File tree: a trail file's row shows `▌` in its kind's color in the row's first cell.
- Overview: a braille cell with a trail file draws its dots in the kind's color, edit before read before grep. This holds inside the viewport too.
  The viewport fill and edge do not change. Heat and mine do not show in the overview.
- Selection panel, after the file's header rows: `CC edited lines 40–58 · 2m ago`, `CC read this file · 5s ago`, or `CC found this file by search · 1m ago`, in the kind's color.
  The label is `CC` for source `claude-code`, the source string for another source, and `agent` without one. The age uses the panel's age format, with `s` and `m` below one hour.
- Status bar: a `1 ● trail` toggle before the heat toggles.

Keys (through the unit `033` keymap, mode `map`): `toggle_trail` (`1`) and `clear_trail` (`x`). The help lists them.

App and main loop:

- The binary starts the listener in repo mode. `AlreadyListening` flashes `Another minimap shows the trail for this repo.` and the map runs without a trail.
- The main loop checks the event channel at least every 100 ms, as for the unit `034` watcher, and redraws when events arrived.

`docs/design.md`: the `Layers` row for layer 1, `Overview` (the trail colors the dots; it was "no layers"), `Color` (the priority and the trail colors),
`Keys` (`1`, `x`), `Screen regions` (the status bar toggle), and `Architecture` (the event socket is in the core).

## Validation

- [x] Unit tests: parsing each type; `v` other than 1; an unknown type; a missing `path`; `lines` given and left out.
- [x] Unit tests of path normalization: repo-relative, absolute under the root, absolute under the canonical root, `./` and `..`, outside the root.
- [x] Unit tests: the socket path for a long `$TMPDIR` falls back to `/tmp`; the nvim socket path of unit `031` is unchanged.
- [x] Integration test: a listener on a temporary repo; `minimap send` events arrive in order; a second listener gets `AlreadyListening`;
  a stale socket file is replaced; dropping the listener removes the file.
- [x] Test: `minimap send` with no listener exits 0, prints nothing, and takes under 50 ms.
- [x] App tests: the strongest kind wins per path; freshness moves with the 3 newest events; `x` and `clear` empty the trail;
  layer 1 off hides the trail and keeps it; an event for a path not in the index shows after an index update adds the path.
- [x] App test: trail events and `x` move no tile and do not change the camera.
- [x] Snapshot tests at 150 × 45 on quayside with a seeded trail like `mockups/v3-template.html:865-870` (one grep of 3 files, 4 reads, 1 edit):
  the map with fresh and older trail backgrounds, the tree marks, the overview dots, and the panel's trail line for the edited file.
- [x] Unit test: the read and grep colors have HSL saturation of at least 60 %.
- [x] Visual check (see `AGENTS.md` `Visual check`) on the real target repo: from another herdr pane, `minimap send --read`, `--grep`, and `--edit` target files
  (read only; nothing writes to the target). Each event shows on the map, the tree, and the overview within 100 ms. `x` clears it. Read `--format ansi` for the colors,
  and look at an edit tile next to the heat layer (layer 4), because the heat color is still under review.
- [x] All `AGENTS.md` validation levels 1–4 pass.

## Result (2026-09-30)

- `minimap_core::events` holds the protocol, the path normalization, the socket path, the listener, and `send`.
  The FNV-1a hash moved there; a test pins the nvim socket name of unit `031` (`/repo` → `…-f7556aa5a4543342.sock`).
- A path matches the root in its given and its canonical form. When neither matches, the canonical form of the path's nearest existing directory
  decides, so a file that does not exist yet still maps through a link such as `/var` → `/private/var`.
- `clear` without a `layer` clears the trail. A `clear` for another layer is dropped, for the focus layer of unit `012`.
- The listener reads one connection at a time, in order, and waits at most 200 ms for a connection's next line.
- `minimap send` took about 5 ms on this machine. The first run of a newly built binary took 0.44 s while macOS checked it; the test skips that run.
  Clap usage errors still print and exit 2; every other problem is silent.
- Layer 1 off hides the trail on the map, in the tree, in the overview, and in the panel. The trail stays, and `1` shows it again.
- The status bar's `1 ● trail` toggle changed every map snapshot's status row, and the help got the row `1 / x  the agent's trail on the map, or clear it`.
  No other snapshot text changed. At 90 columns the `? keys · q quit` hint no longer fits beside the toggles, so it drops out.
- While a trail file is selected, the screen redraws every second so the panel's age stays current.
- Real-repo test (read only): the real repo's largest files, sent as absolute paths, showed on the trail. The frame drew within the frame budget in a dev build, and no tile moved.
- Visual check on the real repo (read only; `minimap send --repo` from this pane): a read, a grep of 2 files, and an edit with `--lines 5-12`
  each showed on the map, the tree (`▌`), and the overview within 100 ms. The panel read `agent edited lines 5–12 · 0s ago`, because `send` flags carry no source.
  `x` flashed `Cleared the agent's trail.` and removed every mark. A second minimap on the same repo flashed `Another minimap shows the trail for this repo.`
- Heat comparison for the user's review: a fresh edit tile draws `#6f3f38` (RGB 111, 63, 56). The age heat steps draw `#6e3156`, `#4e2842`, and `#352133`.
  The edit reads brown-red and the heat reads magenta, at close lightness. Capture with both on screen: `.private/captures/visual-010-heat.ansi`.

## Scope

- No Claude Code integration (unit `011`), no focus events (unit `012`), no mine layer (unit `013`).
- No persistence: the trail lives for the session.
- No glob events: a file-name search lists names only (M3b review, 2026-09-29).
- No follow mode (the selection following the agent) and no event log in the top bar (Quick Ideas).
- No edit-range marks in the shape: unit `030`'s diff marks already show uncommitted changes.

## Context

- v3 trail reference: `mockups/v3-template.html` `trailCol` (fresh = the 3 newest events), `drawTile`, `miniColor`, the panel trail line, and `seed`.
- v5 map and overview: `crates/tui/src/ui.rs` (map cells, `draw_overview`, tree rows, `draw_file_info`, `draw_status`), `crates/tui/src/theme.rs` (`state_bg`, `mix`).
- Socket hash: `crates/tui/src/editor.rs` (unit `031`). macOS `sun_path` holds 104 bytes.
- Decided: the overview shows the trail (M3 review, 2026-09-28). Colored dots, edit before read before grep, and no glob (M3b review, 2026-09-29).
- Defaults taken in the M3b review (2026-09-29): the `-events` socket name; a second minimap on one repo runs without a trail and says why.
