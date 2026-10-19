//! The file viewer (unit `029`): one file, read-only, in place of the map, with vim motions and find,
//! and its uncommitted diff against `HEAD` (unit `030`).
//!
//! The viewer shows rows. In file mode a row is a working-tree line. In diff mode the removed `HEAD` lines are rows too,
//! between the lines where they were removed. Rows, lines, and the cursor are 0-based here;
//! the screen and copied locations are 1-based.

use std::sync::Arc;

use minimap_core::diff::{FileDiff, Tag};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::keymap::{self, Command, Key, Keymap, Lookup};

/// `h` and `l` scroll this many columns.
pub const HSCROLL: usize = 8;
/// A tab moves to the next multiple of this many columns.
pub const TAB_WIDTH: usize = 4;
/// The wheel scrolls this many lines.
pub const WHEEL_LINES: usize = 3;

/// What a key asks the app to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    None,
    Close,
    Help,
    Copy(String),
    Flash(String),
    Editor,
    ToggleTree,
    TogglePanel,
}

const NO_CHANGES: &str = "No uncommitted changes in this file.";

/// The sign column of a row, as gitsigns draws it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    None,
    /// File mode: an added line (`▎`).
    Added,
    /// File mode: a removal below this line (`▁`), or above line 1 (`▔`).
    RemovedBelow,
    RemovedAbove,
    /// Diff mode: `+` and `-`.
    Plus,
    Minus,
}

pub struct Viewer {
    /// The file, as an index into the tree's files.
    pub file: usize,
    pub path: String,
    /// The working-tree lines.
    pub lines: Vec<String>,
    /// The cursor row.
    pub cursor: usize,
    /// The first row in view.
    pub top: usize,
    /// The first column in view.
    pub left: usize,
    /// The rows in view.
    pub height: usize,
    /// The file is too long for syntax colors.
    pub plain: bool,
    /// The find prompt's text while it is open.
    pub prompt: Option<String>,
    /// The last find, for `n`, `N`, and the match marks.
    pub query: Option<String>,
    /// The diff against `HEAD`, once the background thread sends it.
    pub diff: Option<Arc<FileDiff>>,
    /// The `HEAD` lines, for removed rows.
    pub old: Arc<Vec<String>>,
    /// Diff mode (`=`): removed rows show between the lines.
    pub diff_mode: bool,
    /// File-mode signs per working-tree line.
    signs: Vec<Sign>,
    /// The keymap; the viewer uses its `viewer` table (unit `033`).
    pub keys: Arc<Keymap>,
    count: Option<usize>,
    /// The keys of a sequence typed so far, such as the `g` of `gg`.
    pending: Vec<Key>,
}

impl Viewer {
    pub fn new(file: usize, path: &str, text: &str) -> Viewer {
        let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        Viewer {
            file,
            path: path.to_string(),
            lines,
            cursor: 0,
            top: 0,
            left: 0,
            height: 1,
            plain: false,
            prompt: None,
            query: None,
            diff: None,
            old: Arc::new(Vec::new()),
            diff_mode: false,
            signs: Vec::new(),
            keys: Keymap::shared_default(),
            count: None,
            pending: Vec::new(),
        }
    }

    /// Replaces the text after the file changed on disk (unit `034`). The cursor stays on its line number,
    /// clamped to the new length, and the diff waits for the background thread again.
    pub fn reload(&mut self, text: &str) {
        let line = self.cursor_line();
        self.lines = text.lines().map(str::to_string).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.diff = None;
        self.old = Arc::new(Vec::new());
        self.signs = Vec::new();
        self.go(line.min(self.lines.len() - 1));
    }

    /// True when the viewer shows this text.
    pub fn shows(&self, text: &str) -> bool {
        let mut lines = text.lines();
        let same = self.lines.iter().all(|l| lines.next() == Some(l.as_str()));
        // An empty file shows one empty line.
        same && lines.next().is_none() || text.is_empty() && self.lines == [""]
    }

