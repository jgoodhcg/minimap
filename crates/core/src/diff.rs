//! Uncommitted changes of one file (unit `030`): its text at a commit, and a line diff against the working tree.

use std::path::Path;

use anyhow::{Context, Result};
use similar::{ChangeTag, TextDiff};

/// A file's text at `rev`: `HEAD` or a full commit id. `None` when the commit has no such file, or the branch is unborn.
/// Invalid UTF-8 is replaced.
pub fn text_at(root: &Path, rev: &str, path: &str) -> Result<Option<String>> {
    let repo = gix::open(root)
        .with_context(|| format!("cannot open the repository at {}", root.display()))?;
    let commit = if rev == "HEAD" {
        match repo.head_commit() {
            Ok(c) => c,
            // An unborn branch has no commit.
            Err(_) => return Ok(None),
        }
    } else {
        let id = gix::ObjectId::from_hex(rev.as_bytes())
            .with_context(|| format!("{rev} is not a commit id"))?;
        repo.find_commit(id)?
    };
    let tree = commit.tree()?;
    let Some(entry) = tree.lookup_entry_by_path(path)? else {
        return Ok(None);
    };
    if !entry.mode().is_blob() {
        return Ok(None);
    }
    let blob = entry.object()?;
    Ok(Some(String::from_utf8_lossy(&blob.data).into_owned()))
}

/// What one diff row is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tag {
    Same,
    Added,
    Removed,
}

/// One row of a diff: a line of the new text, or a removed line of the old text between them.
/// Line numbers are 0-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub tag: Tag,
    pub old: Option<usize>,
    pub new: Option<usize>,
}

/// A line diff: the new text's lines in order, with the removed old lines where they were removed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileDiff {
    pub rows: Vec<Row>,
    /// Runs of changed rows, as row ranges `start..end`.
    pub hunks: Vec<(usize, usize)>,
    pub added: usize,
    pub removed: usize,
}

impl FileDiff {
    /// Diffs two texts line by line (Myers).
    pub fn compute(old: &str, new: &str) -> FileDiff {
        let diff = TextDiff::from_lines(old, new);
        let mut d = FileDiff::default();
        for change in diff.iter_all_changes() {
            let tag = match change.tag() {
                ChangeTag::Equal => Tag::Same,
                ChangeTag::Insert => Tag::Added,
                ChangeTag::Delete => Tag::Removed,
            };
            d.added += usize::from(tag == Tag::Added);
            d.removed += usize::from(tag == Tag::Removed);
            d.rows.push(Row {
                tag,
                old: change.old_index(),
                new: change.new_index(),
            });
        }
        let mut start = None;
        for (i, r) in d.rows.iter().enumerate() {
            match (r.tag == Tag::Same, start) {
                (false, None) => start = Some(i),
                (true, Some(s)) => {
                    d.hunks.push((s, i));
                    start = None;
                }
                _ => {}
            }
        }
        if let Some(s) = start {
            d.hunks.push((s, d.rows.len()));
        }
        d
    }

    pub fn is_empty(&self) -> bool {
        self.added == 0 && self.removed == 0
    }

    /// The new-text line of a row: its own, or for a removed row the next new line (the last line at the end).
    pub fn line_of(&self, row: usize) -> usize {
        self.rows[row.min(self.rows.len().saturating_sub(1))..]
            .iter()
            .find_map(|r| r.new)
            .or_else(|| self.rows.iter().rev().find_map(|r| r.new))
            .unwrap_or(0)
    }

    /// The row that shows a new-text line.
    pub fn row_of(&self, line: usize) -> usize {
        self.rows
            .iter()
            .position(|r| r.new.is_some_and(|n| n >= line))
            .unwrap_or(self.rows.len().saturating_sub(1))
    }

    /// The new-text lines that were added.
    pub fn added_lines(&self) -> impl Iterator<Item = usize> + '_ {
        self.rows
            .iter()
            .filter(|r| r.tag == Tag::Added)
            .filter_map(|r| r.new)
    }

    /// The new-text lines just above a removal, as gitsigns marks them. `None` marks a removal above line 1.
    pub fn removal_marks(&self) -> Vec<Option<usize>> {
        let mut out = Vec::new();
        for (i, r) in self.rows.iter().enumerate() {
            let starts = r.tag == Tag::Removed && (i == 0 || self.rows[i - 1].tag != Tag::Removed);
            if starts {
                out.push(self.rows[..i].iter().rev().find_map(|p| p.new));
            }
        }
        out
    }

    /// The new-text lines that a change touches: added lines, and the line where each removal sits.
    pub fn changed_lines(&self) -> Vec<usize> {
        let mut lines: Vec<usize> = self.added_lines().collect();
        for (i, r) in self.rows.iter().enumerate() {
            if r.tag == Tag::Removed {
                lines.push(self.line_of(i));
            }
        }
        lines.sort_unstable();
        lines.dedup();
        lines
    }
}
