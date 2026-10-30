---
title: "GUI glow"
status: done
description: "Make each agent event visible on the map: a bright halo on the file's card that holds for 1 s and fades out over 3 s, with a ripple and an edge marker for an off-screen card"
created: 2026-10-07
updated: 2026-10-07
tags: [milestone-g6, gui]
priority: high
---

# GUI glow

## Intent

At the try-out on 2026-10-07 (11:41), a real Claude Code session read 7 files, and the user saw no change on the map.
The read color `#52b3e0` is blue like the ClojureScript type color `#7eafc8`. In a repo where most files are ClojureScript,
a few blue specks among many blue blocks do not show at the fit view. Only the 3 newest events are fresh, and the files were off screen.
The user asked (11:50): "While the agent is reading I want to see easily visible file highlights in the main visual map area that are more visible than focusing.
It should kind of glow for a few seconds then fade."
So the glow must differ from the cards by more than hue: by brightness, by size, and by motion.

## Specification

The model:

- `GuiApp::glows`: per path, the kind and the egui time of its newest event. Each path of each trail event starts or restarts its glow.
  The glow has the event's kind, not the trail's strongest kind: a read after an edit glows in the read color.
- `scene::glow_strength(age)`: 1 for the first `GLOW_HOLD` (1.0 s), then a smoothstep down to 0 over `GLOW_FADE` (3.0 s). At 2.5 s it is 0.5; from 4.0 s it is 0.
- A frame drops the glows that ended. While a glow runs and layer 1 is on, the frame requests a repaint, so the glow animates.
  With no glow running, the window repaints only on input, as before: an idle window uses no CPU.
- Layer 1 off hides the glows and stops their repaints. `x` clears them with the trail. A re-index (unit `048`) keeps them by path,
  so a file that the agent creates glows when its card appears.

The draw items, in screen points, after every card and header, so a glow draws above the map. They never move a card, and a hit test ignores them:

- The glow box: the card's rect, grown about its center to at least `GLOW_MIN` (36) points wide and high. So a card 4 points wide at the fit view glows 36 points wide.
- Halo (`Kind::Glow`): the kind's color at 85 % opacity times the strength at the box's edge, fading to nothing `GLOW_REACH` (32) points out.
  At the block level it also fills the box. From the icon level the inside stays clear, so the card's content shows.
- Core (`Kind::GlowCore`): a near-white color (`WHITE` 75 % over the kind's color) at the strength's opacity.
  At the block level, it fills the card's rect, grown about its center to at least `GLOW_CORE_MIN` (8) points.
  From the icon level, it is a ring of `GLOW_RING` (3) points around the card, so the card's name and shape stay readable.
- Ripple (`Kind::Ripple`): for `RIPPLE_SECS` (0.9 s) after the event, a 2-point ring around the glow box that grows by up to `RIPPLE_GROW` (48) points
  (ease-out) while its opacity falls from 1 to 0, in the kind's color.
- Edge marker (`Kind::GlowEdge`): when the card is outside the map region, a dot of `EDGE_DOT` (7) points radius in the kind's color with a white center,
  at the region's edge (inset by 14 points) on the line from the region's center to the card's center, at the strength's opacity.

## Validation

- [x] Scene tests on `quayside`, with no window: the strength at 0, 1.0, 2.5, and 4.0 s after the event (1, 1, 0.5, 0);
  a glow at the fit view is at least 36 points wide; the ripple shows at 0.3 s and not at 1.0 s; an off-screen card gives an edge marker inside the region;
  the hit test at a point on the halo outside the card returns no entry; no card's rect changes.
- [x] App-state tests through `GuiApp::record_event` and `on_event`: a second event restarts a glow; `1` hides it; `x` clears it; a re-index keeps it.
- [x] Headless frames (`ctx.run_ui` and `ctx.tessellate`): a frame while a glow runs requests a repaint, and a frame after it ended does not.
- [x] The headless frame probe on the real repo with 50 marks and 50 running glows: p95 under 8 ms, and the atlas does not grow on the second sweep.
- [ ] GUI visual check: the user's try-out after 13:00 with a real Claude Code session. No window opens before then.
- [x] All `AGENTS.md` validation levels 1–3 and 4b pass.

## Result (2026-10-07)

- GUI: `scene::Glow`, `glow_strength`, `glows` (the draw items), and the constants `GLOW_HOLD` 1.0 s, `GLOW_FADE` 3.0 s, `GLOW_SECS` 4.0 s, `GLOW_MIN` 36,
  `GLOW_ALPHA` 0.85, `GLOW_REACH` 32, `GLOW_CORE_MIX` 0.75, `GLOW_CORE_MIN` 8, `GLOW_RING` 3, `RIPPLE_SECS` 0.9 s, `RIPPLE_GROW` 48, `RIPPLE_W` 2,
  `EDGE_DOT` 7, and `EDGE_INSET` 14. `Kind::Glow`, `GlowCore`, `Ripple`, and `GlowEdge`; `Body::Halo` and `Body::Dot`; `paint::halo` (one mesh per halo).
  `GuiApp::glows` and `glowing`; `record_event` starts or restarts the glows; `clear_trail` and a `clear` event clear them; `items` adds them with layer 1.
  The frame drops the ended glows and requests a repaint while one runs. `color::with_alpha`.
- Deviation: the halo is a mesh that fades from the glow box outward, not a blurred fill (`RectShape::with_blur_width`), so from the icon level the card's inside stays clear.
  The core is 75 % white, not 65 %: at 65 % the read core's red channel was 187, which reads as light blue.
- Tests: `crates/gui/tests/glow.rs`, 8 tests with no window: the strength (1, 1, 0.5, 0); at the fit view a filled glow box of at least 36 points,
  a near-white core, the ripple at 0.3 s and not at 1.0 s, and the halo's opacity at half strength; near, a ring and a clear inside; an off-screen card's edge dots
  inside the region; no hit and no card rect changes; a restart, `1`, `x`, and a `clear` event; a re-index keeps a glow; headless frames repaint while a glow runs
  and not after it ends (egui paints one more frame after the last request).
- Headless frame probe on the real repo (release, atlas limit 8,192), with 50 trail marks and 50 running glows restarted at each pass:
  p95 under 2 ms in both passes, for a short file and for the longest file, at 1 and 2 pixels per point. The atlas did not grow on the second sweep.
- Real-repo suite of the GUI: 7 passed. `a_hook_payload_reaches_the_windows_trail_on_the_target` did not run: the user's own window (started 11:39) listens on the repo's socket.
- GUI visual check: not run. No window opens before 13:00; the user's try-out with a real session is the check.

## Scope

- No glow in the tree rows or the selection panel. No change to the trail's tint, its freshness rule, or its colors.
- No camera move on an event (follow mode stays a Quick Idea).

## Context

- The trail: unit `047` (`GuiApp::record_event`, `update_tint`; `scene::card`, `Map::tint`). The live index: unit `048` (`apply_index`).
- The halo is one mesh with the color at its inner corners and none at its outer corners (`paint::halo`).