    /// Sets the diff against `HEAD` and the `HEAD` lines. In diff mode the cursor stays on its working-tree line.
    pub fn set_diff(&mut self, diff: Arc<FileDiff>, old: Arc<Vec<String>>) {
        let line = self.cursor_line();
        let mut signs = vec![Sign::None; self.lines.len()];
        for m in diff.removal_marks() {
            match m {
                Some(l) if l < signs.len() => signs[l] = Sign::RemovedBelow,
                None if !signs.is_empty() => signs[0] = Sign::RemovedAbove,
                _ => {}
            }
        }
        for l in diff.added_lines() {
            if l < signs.len() {
                signs[l] = Sign::Added;
            }
        }
        self.signs = signs;
        self.diff = Some(diff);
        self.old = old;
        if self.diff_mode {
            self.go_line(line);
        }
    }

    /// A row's sign: in file mode the gitsigns marks, in diff mode `+` or `-`.
    pub fn sign(&self, row: usize) -> Sign {
        match self.row_tag(row) {
            Tag::Added if self.diff_mode => Sign::Plus,
            Tag::Removed => Sign::Minus,
            _ if self.diff_mode => Sign::None,
            _ => self.signs.get(row).copied().unwrap_or(Sign::None),
        }
    }

    /// The diff while diff mode shows it.
    fn shown_diff(&self) -> Option<&FileDiff> {
        self.diff.as_deref().filter(|_| self.diff_mode)
    }

