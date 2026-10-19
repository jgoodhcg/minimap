---
title: "Repo config applies to worktrees"
status: done
description: "A [repos.\"<path>\"] table matches every linked worktree of that checkout, so one entry configures a repo and all of its worktrees"
created: 2026-10-13
updated: 2026-10-13
tags: [milestone-g8, core]
priority: high
---

# Repo config applies to worktrees

## Intent

The user reviews pull requests in `git worktree` checkouts of the target repo. Each worktree has its own path,
so the `[repos."<path>"]` excludes of the main checkout did not apply, and the map showed the vendor and media files again.
One config entry must cover the main checkout and every worktree of it, including worktrees created later.

## Specification

- `Config::repos_for(root)` matches a `[repos."<path>"]` key when its path is `root`, or when its path is the main working tree
  of the repository that `root` belongs to. The main working tree is the parent of `gix`'s `common_dir()`.
  A key that names a worktree still matches that worktree only.
- `excludes_for` and `generated_for` use the same match. A worktree key and a main key both match a worktree; their rules concatenate in key order, as before.
- A `root` that is not a git repository, or a bare repository, matches by exact path only.
- `config.rs`'s module doc says that a key covers the checkout's worktrees.

## Validation

- [x] Core test: a config keyed by the main checkout gives its excludes and generated rules for a linked worktree, and a key for another path does not.
- [x] `AGENTS.md` validation levels 1–3 pass.

## Result (2026-10-13)

- `config.rs`: `repos_for` canonicalizes `root`, resolves the main working tree with `gix::discover` and `common_dir`, and matches a key against both.
- Test `a_repo_key_covers_its_worktrees` in `crates/core/tests/index.rs`.

## Scope

- No glob keys and no `[map] exclude`. Non-git directories keep the exact match.

## Context

- `crates/core/src/config.rs` (`repos_for`), `crates/core/src/index.rs` (`repo_root`).
