---
title: "Names that scale with the zoom"
status: done
description: "Give every text a size in the world so it grows and shrinks with the zoom, and size each directory name by its block, so names show at every zoom"
created: 2026-10-01
updated: 2026-10-01
tags: [milestone-g2, gui, core]
priority: high
---

# Names that scale with the zoom

## Intent

G1 review feedback (2026-10-01).
At zoom 0.199 (card width 33 points), the user saw no directory names and expected to see them.
The header row is 18 points high at zoom 1, so a header name there is 2.9 points on the screen, under the 9-point floor.
The user asked whether text can scale without limit in either direction.

Scaling alone does not fix the case: a 2.9-point name is too small to read.
A directory name must also be larger in the world when its block is large, as a map draws a country's name larger than a town's.
Large directories then show their names first, and smaller ones show as the zoom comes in.

## Specification

Text in the world:

- Every text and icon has a size in world points, and its screen size is that size times the zoom. No size has a cap or a floor.
- A text 7 points or larger on the screen draws as text. From 2 to 7 points, it draws as a bar (greeking): a rounded rect as wide as the text
  and half as high as its size, in the text color at 35 %. Below 2 points, it does not draw. Tune these values at the visual check.
- Glyph sizes snap down to a geometric ladder: 7 points times `1.1^n`, with no top end, so the glyph atlas holds a bounded set of sizes.
  When the snap shows as a step during a pinch, scale the text's mesh from its ladder size to its exact size.
- A directory name more than 96 points high on the screen fades out over the next 50 %, so a name never fills the window.
  The breadcrumb still names the place. Card text does not fade: at zoom 6 a card name is about 85 points.
- Card text: the icon, the name, and the line count have sizes in proportion to the card width, with today's proportions at card width 168.
  The name row keeps its world height, so a zoom never moves an element inside a card.

Directory names (`minimap-core` layout, `CARDS` only):

- A directory's name size is the largest at which the name spans 30 % of its block's width, from the font's advance.
  It is at least 0.8 × the header row (14.4 points).
- The block's header band has `ceil(1.25 × name size / PY)` rows, at least `HEAD`. The band rounds up to a `Growth(1.25)` step.
  The block's width does not depend on its band, so the layout stays one bottom-up pass.
- The name sits in its band, and the 1-point rule is at the band's bottom. No name covers a card.
- Root and district names lose their special rules: the 13-point minimum, the drawing above the cards, and the backing.
- A click in a header band selects its directory.

Capture mode adds a step at card width 33, the user's case.

Docs: `docs/design.md` `Layout` (the band rule in the `CARDS` column) and `GUI (prototype)` (text sizes, names, greeking, fades).

## Validation

- [x] Core layout tests on quayside: a block's band follows the formula; a larger block has a name at least as large as a smaller block with the same name length;
  the band changes no card place when the block's width holds; the `CARDS` snapshot changes, and no `CHARS` or `TILES` snapshot does.
- [x] GUI scene tests: a text's screen size is its world size times the zoom; text below 7 points is a bar, and below 2 points is absent;
  every glyph size is on the ladder; a directory name fades from 96 points; no name rect overlaps a card rect.
- [x] A toggle, the selection, and the hover change no card's world rect.
- [x] Real-repo test (read only): at the fit view and at card width 33 on the map center, count the directory names drawn as text and as bars. Print them.
- [x] Rerun the stability report of unit `039` with the bands. Record the numbers.
- [ ] GUI visual check on the real repo (partly met: narrow districts do not name themselves at the fit view; see the Result): at the fit view, every district names itself; at card width 33, the large directories in view name themselves;
  after a zoom to a large directory, its name shows. Keep every PNG under `.private/`.
- [x] `frames.txt` on the target, in a release build: p95 frame CPU time under 8 ms. The font atlas size after the second sweep equals the size after the first.
- [x] All `AGENTS.md` validation levels 1–3 and 4b pass.

## Result (2026-10-01)

- Layout: `LayoutConfig::name_share` (`NAME_SHARE` 0.30 for `CARDS`, 0 for `CHARS` and `TILES`), `LayoutConfig::name_band(w, chars)`,
  `Header::band`, and `Layout::name_size` (per node, in points at zoom 1). `NAME_ADVANCE` is 0.6, `NAME_MIN` 0.8 of the header row, and `BAND_K` 1.25.
  `size()` sets each node's band from its width, after the width, so the layout stays one bottom-up pass. `CHARS` and `TILES` did not change.
- quayside: the bands make the map taller, and most names are larger than the least size. The bands kept their size in every
  block comparison where a block kept its width while a file grew past its step.