    /// The row count.
    pub fn len(&self) -> usize {
        match self.shown_diff() {
            Some(d) if !d.rows.is_empty() => d.rows.len(),
            _ => self.lines.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The last row.
    fn last(&self) -> usize {
        self.len() - 1
    }

    /// A row's text.
    pub fn row_text(&self, row: usize) -> &str {
        match self.shown_diff().and_then(|d| d.rows.get(row)) {
            Some(r) => match (r.new, r.old) {
                (Some(n), _) => self.lines.get(n).map_or("", String::as_str),
                (None, Some(o)) => self.old.get(o).map_or("", String::as_str),
                (None, None) => "",
            },
            None => self.lines.get(row).map_or("", String::as_str),
        }
    }

    /// A row's kind: same, added, or removed. In file mode, a line is added when the diff says so.
    pub fn row_tag(&self, row: usize) -> Tag {
        match self.shown_diff() {
            Some(d) => d.rows.get(row).map_or(Tag::Same, |r| r.tag),
            None if self.signs.get(row) == Some(&Sign::Added) => Tag::Added,
            None => Tag::Same,
        }
    }

    /// A row's working-tree line. `None` for a removed row.
    pub fn row_line(&self, row: usize) -> Option<usize> {
        match self.shown_diff() {
            Some(d) => d.rows.get(row).and_then(|r| r.new),
            None => Some(row),
        }
    }

    /// A removed row's `HEAD` line.
    pub fn row_old(&self, row: usize) -> Option<usize> {
        self.shown_diff()
            .and_then(|d| d.rows.get(row))
            .filter(|r| r.tag == Tag::Removed)
            .and_then(|r| r.old)
    }

    /// The working-tree line of a row: its own, or for a removed row the next line.
    pub fn line_of_row(&self, row: usize) -> usize {
        match self.shown_diff() {
            Some(d) if !d.rows.is_empty() => d.line_of(row),
            _ => row,
        }
    }

    /// The row that shows a working-tree line.
    pub fn row_of_line(&self, line: usize) -> usize {
        match self.shown_diff() {
            Some(d) if !d.rows.is_empty() => d.row_of(line),
            _ => line.min(self.lines.len() - 1),
        }
    }

    /// The working-tree line of the cursor.
    pub fn cursor_line(&self) -> usize {
        self.line_of_row(self.cursor)
    }

    /// Sets the rows in view and keeps the cursor in them.
    pub fn set_height(&mut self, h: usize) {
        self.height = h.max(1);
        self.keep_visible();
    }

    /// Moves the cursor to a row, clamped, and scrolls only as far as needed.
    pub fn go(&mut self, row: usize) {
        self.cursor = row.min(self.last());
        self.keep_visible();
    }

    /// Moves the cursor to the row of a working-tree line.
    pub fn go_line(&mut self, line: usize) {
        self.go(self.row_of_line(line));
    }

    /// Puts the cursor row at the top of the view, as `zt` does.
    pub fn to_top(&mut self) {
        self.top = self.cursor;
    }

    fn keep_visible(&mut self) {
        self.cursor = self.cursor.min(self.last());
        self.top = self.top.min(self.last());
        if self.cursor < self.top {
            self.top = self.cursor;
        } else if self.cursor >= self.top + self.height {
            self.top = self.cursor + 1 - self.height;
        }
    }

    /// The highest top that still fills the view.
    fn full_top(&self) -> usize {
        self.len().saturating_sub(self.height)
    }

    /// Scrolls the view and the cursor together by `n` rows, as Ctrl+D and Ctrl+F do.
    fn scroll_both(&mut self, n: usize, down: bool) {
        if down {
            self.top = (self.top + n).min(self.full_top().max(self.top));
            self.cursor = (self.cursor + n).min(self.last());
        } else {
            self.top = self.top.saturating_sub(n);
            self.cursor = self.cursor.saturating_sub(n);
        }
        self.keep_visible();
    }

    /// Scrolls the view by `n` rows and keeps the cursor in it, as the wheel does.
    pub fn scroll(&mut self, n: usize, down: bool) {
        if down {
            self.top = (self.top + n).min(self.full_top().max(self.top));
        } else {
            self.top = self.top.saturating_sub(n);
        }
        self.cursor = self
            .cursor
            .clamp(self.top, (self.top + self.height - 1).min(self.last()));
    }

    /// The display width of the widest row, with tabs expanded.
    pub fn width(&self) -> usize {
        (0..self.len())
            .map(|r| display_width(self.row_text(r)))
            .max()
            .unwrap_or(0)
    }

    /// The matches of the last find in a row, as char ranges.
    pub fn matches_in(&self, row: usize) -> Vec<(usize, usize)> {
        match &self.query {
            Some(q) => find_in(self.row_text(row), q),
            None => vec![],
        }
    }

    /// The count of matches of a query in the rows.
    pub fn match_count(&self, q: &str) -> usize {
        (0..self.len())
            .map(|r| find_in(self.row_text(r), q).len())
            .sum()
    }

    /// The next row with a match, after the cursor (or at it, with `here`), wrapping. Backward with `!forward`.
    fn next_match(&self, forward: bool, here: bool) -> Option<usize> {
        let q = self.query.as_ref()?;
        let n = self.len();
        let start = if here { 0 } else { 1 };
        (start..=n).find_map(|k| {
            let row = if forward {
                (self.cursor + k) % n
            } else {
                (self.cursor + n * 2 - k) % n
            };
            (!find_in(self.row_text(row), q).is_empty()).then_some(row)
        })
    }

    fn jump_match(&mut self, forward: bool, here: bool) -> Action {
        let Some(q) = self.query.clone() else {
            return Action::Flash("No search yet. Press / to find.".to_string());
        };
        match self.next_match(forward, here) {
            Some(row) => {
                self.go(row);
                Action::None
            }
            None => Action::Flash(format!("Not found: {q}")),
        }
    }

    /// The next (or previous) outline line from the cursor's line. `outline` holds 1-based lines.
    fn outline_jump(&self, outline: &[u32], forward: bool, n: usize) -> Option<usize> {
        let mut at = self.cursor_line();
        for _ in 0..n {
            let lines = outline.iter().map(|&l| l.max(1) as usize - 1);
            let next = if forward {
                lines.filter(|&l| l > at).min()
            } else {
                lines.filter(|&l| l < at).max()
            };
            at = next?;
        }
        Some(at)
    }

    /// The next (or previous) hunk from the cursor, as a row: in file mode the hunk's working-tree line.
    fn hunk_jump(&self, forward: bool, n: usize) -> Result<Option<usize>, &'static str> {
        let Some(d) = self.diff.as_deref() else {
            return Err("The diff is still loading.");
        };
        if d.is_empty() {
            return Err(NO_CHANGES);
        }
        let starts: Vec<usize> = d
            .hunks
            .iter()
            .map(|&(s, _)| if self.diff_mode { s } else { d.line_of(s) })
            .collect();
        let mut at = self.cursor;
        for _ in 0..n {
            let next = if forward {
                starts.iter().copied().filter(|&s| s > at).min()
            } else {
                starts.iter().copied().filter(|&s| s < at).max()
            };
            match next {
                Some(s) => at = s,
                None => break,
            }
        }
        Ok((at != self.cursor).then_some(at))
    }

    /// `=`: switches between file mode and diff mode. The cursor stays on its working-tree line and its screen row.
    fn toggle_diff(&mut self) -> Action {
        match self.diff.as_deref() {
            None => return Action::Flash("The diff is still loading.".to_string()),
            Some(d) if d.is_empty() => return Action::Flash(NO_CHANGES.to_string()),
            Some(_) => {}
        }
        let line = self.cursor_line();
        let offset = self.cursor.saturating_sub(self.top);
        self.diff_mode = !self.diff_mode;
        self.cursor = self.row_of_line(line);
        self.top = self.cursor.saturating_sub(offset);
        self.keep_visible();
        Action::None
    }

    /// The next (or previous) blank row from the cursor, else the last (or first) row, as `}` and `{` do.
    fn paragraph(&self, forward: bool) -> usize {
        let blank = |r: usize| self.row_text(r).trim().is_empty();
        if forward {
            (self.cursor + 1..self.len())
                .find(|&r| blank(r))
                .unwrap_or(self.last())
        } else {
            (0..self.cursor).rev().find(|&r| blank(r)).unwrap_or(0)
        }
    }

    /// Handles a key through the keymap's `viewer` table. `outline` holds the file's outline lines, 1-based.
    pub fn key(&mut self, key: KeyEvent, outline: &[u32]) -> Action {
        if self.prompt.is_some() {
            return self.prompt_key(key);
        }
        let k = Key::from_event(&key);
        if self.pending.is_empty() {
            if keymap::is_count_digit(k) {
                let KeyCode::Char(c) = k.code else {
                    unreachable!("a count digit is a character")
                };
                self.count = Some(self.count.unwrap_or(0) * 10 + (c as usize - '0' as usize));
                return Action::None;
            }
            if k.code == KeyCode::Char('0') && k.mods.is_empty() && self.count.is_some() {
                self.count = self.count.map(|n| n * 10);
                return Action::None;
            }
        }
        self.pending.push(k);
        match self.keys.viewer.lookup(&self.pending) {
            Lookup::Wait => Action::None,
            Lookup::None => {
                self.pending.clear();
                self.count = None;
                Action::None
            }
            Lookup::Run(cmd) => {
                self.pending.clear();
                let count = self.count.take();
                self.run(cmd, count, outline)
            }
        }
    }

    /// Runs a viewer action. `count` repeats the motions that take one.
    fn run(&mut self, cmd: Command, count: Option<usize>, outline: &[u32]) -> Action {
        let n = count.unwrap_or(1);
        let half = (self.height / 2).max(1);
        match cmd {
            Command::Down => self.go(self.cursor + n),
            Command::Up => self.go(self.cursor.saturating_sub(n)),
            Command::HalfPageDown => self.scroll_both(half, true),
            Command::HalfPageUp => self.scroll_both(half, false),
            Command::PageDown => self.scroll_both(self.height, true),
            Command::PageUp => self.scroll_both(self.height, false),
            Command::FirstLine => match count {
                Some(c) => self.go_line(c.max(1) - 1),
                None => self.go(0),
            },
            Command::LastLine => match count {
                Some(c) => self.go_line(c.max(1) - 1),
                None => self.go(self.last()),
            },
            Command::CenterCursor => self.top = self.cursor.saturating_sub(self.height / 2),
            Command::CursorToTop => self.to_top(),
            Command::CursorToBottom => self.top = (self.cursor + 1).saturating_sub(self.height),
            Command::NextDefinition | Command::PreviousDefinition => {
                let next = cmd == Command::NextDefinition;
                match self.outline_jump(outline, next, n) {
                    Some(line) => self.go_line(line),
                    None if outline.is_empty() => {
                        return Action::Flash("This file has no outline.".to_string());
                    }
                    None => {}
                }
            }
            Command::NextChange | Command::PreviousChange => {
                match self.hunk_jump(cmd == Command::NextChange, n) {
                    Ok(Some(row)) => self.go(row),
                    Ok(None) => {}
                    Err(msg) => return Action::Flash(msg.to_string()),
                }
            }
            Command::NextBlank | Command::PreviousBlank => {
                for _ in 0..n {
                    self.go(self.paragraph(cmd == Command::NextBlank));
                }
            }
            Command::ScrollLeft => self.left = self.left.saturating_sub(HSCROLL * n),
            Command::ScrollRight => {
                let max = self.width().saturating_sub(1);
                self.left = (self.left + HSCROLL * n).min(max.max(self.left));
            }
            Command::ScrollHome => self.left = 0,
            Command::Find => self.prompt = Some(String::new()),
            Command::NextMatch => return self.jump_match(true, false),
            Command::PreviousMatch => return self.jump_match(false, false),
            Command::ToggleDiff => return self.toggle_diff(),
            Command::CopyLocation => {
                return Action::Copy(format!("{}:{}", self.path, self.cursor_line() + 1));
            }
            Command::OpenEditor => return Action::Editor,
            Command::ToggleTree => return Action::ToggleTree,
            Command::TogglePanel => return Action::TogglePanel,
            Command::Help => return Action::Help,
            Command::Close => return Action::Close,
            _ => {}
        }
        Action::None
    }

    /// Keys in the find prompt: text edits the query, Enter finds, Esc cancels.
    fn prompt_key(&mut self, key: KeyEvent) -> Action {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let Some(p) = &mut self.prompt else {
            return Action::None;
        };
        match key.code {
            KeyCode::Esc => self.prompt = None,
            KeyCode::Enter => {
                let q = std::mem::take(p);
                self.prompt = None;
                if !q.is_empty() {
                    self.query = Some(q);
                }
                return self.jump_match(true, true);
            }
            KeyCode::Backspace => {
                p.pop();
            }
            KeyCode::Char('u') if ctrl => p.clear(),
            KeyCode::Char(c) if !ctrl => p.push(c),
            _ => {}
        }
        Action::None
    }
}

/// Smart case: a query with an uppercase letter matches case, else it ignores case.
fn same(a: char, b: char, ignore_case: bool) -> bool {
    if ignore_case {
        a.to_lowercase().eq(b.to_lowercase())
    } else {
        a == b
    }
}

/// The literal matches of `q` in a line, as char ranges, left to right, not overlapping.
pub fn find_in(line: &str, q: &str) -> Vec<(usize, usize)> {
    let hay: Vec<char> = line.chars().collect();
    let needle: Vec<char> = q.chars().collect();
    let ignore_case = !q.chars().any(char::is_uppercase);
    let mut out = Vec::new();
    if needle.is_empty() || needle.len() > hay.len() {
        return out;
    }
    let mut i = 0;
    while i + needle.len() <= hay.len() {
        if (0..needle.len()).all(|j| same(hay[i + j], needle[j], ignore_case)) {
            out.push((i, i + needle.len()));
            i += needle.len();
        } else {
            i += 1;
        }
    }
    out
}

/// The display width of a line, with tabs to the next multiple of [`TAB_WIDTH`].
pub fn display_width(line: &str) -> usize {
    let mut col = 0;
    for ch in line.chars() {
        col += char_width(ch, col);
    }
    col
}

/// The cells a char takes at a column: a tab runs to the next tab stop; other chars take their display width.
pub fn char_width(ch: char, col: usize) -> usize {
    if ch == '\t' {
        TAB_WIDTH - col % TAB_WIDTH
    } else {
        unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0)
    }
}
