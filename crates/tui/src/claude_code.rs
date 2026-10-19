//! The Claude Code adapter (unit `011`): `minimap hook claude-code` reads one `PostToolUse` payload on stdin
//! and sends trail events (unit `010`) to the minimap of each path's repo.
//!
//! - `Read` → `read`. `Grep` → `grep` with the matched paths from the response.
//! - `Edit` and `MultiEdit` → `edit`, with the edited lines from the structured patch, else from the new text.
//! - `Write` → `edit` without lines. `NotebookEdit` → `edit` with the notebook path.
//! - `Glob` and other tools send nothing: a file-name search lists names only (M3b review).
//!
//! Each event carries the payload's `session_id` (unit `050`).
//!
//! The adapter always exits 0 and prints nothing, so a hook never fails or slows the agent.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use serde_json::Value;

use minimap_core::events::{self, Event, Kind};
use minimap_core::index::repo_root;

/// The event source of this adapter.
pub const SOURCE: &str = "claude-code";

/// Grep response rows read at most, so a huge result cannot slow the hook.
const MAX_ROWS: usize = 5000;

/// What a payload asks for: a trail kind, absolute paths, and an edit's lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mapped {
    pub kind: Kind,
    pub paths: Vec<PathBuf>,
    pub lines: Option<(u32, u32)>,
}

/// One event for one repo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Routed {
    /// The repo's canonical working-tree root.
    pub root: PathBuf,
    pub event: Event,
}

/// Reads a payload on stdin, sends its events, and logs the raw payload to `log` when given.
/// Every error is silent.
pub fn run(log: Option<&Path>) {
    let mut text = String::new();
    if std::io::stdin().read_to_string(&mut text).is_err() {
        return;
    }
    let Ok(payload) = serde_json::from_str::<Value>(&text) else {
        return;
    };
    if let Some(dir) = log {
        let _ = write_log(dir, &payload, &text);
    }
    for r in route(&payload) {
        let _ = events::send(&r.root, &r.event);
    }
}

/// Writes the raw payload to `<dir>/<tool>-<tool_use_id>.json`.
fn write_log(dir: &Path, payload: &Value, text: &str) -> std::io::Result<()> {
    let name = |k: &str| {
        payload
            .get(k)
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>()
    };
    fs::create_dir_all(dir)?;
    fs::write(
        dir.join(format!(
            "{}-{}.json",
            name("tool_name"),
            name("tool_use_id")
        )),
        text,
    )
}

/// The events of a payload, each with the root of the repo that holds its paths.
/// A trail event whose paths lie in two repos splits in two; a path in no repo drops out.
pub fn route(payload: &Value) -> Vec<Routed> {
    let Some(Mapped { kind, paths, lines }) = map(payload) else {
        return Vec::new();
    };
    let session = payload
        .get("session_id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let mut roots: HashMap<PathBuf, Option<PathBuf>> = HashMap::new();
    let mut by_root: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();
    for p in paths {
        let dir = p.parent().unwrap_or(&p).to_path_buf();
        let root = roots
            .entry(dir.clone())
            .or_insert_with(|| repo_root(&nearest_dir(&dir)).ok())
            .clone();
        if let Some(root) = root {
            let list = by_root.entry(root).or_default();
            let s = p.to_string_lossy().into_owned();
            if !list.contains(&s) {
                list.push(s);
            }
        }
    }
    by_root
        .into_iter()
        .map(|(root, paths)| Routed {
            root,
            event: Event::Trail {
                kind,
                paths,
                lines,
                source: Some(SOURCE.to_string()),
                session: session.clone(),
            },
        })
        .collect()
}

/// The nearest directory that exists, so a deleted file's repo is still found.
fn nearest_dir(dir: &Path) -> PathBuf {
    let mut d = dir;
    while !d.is_dir() {
        match d.parent() {
            Some(p) => d = p,
            None => break,
        }
    }
    d.to_path_buf()
}

/// A payload's trail kind, its absolute paths, and an edit's lines. `None` for tools that send nothing.
pub fn map(payload: &Value) -> Option<Mapped> {
    let tool = payload.get("tool_name")?.as_str()?;
    let input = payload.get("tool_input")?;
    let response = payload.get("tool_response");
    let cwd = payload
        .get("cwd")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .unwrap_or_default();
    let abs = |p: &str| {
        let p = Path::new(p);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            cwd.join(p)
        }
    };
    let field = |k: &str| {
        input
            .get(k)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
    };
    let one = |kind, path, lines| {
        Some(Mapped {
            kind,
            paths: vec![path],
            lines,
        })
    };
    match tool {
        "Read" => one(Kind::Read, abs(field("file_path")?), None),
        "Write" => one(Kind::Edit, abs(field("file_path")?), None),
        "NotebookEdit" => one(Kind::Edit, abs(field("notebook_path")?), None),
        "Edit" | "MultiEdit" => {
            let path = abs(field("file_path")?);
            let lines = response
                .and_then(patch_lines)
                .or_else(|| new_text_lines(&path, input));
            one(Kind::Edit, path, lines)
        }
        "Grep" => {
            let base = field("path").map(abs).unwrap_or_else(|| cwd.clone());
            let paths = grep_paths(response?, &base, &cwd);
            (!paths.is_empty()).then_some(Mapped {
                kind: Kind::Grep,
                paths,
                lines: None,
            })
        }
        _ => None,
    }
}

