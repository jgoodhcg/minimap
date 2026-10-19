//! The file picker (unit 026): a Telescope-style search, ranked by `minimap_core::fuzzy`.
//!
//! The results are best first. The list draws them bottom-up, so the best one sits next to the prompt.

use std::collections::HashSet;

use minimap_core::fuzzy::Ranked;

use crate::filetree::Entry;

/// The picker, from `/` until Esc.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Search {
    pub query: String,
    /// The ranked results, best first. With an empty query, every visible file in tree order.
    pub results: Vec<Ranked>,
    /// The highlighted result, into `results`. `None` until the query or a move picks one.
    pub cursor: Option<usize>,
    /// The result drawn on the list's bottom row.
    pub scroll: usize,
    /// The number of rows the list shows.
    pub rows: usize,
    /// The number of visible files, the `M` of `N / M`.
    pub total: usize,
    /// The picker is open. After Enter it closes, and `kept` holds the marked results.
    pub active: bool,
    /// After Enter: the files of the rows that were visible, in rank order, for `n` and `N`.
    pub kept: Vec<usize>,
    /// The kept file that `n` or `N` last selected, into `kept`.
    pub k: usize,
    /// The results count when Enter kept them.
    pub kept_of: usize,
    /// The selection and camera before `/`, which Esc restores.
    pub origin: Option<(Option<Entry>, i32, i32)>,
    /// The marked files: the visible rows while open, `kept` after Enter. Empty for an empty query.
    marks: HashSet<usize>,
}

impl Search {
    /// A picker that opens with every visible file, in tree order, and nothing highlighted.
    pub fn open(results: Vec<Ranked>, origin: (Option<Entry>, i32, i32)) -> Search {
        Search {
            total: results.len(),
            results,
            active: true,
            origin: Some(origin),
            ..Search::default()
        }
    }

    /// True for a file marked on the map.
    pub fn is_marked(&self, f: usize) -> bool {
        self.marks.contains(&f)
    }

    /// The results in the list's visible rows, best first.
    pub fn visible(&self) -> &[Ranked] {
        let end = (self.scroll + self.rows).min(self.results.len());
        &self.results[self.scroll.min(end)..end]
    }

    /// The highlighted file.
    pub fn current(&self) -> Option<usize> {
        self.cursor
            .and_then(|k| self.results.get(k))
            .map(|r| r.index)
    }

    /// Replaces the results after a query change. The best result is highlighted unless the query is empty.
    pub fn set_results(&mut self, results: Vec<Ranked>) {
        self.cursor = (!self.query.trim().is_empty() && !results.is_empty()).then_some(0);
        self.results = results;
        self.scroll = 0;
        self.update_marks();
    }

    /// Moves the highlight by `delta` results (positive: worse, up the list), and scrolls to keep it in view.
    pub fn move_by(&mut self, delta: isize) {
        if self.results.is_empty() {
            return;
        }
        let last = self.results.len() as isize - 1;
        let k = match self.cursor {
            Some(k) => (k as isize + delta).clamp(0, last),
            None => 0,
        } as usize;
        self.cursor = Some(k);
        self.keep_visible();
    }

    /// Highlights the result on a list row, counted from the bottom.
    pub fn pick_row(&mut self, from_bottom: usize) {
        let k = self.scroll + from_bottom;
        if k < self.results.len() {
            self.cursor = Some(k);
            self.update_marks();
        }
    }

    /// Sets the list height and keeps the highlight in view.
    pub fn set_rows(&mut self, rows: usize) {
        self.rows = rows.max(1);
        self.keep_visible();
    }

    fn keep_visible(&mut self) {
        if let Some(k) = self.cursor {
            if k >= self.scroll + self.rows {
                self.scroll = k + 1 - self.rows;
            } else if k < self.scroll {
                self.scroll = k;
            }
        }
        self.update_marks();
    }

    /// Closes the picker and keeps the visible rows for `n` and `N`.
    pub fn keep(&mut self) {
        self.active = false;
        self.kept = self.visible().iter().map(|r| r.index).collect();
        self.kept_of = self.results.len();
        self.k = self
            .current()
            .and_then(|f| self.kept.iter().position(|&x| x == f))
            .unwrap_or(0);
        self.update_marks();
    }

    /// `n` / `N`: the next or previous kept file, wrapping.
    pub fn next_kept(&mut self, delta: isize) -> Option<usize> {
        if self.kept.is_empty() {
            return None;
        }
        let n = self.kept.len() as isize;
        self.k = (self.k as isize + delta).rem_euclid(n) as usize;
        Some(self.kept[self.k])
    }

    /// Moves the picker to a new index (unit `034`). `file` maps an old file index to its new one, and `entry` an old
    /// selection. While the picker is open, `results` are the new ranking and the highlight stays on its file.
    pub fn remap(
        &mut self,
        file: impl Fn(usize) -> Option<usize>,
        entry: impl Fn(Entry) -> Option<Entry>,
        results: Vec<Ranked>,
        total: usize,
    ) {
        if let Some((e, x, y)) = self.origin {
            self.origin = Some((e.and_then(&entry), x, y));
        }
        let kept: Vec<usize> = self.kept.iter().filter_map(|&f| file(f)).collect();
        let current = self.kept.get(self.k).and_then(|&f| file(f));
        self.k = current
            .and_then(|f| kept.iter().position(|&x| x == f))
            .unwrap_or(0);
        self.kept = kept;
        if self.active {
            let highlighted = self.current().and_then(&file);
            self.total = total;
            self.results = results;
            self.cursor = highlighted.and_then(|f| self.results.iter().position(|r| r.index == f));
            self.scroll = self.scroll.min(self.results.len().saturating_sub(1));
            self.keep_visible();
        }
        self.update_marks();
    }

    /// Drops kept files that are no longer visible, for the hidden toggle.
    pub fn retain_kept(&mut self, visible: impl Fn(usize) -> bool) {
        self.kept.retain(|&f| visible(f));
        self.k = self.k.min(self.kept.len().saturating_sub(1));
        self.update_marks();
    }

    fn update_marks(&mut self) {
        self.marks = if self.active {
            if self.query.trim().is_empty() {
                HashSet::new()
            } else {
                self.visible().iter().map(|r| r.index).collect()
            }
        } else {
            self.kept.iter().copied().collect()
        };
    }
}
