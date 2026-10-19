//! The trail (unit `010`): where the agent read, searched, and edited in this session, by path. Both front ends share it (unit `047`).
//!
//! A path keeps the strongest kind it has seen (edit, then read, then grep), and the sequence number and time of its
//! newest event. A path is fresh while one of its events is among the [`FRESH`] newest events.

use std::collections::HashMap;
use std::time::Duration;

use crate::events::{Event, Kind};

/// A path is fresh while its newest event is among this many newest events.
pub const FRESH: u64 = 3;

/// One path on the trail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mark {
    /// The strongest kind seen.
    pub kind: Kind,
    /// The sequence number of the newest event.
    pub seq: u64,
    /// The app time of the newest event.
    pub at: Duration,
    /// The lines of the newest edit.
    pub lines: Option<(u32, u32)>,
    /// The newest event's sender.
    pub source: Option<String>,
}

/// The trail for the session. It has no size limit.
#[derive(Debug, Clone, Default)]
pub struct Trail {
    marks: HashMap<String, Mark>,
    seq: u64,
}

impl Trail {
    /// Records an event at an app time. A path that the index does not have yet is kept too.
    pub fn record(&mut self, event: &Event, now: Duration) {
        match event {
            Event::Clear => self.clear(),
            // A note is not a read, a search, or an edit (unit `052`).
            Event::Note { .. } | Event::ClearNotes => {}
            Event::Trail {
                kind,
                paths,
                lines,
                source,
                ..
            } => {
                self.seq += 1;
                for p in paths {
                    let m = self.marks.entry(p.clone()).or_insert(Mark {
                        kind: *kind,
                        seq: 0,
                        at: now,
                        lines: None,
                        source: None,
                    });
                    m.kind = m.kind.max(*kind);
                    m.seq = self.seq;
                    m.at = now;
                    m.source = source.clone();
                    if *kind == Kind::Edit {
                        m.lines = *lines;
                    }
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.marks.clear();
    }

    pub fn get(&self, path: &str) -> Option<&Mark> {
        self.marks.get(path)
    }

    /// The mark's newest event is among the [`FRESH`] newest events.
    pub fn is_fresh(&self, m: &Mark) -> bool {
        self.seq - m.seq < FRESH
    }

    pub fn len(&self) -> usize {
        self.marks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.marks.is_empty()
    }
}

/// The sender's label: `CC` for `claude-code`, the source string for another sender, and `agent` without one.
pub fn label(source: Option<&str>) -> &str {
    match source {
        Some("claude-code") => "CC",
        Some(s) if !s.is_empty() => s,
        _ => "agent",
    }
}

/// The selection panel's trail line, for example `CC edited lines 40–58 · 2m ago`.
pub fn describe(m: &Mark, age: Duration) -> String {
    let who = label(m.source.as_deref());
    let what = match (m.kind, m.lines) {
        (Kind::Edit, Some((a, b))) if a == b => format!("edited line {a}"),
        (Kind::Edit, Some((a, b))) => format!("edited lines {a}–{b}"),
        (Kind::Edit, None) => "edited this file".to_string(),
        (Kind::Read, _) => "read this file".to_string(),
        (Kind::Grep, _) => "found this file by search".to_string(),
    };
    format!("{who} {what} · {}", age_phrase(age))
}

/// `Ns ago` and `Nm ago` below one hour, else the panel's age format (`today`, `Nd ago`, …).
pub fn age_phrase(age: Duration) -> String {
    let s = age.as_secs();
    if s < 60 {
        format!("{s}s ago")
    } else if s < 3600 {
        format!("{}m ago", s / 60)
    } else {
        crate::history::ago_phrase((s / 86_400) as i64)
    }
}