- Real repo (read only): the bands take under a fifth of the map, and a card at the fit view is a few points wide.
  A larger share costs more band area and lets more names read at the fit view and at card width 33 (zoom 0.196). 0.30 gave the most readable names
  for a band cost the map can carry; 0.10 and 0.20 left almost every district without a readable name.
- Stability report with the bands (`.private/stability-2026-10-01-bands.md`): the summed moves stayed about the same as without bands.
  Measured, not adopted: name order moved more; rounded widths moved less, for a much larger map.
- Text: `scene::ladder` (`7 x 1.1^n`, no top end), `TEXT_FROM` 7, `GREEK_FROM` 2, `GREEK_H` 0.5, `GREEK_ALPHA` 0.35, `FADE_SPAN` 0.5.
  `FADE_FROM` is 200, not 96: a block fitted in the window gives its name about `776 / chars` points, so at 96 a zoom to a directory with a name
  of 5 characters or fewer faded it out. At 200, every quayside block 200 cells or wider with a name of 4 characters or more shows its name after a zoom to it.
  `Item` has the exact screen `size`, the `glyph` size it rasterizes at, and an `opacity`. `Body::Greek` draws a bar.
  An icon from 2 to 7 points is a square in its type color (the spec names text only).
- The painter lays out each text at its glyph size, then builds one mesh per text, scaled by `size / glyph` around its anchor,
  with the atlas's texel coordinates normalized as egui's tessellator does. It always scales, not only during a pinch, so text never steps.
  It lays out every text of a frame before it reads the atlas size, and the bars paint first, so no glyph enters the atlas after the meshes read it.
- Card text in world points, in proportion to the card width: the name row 18, the icon 16, the name and the line count 14. `CardGeometry` has no caps.
- Headers: the name at the band's left and vertical center, cut to the band's width; the rule along the band's bottom. `Kind::Backing` and the on-top pass are gone.
  `scene::hit` takes the region: an item first, else the deepest band that holds the point.
- Capture mode has a step at card width 33.
- Detail levels: names from card width 40 (was 60) and shapes from 60 (was 110). A card name greeks below card width 84,
  so from 40 to 84 the names are bars, and from 60 the shape shows a card's code where unit `039` left an empty box.
- Real-repo test (read only): the fit view draws some names as text and more as bars; at card width 33 on the card nearest the map center,
  about a third of the names in view read as text, and the rest are bars.
- Deviation: at the fit view, not every district names itself. A few districts read as text, some show as bars, and the narrowest draw nothing (under 2 points).
  A district's name stays inside its band, and a narrow district cannot hold a readable name there: it is only a few points wide at the fit view,
  and a 7-point name of 8 characters is 34 points wide. G1's 13-point district names covered cards to show.
- Headless frame probe (an egui context with no window; scene, paint, and tessellation; the capture's sweep on the real repo, release build):
  p95 under 2 ms in both passes; the atlas was the same size after both passes.
- Visual check on the real repo, in a release build: captures with a short file and with the longest file as `--select` on 2026-10-01 at 19:08 and 19:10
  (before and after the threshold change), and with a large depth-2 directory of a 4-character name on 2026-10-02 at 13:07, after the fade change. PNGs: `.private/captures/040/`.
  At the fit view the root name is a large title, the widest districts read, and deeper names in wide blocks read too.
  At card width 33 the large directories in view name themselves, and small ones are bars. After the zoom to that directory, its name shows white and large.
- `frames.txt` after the threshold change: p95 under 8 ms in both passes for every capture. The font atlas was the same size after both sweeps in every run.
- For the G2 review: a card name keeps G1's proportions, so it shows the same 10 to 13 characters at every zoom, and a long name stays cut at zoom 6.

## Scope

- No label collision rules beyond the bands. Each name stays inside its own band.
- No change to the TUI's map or its text.
- No work on empty space: the user deferred it (G1 review).

## Context

- Decided in the G1 review (2026-10-01): text grows and shrinks with the zoom; directory names show at every zoom where they can be read.
  The user left the method to the agent.
- Unit `038` noted that after a double-click on a large directory, its name could be under 9 points. With the 30 % rule, a fitted block's name is readable.
- Text today: `crates/gui/src/scene.rs` (`LADDER`, `HEADER_K`, `DISTRICT_MIN`, `BACKING_ALPHA`, `NAME_K`, `ICON_MIN`, `ICON_MAX`), `crates/gui/src/text.rs`, `crates/gui/src/fonts.rs`.
- The font atlas measure: `crates/gui/src/capture.rs` and unit `035`'s Result.
- Unit `039` comes first. This unit builds on its layout.
