---
title: "Focus MCP tool and focus layer"
status: draft
description: "Let the agent highlight and annotate the files of a concept through an MCP tool, and show them as the focus layer"
created: 2026-09-25
updated: 2026-09-25
tags: [milestone-5, adapter, mcp, layers]
priority: medium
---

# Focus MCP tool and focus layer

## Intent

When the agent explains a concept, the user sees which files carry it, in order, with a note on each.
This ties the chat explanation to places on the map.

## Specification

- `minimap mcp`: a stdio MCP server built with `rmcp` (3.x on 2026-09-24).
  - Tool `map_focus(concept: string, items: [{path: string, line?: integer, note: string}])`. It sends a `focus` event (unit `010`) and returns the number of items that matched indexed files.
  - Tool `map_clear_focus()`.
- Registration, run by the user: `claude mcp add --transport stdio minimap -- minimap mcp`.
- Focus layer (`2`), per `docs/design.md` `Layers` and `mockups/v3-template.html:365-401`, `:504`, `:576`:
  circled numbers `①`–`⑨` on tiles, focus background `#302a10`, the top-bar banner `◆ CC focus: <concept> · N files · tab cycles`, a note popup on hover, a CC NOTE section in the panel, and `◀` at the focus line in the shape.
- Tab and Shift+Tab cycle the focus files and center the map on each.

## Validation

- [ ] Unit test: the MCP tool call maps to the expected focus event.
- [ ] Integration test: an MCP client over stdio calls `map_focus`; the TUI state holds the items.
- [ ] Snapshot test: the seeded focus from `mockups/v3-template.html:786-792` on the quayside fixture.
- [ ] Manual: ask Claude Code to explain a concept in the target repo; the focus appears.

## Scope

- One focus at a time; a new focus replaces the old one.
- No history of past focuses.

## Context

- `docs/design.md` `Layers`, `Keys`.

## Open Questions (draft only)

- How does the agent learn to call `map_focus`? The tool description alone, or a line in the target repo's agent instructions? That is the user's decision.
- More than 9 items: cap at 9, or use a different marker after 9?
