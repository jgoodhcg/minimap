//! The notes (unit `052`): what an agent pointed a person at, in arrival order, for every front end.
//!
//! A note is about one file, or lines of it, with an optional text. An agent sends notes in the order to read them,
//! so the order is the tour: [`Notes::next`] and [`Notes::previous`] step through it.

use crate::events::Event;

/// The notes keep at most this many; the oldest drop.
pub const MAX: usize = 1000;

/// One note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// From 1, in arrival order. An id never repeats in a session, also after a clear.
    pub id: u64,
    /// The repo-relative path.
    pub path: String,
    /// The first and last noted line, 1-based.
    pub lines: Option<(u32, u32)>,
    pub text: String,
    pub source: Option<String>,
    pub session: Option<String>,
}

/// The notes of this session, oldest first.
#[derive(Debug, Clone, Default)]
pub struct Notes {
    list: Vec<Note>,
    last_id: u64,
}

impl Notes {
    /// Records an event: a note is added and its id returned; a notes clear empties the list. Other events change nothing.
    pub fn record(&mut self, event: &Event) -> Option<u64> {
        match event {
            Event::Note {
                path,
                lines,
                text,
                source,
                session,
            } => {
                self.last_id += 1;
                self.list.push(Note {
                    id: self.last_id,
                    path: path.clone(),
                    // In order, also for an event that was not parsed.
                    lines: lines.map(|(a, b)| (a.min(b), a.max(b))),
                    text: text.clone(),
                    source: source.clone(),
                    session: session.clone(),
                });
                if self.list.len() > MAX {
                    let n = self.list.len() - MAX;
                    self.list.drain(..n);
                }
                Some(self.last_id)
            }
            Event::ClearNotes => {
                self.clear();
                None
            }
            Event::Trail { .. } | Event::Clear => None,
        }
    }

    pub fn clear(&mut self) {
        self.list.clear();
    }

    /// Every note, oldest first.
    pub fn all(&self) -> &[Note] {
        &self.list
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    fn index(&self, id: u64) -> Option<usize> {
        // Ids grow with the place in the list, so a binary search finds one.
        self.list.binary_search_by_key(&id, |n| n.id).ok()
    }

    pub fn get(&self, id: u64) -> Option<&Note> {
        self.index(id).map(|i| &self.list[i])
    }

    /// A note's 1-based place in the current list, as `note 3 of 7` shows it.
    pub fn ordinal(&self, id: u64) -> Option<usize> {
        self.index(id).map(|i| i + 1)
    }

    /// The notes about a path, oldest first.
    pub fn of_path<'a>(&'a self, path: &'a str) -> impl Iterator<Item = &'a Note> + 'a {
        self.list.iter().filter(move |n| n.path == path)
    }

    /// The note after `after`: the first note for `None`, or for an id that is no longer in the list. `None` after the last note.
    pub fn next(&self, after: Option<u64>) -> Option<&Note> {
        match after.and_then(|id| self.index(id)) {
            Some(i) => self.list.get(i + 1),
            None => self.list.first(),
        }
    }

    /// The note before `before`: the last note for `None`, or for an id that is no longer in the list. `None` before the first note.
    pub fn previous(&self, before: Option<u64>) -> Option<&Note> {
        match before.and_then(|id| self.index(id)) {
            Some(i) => i.checked_sub(1).map(|j| &self.list[j]),
            None => self.list.last(),
        }
    }
}
