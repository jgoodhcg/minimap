---
title: "Mine layer"
status: draft
description: "Mark the files the user opened this session on the map, after unit 031 opens files in the editor"
created: 2026-09-25
updated: 2026-09-28
tags: [milestone-6, layers]
priority: medium
---

# Mine layer

## Intent

The user's own reading is part of their mental model of the codebase.
The mine layer shows which files they opened this session, beside the agent's trail.

The editor handoff moved to unit `031` (Milestone 3b, M3 review 2026-09-28). This unit keeps the mine layer.

## Specification

- Mine layer (`3`): files opened this session with `e` (unit `031`), and maybe files shown in the viewer (unit `029`), get the mine background `#86c07a` on the map.
- The selection panel shows `You opened this Nm ago`. `c` clears the layer.

## Validation

- [ ] Unit test: an open marks the file, and `c` clears the marks.
- [ ] Snapshot test: the map with the mine layer on.

## Scope

- No nvim-side adapter (Quick Ideas): files opened outside minimap do not count.

## Context

- `docs/design.md` `Layers`, `Color`. Unit `031` for opening files.

## Open Questions (draft only)

- Does viewing a file in the viewer count as "mine", or only `e`?
- Does the overview show the mine layer? The M3 review (2026-09-28) put only the trail and focus in the overview.
- Revise the map rendering against the v5 map before this unit becomes `ready`.