/// The first and last changed lines on the new side of a `structuredPatch`, 1-based.
/// A removal counts at the line where it was removed.
fn patch_lines(response: &Value) -> Option<(u32, u32)> {
    let hunks = response.get("structuredPatch")?.as_array()?;
    let (mut lo, mut hi) = (u32::MAX, 0);
    for h in hunks {
        let mut line = u32::try_from(h.get("newStart")?.as_u64()?).ok()?;
        for row in h.get("lines")?.as_array()? {
            let row = row.as_str()?;
            match row.chars().next() {
                Some('+') => {
                    lo = lo.min(line);
                    hi = hi.max(line);
                    line += 1;
                }
                Some('-') => {
                    lo = lo.min(line);
                    hi = hi.max(line);
                }
                // A context row, or `\ No newline at end of file`.
                Some('\\') => {}
                _ => line += 1,
            }
        }
    }
    (hi > 0).then_some((lo.max(1), hi.max(lo.max(1))))
}

/// The line range of the first new text in the file after the edit, 1-based.
fn new_text_lines(path: &Path, input: &Value) -> Option<(u32, u32)> {
    let new = input
        .get("new_string")
        .and_then(Value::as_str)
        .or_else(|| {
            // MultiEdit: the first edit.
            input
                .get("edits")?
                .as_array()?
                .first()?
                .get("new_string")?
                .as_str()
        })
        .filter(|s| !s.is_empty())?;
    let text = fs::read_to_string(path).ok()?;
    let at = text.find(new)?;
    let start = text[..at].matches('\n').count() as u32 + 1;
    let rows = new.trim_end_matches('\n').matches('\n').count() as u32;
    Some((start, start + rows))
}

/// The matched paths of a Grep response, each once, in response order:
/// `filenames` for `files_with_matches`, and the paths that start `content` rows for `content` and `count`.
fn grep_paths(response: &Value, base: &Path, cwd: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let push = |out: &mut Vec<PathBuf>, p: PathBuf| {
        if !out.contains(&p) {
            out.push(p);
        }
    };
    if let Some(names) = response.get("filenames").and_then(Value::as_array) {
        for n in names.iter().filter_map(Value::as_str) {
            if let Some(p) = resolve(n, base, cwd, false) {
                push(&mut out, p);
            }
        }
    }
    let content = response
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or("");
    let mut seen: HashMap<&str, Option<PathBuf>> = HashMap::new();
    for row in content.lines().take(MAX_ROWS) {
        // A row is `path:line:text`, `path-line-text` for context, or `path:count`. The path is the longest
        // prefix before a `:` or `-` that names a file.
        let cuts = row
            .char_indices()
            .filter(|&(_, c)| c == ':' || c == '-')
            .map(|(i, _)| i);
        for i in cuts {
            let prefix = &row[..i];
            let found = seen
                .entry(prefix)
                .or_insert_with(|| resolve(prefix, base, cwd, true))
                .clone();
            if let Some(p) = found {
                push(&mut out, p);
                break;
            }
        }
    }
    // A search of one file prints no file names: the file is the match.
    if out.is_empty() && !content.trim().is_empty() && base.is_file() {
        push(&mut out, base.to_path_buf());
    }
    out
}

/// A response path as an absolute path. A relative path resolves against the search path, else the session's
/// directory. With `must_exist`, only a path that names a file counts.
fn resolve(p: &str, base: &Path, cwd: &Path, must_exist: bool) -> Option<PathBuf> {
    if p.is_empty() {
        return None;
    }
    let path = Path::new(p);
    if path.is_absolute() {
        return (!must_exist || path.is_file()).then(|| path.to_path_buf());
    }
    let dir = if base.is_file() {
        base.parent().unwrap_or(base)
    } else {
        base
    };
    let candidates = [dir.join(path), cwd.join(path)];
    if let Some(c) = candidates.iter().find(|c| c.is_file()) {
        return Some(c.clone());
    }
    (!must_exist).then(|| candidates[0].clone())
}
