//! The note reader (unit `054`): one note's file text in the map region, in place of the map, with the note on its lines.
//!
//! `lines` mode shows the noted lines with [`CONTEXT`] lines around them. `whole file` mode shows every line,
//! with each of the file's notes in a box above its first line. [`build`] turns the facts into draw items and the clickable mode chips,
//! and [`paint`] draws them, so tests read the reader with no window, as `panel` and `activity` do. Input goes through `GuiApp::on_event`.
//! A [`Colorer`] thread colors the file with the core's highlighter (unit `055`); until it answers, the text is plain.
//! A drag selects text by row and column, and [`selected_text`] gives the copy (unit `057`).

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use eframe::egui::{Painter, Pos2, Rect, pos2, vec2};
use minimap_core::content::Content;
use minimap_core::highlight::{Highlighter, Span};
use minimap_core::notes::{Note, Notes};
use minimap_core::palette;

use crate::activity::fit_left;
use crate::color::{mix_hex, rgb};
use crate::panel::{Draw, paint_draws};
use crate::text::fit;
use crate::tree::{ROW_H, TEXT};

/// A row of the reader, in points: a code line, or a line of a note box.
pub const CODE_ROW: f32 = 18.0;
/// `lines` mode shows this many lines above and below the noted lines, in `DIM`.
pub const CONTEXT: usize = 2;
/// The line-number gutter, in cells: 5 digits and a space.
pub const GUTTER: usize = 6;
/// A tab runs to the next multiple of this many columns.
pub const TAB_WIDTH: usize = 4;
/// The note box's fill and the noted lines' fill: `FOCUS` at these shares over the pane.
pub const BOX_MIX: f32 = 0.14;
pub const LINES_MIX: f32 = 0.05;
/// A file larger than this shows a message, as the TUI's viewer shows it plain.
pub const MAX_BYTES: u64 = minimap_core::syntax::MAX_BYTES as u64;
/// The mode chips in the head row.
pub const LINES_CHIP: &str = "lines";
pub const WHOLE_CHIP: &str = "whole file";
/// The text without a text source.
pub const NO_SOURCE: &str = "No file text: the window shows an index JSON.";

/// The reader's parts on the screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReaderView {
    /// The map region.
    pub rect: Rect,
    /// The first row: the note's number, its path and lines, and the mode chips.
    pub head: Rect,
    /// The rows below the head row and its rule.
    pub body: Rect,
}

impl ReaderView {
    pub fn new(rect: Rect) -> ReaderView {
        let head = Rect::from_min_size(rect.min, vec2(rect.width(), ROW_H.min(rect.height())));
        let body = Rect::from_min_max(
            pos2(rect.left(), (head.bottom() + 1.0).min(rect.bottom())),
            rect.max,
        );
        ReaderView { rect, head, body }
    }

    /// The whole rows that fit the body.
    pub fn visible_rows(&self) -> usize {
        (self.body.height() / CODE_ROW + 1e-3).floor().max(0.0) as usize
    }
}

/// The file text that the reader shows: its lines, or a message in their place.
#[derive(Debug, Clone, PartialEq)]
pub enum Text {
    Lines(Arc<Vec<String>>),
    Message(String),
}

impl Text {
    /// A file's text from a text source. A file over [`MAX_BYTES`], a binary file, and a read error give a message.
    pub fn load(content: &Content, path: &str) -> Text {
        if content.is_none() {
            return Text::Message(NO_SOURCE.to_string());
        }
        if let Some(root) = content.root()
            && let Ok(m) = std::fs::metadata(root.join(path))
            && m.len() > MAX_BYTES
        {
            return Text::Message(too_large(path, m.len()));
        }
        match content.text(path) {
            Ok(Some(t)) => Text::of(path, &t),
            Ok(None) => Text::Message(format!("{path} is not in the working tree.")),
            Err(e) => {
                let e = format!("{e:#}");
                let mut c = e.chars();
                let first = c.next().map(|f| f.to_uppercase().collect::<String>());
                Text::Message(format!("{}{}.", first.unwrap_or_default(), c.as_str()))
            }
        }
    }

    /// A text as lines, or a message for a binary or too large text.
    pub fn of(path: &str, t: &str) -> Text {
        if t.len() as u64 > MAX_BYTES {
            return Text::Message(too_large(path, t.len() as u64));
        }
        if t.contains('\0') {
            return Text::Message(format!("{path} is a binary file."));
        }
        Text::Lines(Arc::new(t.lines().map(str::to_string).collect()))
    }
}

fn too_large(path: &str, bytes: u64) -> String {
    format!(
        "{path} is too large for the reader ({:.1} MiB).",
        bytes as f64 / (1024.0 * 1024.0)
    )
}

/// One row of the reader's page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    /// A file line, 0-based. A context line draws in `DIM`.
    Line { n: usize, context: bool },
    /// A note box's label row: `note 3 of 7` and its lines.
    Label { note: u64 },
    /// A line of a note box's wrapped text.
    Note { note: u64, k: usize },
    /// A message in place of the text.
    Message,
}

/// What the reader shows.
pub struct Facts<'a> {
    pub notes: &'a Notes,
    /// The note that the reader shows, by its id.
    pub note: u64,
    /// `whole file` mode; else `lines` mode.
    pub whole: bool,
    pub text: &'a Text,
    /// The first row to show, before the limits.
    pub top: usize,
    /// A monospace glyph's advance as a share of the text size.
    pub char_w: f32,
    /// The chip under the pointer.
    pub hovered: Option<ReaderHit>,
    /// The text's syntax colors, one span list per line, once the [`Colorer`] has them (unit `055`).
    pub colors: Option<&'a [Vec<Span>]>,
    /// The text selection (unit `057`).
    pub selection: Option<Selection>,
}

/// A selection end (unit `057`): a row of the page's rows, and a column in the row's text, in cells from the text's left edge.
/// Ends compare by row, then by column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Spot {
    pub row: usize,
    pub col: usize,
}

/// The reader's text selection (unit `057`): from the press point to the pointer, by row and column, as in a terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: Spot,
    pub head: Spot,
}

impl Selection {
    /// The two ends, the first one first.
    pub fn ends(&self) -> (Spot, Spot) {
        (self.anchor.min(self.head), self.anchor.max(self.head))
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }
}

/// The selection's fill: `CURSOR` at this share over the row, below the text and above the noted-lines tint (unit `057`).
pub const SELECT_MIX: f32 = 0.25;

/// A clickable part of the reader: a mode chip, `true` for `whole file`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReaderHit {
    Mode(bool),
}

/// The reader's draw items, its chips, and its rows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Page {
    pub draws: Vec<Draw>,
    pub hits: Vec<(Rect, ReaderHit)>,
    pub rows: Vec<Row>,
    /// The first row shown, within the limits, and the largest first row.
    pub top: usize,
    pub max_top: usize,
    /// The row of the current note's label.
    pub current: usize,
    /// Each shown note's frame on the screen, before the body clips it, by note id.
    pub frames: Vec<(u64, Rect)>,
}

impl Page {
    /// The chip under a point.
    pub fn hit(&self, p: Pos2) -> Option<ReaderHit> {
        self.hits
            .iter()
            .find(|(r, _)| r.contains(p))
            .map(|&(_, h)| h)
    }

    pub fn rect_of(&self, h: ReaderHit) -> Option<Rect> {
        self.hits.iter().find(|(_, k)| *k == h).map(|&(r, _)| r)
    }

    /// The texts from the top, in paint order.
    pub fn texts(&self) -> Vec<&str> {
        self.draws
            .iter()
            .filter_map(|d| match d {
                Draw::Text { text, .. } => Some(text.as_str()),
                Draw::Fill { .. } => None,
            })
            .collect()
    }
}

/// A note's lines as 0-based lines of a text of `len` lines, clamped to it. `None` for a note without lines, or an empty text.
pub fn span(lines: Option<(u32, u32)>, len: usize) -> Option<(usize, usize)> {
    let (a, b) = lines?;
    if len == 0 {
        return None;
    }
    let a = (a.max(1) as usize - 1).min(len - 1);
    let b = (b.max(1) as usize - 1).min(len - 1).max(a);
    Some((a, b))
}

/// A note's text as box lines of at most `w` characters: each text line wrapped, blank lines kept, trailing blank lines dropped.
pub fn note_lines(text: &str, w: usize) -> Vec<String> {
    box_rows(text, w).into_iter().map(|r| r.text).collect()
}

/// A row of a note box, with its place in the note's text, so a copy gives the note's text lines (unit `057`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxRow {
    pub text: String,
    /// The text line that the row comes from, among `text.lines()`.
    pub line: usize,
    /// Each char's offset in that line, in chars. A space that joins two words is the first blank after the word before it.
    pub at: Vec<usize>,
    /// The row starts its text line; the row ends its text line.
    pub first: bool,
    pub last: bool,
}

/// The rows of [`note_lines`] with their places in the note's text. Each line wraps as `minimap_core::text::wrap` wraps it.
pub fn box_rows(text: &str, w: usize) -> Vec<BoxRow> {
    let mut out: Vec<BoxRow> = Vec::new();
    for (k, line) in text.lines().enumerate() {
        let rows = if line.trim().is_empty() {
            vec![Vec::new()]
        } else {
            wrap_at(line, w)
        };
        let n = rows.len();
        for (j, r) in rows.into_iter().enumerate() {
            out.push(BoxRow {
                text: r.iter().map(|&(c, _)| c).collect(),
                line: k,
                at: r.iter().map(|&(_, i)| i).collect(),
                first: j == 0,
                last: j + 1 == n,
            });
        }
    }
    while out.last().is_some_and(|r| r.text.is_empty()) {
        out.pop();
    }
    out
}

/// `minimap_core::text::wrap` with each char's offset in the line: words joined by one space, a word longer than `w` broken across rows.
fn wrap_at(line: &str, w: usize) -> Vec<Vec<(char, usize)>> {
    let mut out = Vec::new();
    if w == 0 {
        return out;
    }
    let mut words: Vec<Vec<(char, usize)>> = Vec::new();
    let mut word = Vec::new();
    for (i, c) in line.chars().enumerate() {
        if c.is_whitespace() {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
        } else {
            word.push((c, i));
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    let mut row: Vec<(char, usize)> = Vec::new();
    for word in words {
        let (n, k) = (row.len(), word.len());
        if n > 0 && n + 1 + k > w {
            out.push(std::mem::take(&mut row));
        } else if let Some(&(_, i)) = row.last() {
            row.push((' ', i + 1));
        }
        row.extend(word);
        while row.len() > w {
            let rest = row.split_off(w);
            out.push(std::mem::replace(&mut row, rest));
        }
    }
    if !row.is_empty() {
        out.push(row);
    }
    out
}

/// A line with its tabs run to the next multiple of [`TAB_WIDTH`] columns, cut to `cols` characters.
pub fn expand(line: &str, cols: usize) -> String {
    expand_from(line, 0, cols).0
}

/// A line's width in columns, with its tabs run to their stops.
pub fn width(line: &str) -> usize {
    expand_from(line, 0, usize::MAX).1
}

/// The chars of a line whose first column is in `from..to`, with the tabs kept (unit `057`).
pub fn cut_cols(line: &str, from: usize, to: usize) -> String {
    let mut out = String::new();
    let mut n = 0;
    for c in line.chars() {
        if (from..to).contains(&n) {
            out.push(c);
        }
        n = if c == '\t' {
            (n / TAB_WIDTH + 1) * TAB_WIDTH
        } else {
            n + 1
        };
    }
    out
}

/// [`expand`] for a part of a line that starts at column `from`. Returns the text and the column after it.
pub fn expand_from(line: &str, from: usize, cols: usize) -> (String, usize) {
    let mut out = String::new();
    let mut n = from;
    for c in line.chars() {
        if n >= cols {
            break;
        }
        if c == '\t' {
            let to = (n / TAB_WIDTH + 1) * TAB_WIDTH;
            while n < to.min(cols) {
                out.push(' ');
                n += 1;
            }
        } else {
            out.push(c);
            n += 1;
        }
    }
    (out, n)
}

/// `40–58`, or `12` for one line.
pub fn lines_text(lines: Option<(u32, u32)>) -> Option<String> {
    lines.map(|(a, b)| {
        if a == b {
            a.to_string()
        } else {
            format!("{a}–{b}")
        }
    })
}

/// The rows of the page and the row of the current note's label, for box text `wrap_w` characters wide.
pub fn layout(facts: &Facts, wrap_w: usize) -> (Vec<Row>, usize) {
    let Some(cur) = facts.notes.get(facts.note) else {
        return (Vec::new(), 0);
    };
    let mut rows = Vec::new();
    let push_box = |rows: &mut Vec<Row>, n: &Note| {
        rows.push(Row::Label { note: n.id });
        for k in 0..note_lines(&n.text, wrap_w).len() {
            rows.push(Row::Note { note: n.id, k });
        }
    };
    let len = match facts.text {
        Text::Lines(l) if !l.is_empty() => l.len(),
        _ => {
            push_box(&mut rows, cur);
            rows.push(Row::Message);
            return (rows, 0);
        }
    };
    let mut current = 0;
    match span(cur.lines, len).filter(|_| !facts.whole) {
        Some((a, b)) => {
            for n in a.saturating_sub(CONTEXT)..a {
                rows.push(Row::Line { n, context: true });
            }
            current = rows.len();
            push_box(&mut rows, cur);
            for n in a..=b {
                rows.push(Row::Line { n, context: false });
            }
            for n in b + 1..(b + 1 + CONTEXT).min(len) {
                rows.push(Row::Line { n, context: true });
            }
        }
        // `whole file` mode, or a note without lines: every line, each of the file's notes above its first line.
        None => {
            let mut starts: Vec<(usize, &Note)> = facts
                .notes
                .of_path(&cur.path)
                .map(|x| (span(x.lines, len).map_or(0, |s| s.0), x))
                .collect();
            starts.sort_by_key(|&(first, x)| (first, x.id));
            let mut next = starts.iter().peekable();
            for n in 0..len {
                while let Some(&(_, note)) = next.next_if(|(first, _)| *first == n) {
                    if note.id == cur.id {
                        current = rows.len();
                    }
                    push_box(&mut rows, note);
                }
                rows.push(Row::Line { n, context: false });
            }
        }
    }
    (rows, current)
}

/// The first row that puts the current note's box a quarter of the body below its top, within the limits.
pub fn entry_top(page: &Page, view: &ReaderView) -> usize {
    page.current
        .saturating_sub(view.visible_rows() / 4)
        .min(page.max_top)
}

/// The reader's grid: a cell's width in points, the columns of the region, and the note box's wrap width in characters.
fn grid(view: &ReaderView, char_w: f32) -> (f32, usize, usize) {
    let cell = char_w * TEXT;
    let cols = (view.rect.width() / cell).floor().max(0.0) as usize;
    (cell, cols, cols.saturating_sub(4).max(8))
}

/// The column where a row's selectable text starts (unit `057`): after the gutter for a code line, inside the box for a note line.
/// A label row and a message row have no selectable text.
pub fn text_start(row: Row) -> Option<usize> {
    match row {
        Row::Line { .. } => Some(GUTTER + 1),
        Row::Note { .. } => Some(2),
        Row::Label { .. } | Row::Message => None,
    }
}

/// A row's text width in columns: a code line's whole width, not the part on the screen, or a note box row's length. 0 for other rows.
fn row_width(facts: &Facts, wrap_w: usize, row: Row) -> usize {
    match (row, facts.text) {
        (Row::Line { n, .. }, Text::Lines(l)) => l.get(n).map_or(0, |s| width(s)),
        (Row::Note { note, k }, _) => facts.notes.get(note).map_or(0, |n| {
            note_lines(&n.text, wrap_w)
                .get(k)
                .map_or(0, |s| s.chars().count())
        }),
        _ => 0,
    }
}

/// The selection end under a point (unit `057`): the shown row under it, and the column `x / char_w` from the row's text start,
/// to the nearest cell edge and clamped to the row's width, so the gutter gives column 0. Above the rows it is the first shown row's start;
/// below them, the last shown row's end. `None` without rows.
pub fn spot_at(view: &ReaderView, facts: &Facts, page: &Page, p: Pos2) -> Option<Spot> {
    let (cell, _, wrap_w) = grid(view, facts.char_w);
    let shown = view
        .visible_rows()
        .min(page.rows.len().saturating_sub(page.top));
    if shown == 0 {
        return None;
    }
    let k = ((p.y - view.body.top()) / CODE_ROW).floor();
    let last = page.top + shown - 1;
    if k < 0.0 {
        return Some(Spot {
            row: page.top,
            col: 0,
        });
    }
    let row = page.top + k as usize;
    if row > last {
        let w = row_width(facts, wrap_w, page.rows[last]);
        return Some(Spot { row: last, col: w });
    }
    let start = text_start(page.rows[row]).unwrap_or(0) as f32;
    let col = ((p.x - view.rect.left()) / cell - start).round().max(0.0) as usize;
    Some(Spot {
        row,
        col: col.min(row_width(facts, wrap_w, page.rows[row])),
    })
}

/// The selected text (unit `057`): a code row gives its line, the full line with its tabs and without the gutter, one line per row;
/// the rows of a note box give the note's text lines. Label and message rows give nothing.
pub fn selected_text(view: &ReaderView, facts: &Facts, sel: Selection) -> String {
    let (_, _, wrap_w) = grid(view, facts.char_w);
    let (rows, _) = layout(facts, wrap_w);
    let (a, b) = sel.ends();
    let lines: &[String] = match facts.text {
        Text::Lines(l) => l.as_slice(),
        Text::Message(_) => &[],
    };
    let mut out: Vec<String> = Vec::new();
    let mut k = a.row;
    while k <= b.row && k < rows.len() {
        match rows[k] {
            Row::Line { n, .. } => {
                let from = if k == a.row { a.col } else { 0 };
                let to = if k == b.row { b.col } else { usize::MAX };
                out.push(cut_cols(lines.get(n).map_or("", |s| s), from, to));
            }
            Row::Note { note, k: first } => {
                // The box's rows in the selection, as one stretch of the note's text.
                let mut end = k;
                while end < b.row
                    && matches!(rows.get(end + 1), Some(Row::Note { note: m, .. }) if *m == note)
                {
                    end += 1;
                }
                let Row::Note { k: last, .. } = rows[end] else {
                    unreachable!()
                };
                if let Some(n) = facts.notes.get(note) {
                    out.extend(note_stretch(&n.text, wrap_w, (first, a, k), (last, b, end)));
                }
                k = end;
            }
            Row::Label { .. } | Row::Message => {}
        }
        k += 1;
    }
    out.join("\n")
}

/// The note's text from one box row to another, as text lines. Each end is the box row, the selection's end, and the page row:
/// a stretch that starts above the selection's first row starts at the text line's start, and one that runs on past its last row ends at the line's end.
fn note_stretch(
    text: &str,
    wrap_w: usize,
    (r0, a, k0): (usize, Spot, usize),
    (r1, b, k1): (usize, Spot, usize),
) -> Vec<String> {
    let rows = box_rows(text, wrap_w);
    let (Some(first), Some(last)) = (rows.get(r0), rows.get(r1)) else {
        return Vec::new();
    };
    let src: Vec<Vec<char>> = text.lines().map(|l| l.chars().collect()).collect();
    let len = |line: usize| src.get(line).map_or(0, Vec::len);
    // The place in the text before a row's column.
    let before = |r: &BoxRow, col: usize| match r.at.get(col) {
        _ if col == 0 && r.first => 0,
        Some(&i) => i,
        None => r.at.last().map_or(0, |&i| i + 1),
    };
    // The place in the text after a row's columns up to `col`.
    let after = |r: &BoxRow, col: usize| {
        if col >= r.at.len() && r.last {
            len(r.line)
        } else if col == 0 {
            if r.first { 0 } else { r.at[0] }
        } else {
            r.at[col.min(r.at.len()) - 1] + 1
        }
    };
    let start = if k0 == a.row { before(first, a.col) } else { 0 };
    let end = if k1 == b.row {
        after(last, b.col)
    } else {
        len(last.line)
    };
    (first.line..=last.line)
        .map(|line| {
            let s = if line == first.line { start } else { 0 };
            let e = if line == last.line { end } else { len(line) };
            src[line][s.min(e)..e].iter().collect()
        })
        .collect()
}

/// The reader's draw items for a view and the facts.
pub fn build(view: &ReaderView, facts: &Facts) -> Page {
    let mut page = Page::default();
    let (cell, cols, wrap_w) = grid(view, facts.char_w);
    let x = |c: f32| view.rect.left() + c * cell;
    let Some(cur) = facts.notes.get(facts.note) else {
        return page;
    };
    let total = facts.notes.len();
    let number = |id: u64| facts.notes.ordinal(id).unwrap_or(0);
    let put = |page: &mut Page, c: usize, y: f32, s: &str, color: u32, bold: bool| {
        if !s.is_empty() {
            page.draws.push(Draw::Text {
                at: pos2(x(c as f32), y),
                text: s.to_string(),
                color,
                bold,
            });
        }
    };

    // The head row: the note's number, its path and lines, and the mode chips at the right.
    let hy = view.head.center().y;
    let label = format!("NOTE {} of {total}", number(cur.id));
    put(&mut page, 1, hy, &label, palette::LABEL, true);
    let mut right = cols.saturating_sub(1);
    let mut chips = Vec::new();
    for (word, whole) in [(WHOLE_CHIP, true), (LINES_CHIP, false)] {
        let n = word.chars().count();
        if right < n + 2 {
            break;
        }
        right -= n;
        chips.push((right, word, whole));
        right = right.saturating_sub(3);
    }
    for &(c, word, whole) in &chips {
        let rect = Rect::from_min_max(
            pos2(x(c as f32 - 1.0), view.head.top()),
            pos2(
                x((c + word.chars().count()) as f32 + 1.0),
                view.head.bottom(),
            ),
        );
        let hit = ReaderHit::Mode(whole);
        let active = whole == facts.whole;
        if active {
            page.draws.push(Draw::Fill {
                rect,
                color: palette::CURSOR,
            });
        } else if facts.hovered == Some(hit) {
            page.draws.push(Draw::Fill {
                rect,
                color: palette::TILE_HI,
            });
        }
        page.hits.push((rect, hit));
        let color = if active { palette::WHITE } else { palette::DIM };
        put(&mut page, c, hy, word, color, false);
    }
    let path_at = 1 + label.chars().count() + 2;
    let lines = lines_text(cur.lines).map(|l| format!(":{l}"));
    let lines_n = lines.as_ref().map_or(0, |l| l.chars().count());
    let chips_at = chips.last().map_or(cols, |&(c, _, _)| c.saturating_sub(1));
    let room = chips_at.saturating_sub(path_at + 2 + lines_n);
    let path = fit_left(&cur.path, room);
    put(&mut page, path_at, hy, &path, palette::INK2, false);
    if let Some(l) = &lines
        && room > 0
    {
        put(
            &mut page,
            path_at + path.chars().count(),
            hy,
            l,
            palette::DIM,
            false,
        );
    }
    page.draws.push(Draw::Fill {
        rect: Rect::from_min_max(
            pos2(view.rect.left(), view.head.bottom()),
            pos2(view.rect.right(), view.head.bottom() + 1.0),
        ),
        color: palette::RULE,
    });

    // The body: the rows from the scroll's first row.
    let (rows, current) = layout(facts, wrap_w);
    let vis = view.visible_rows();
    page.max_top = rows.len().saturating_sub(vis);
    page.top = facts.top.min(page.max_top);
    page.current = current;
    let lines: &[String] = match facts.text {
        Text::Lines(l) => l.as_slice(),
        Text::Message(_) => &[],
    };
    let message = match facts.text {
        Text::Message(m) => m.clone(),
        Text::Lines(_) => format!("{} is empty.", cur.path),
    };
    let len = lines.len();
    let shown: Vec<&Note> = if facts.whole || span(cur.lines, len).is_none() {
        facts.notes.of_path(&cur.path).collect()
    } else {
        vec![cur]
    };
    let cur_span = span(cur.lines, len);
    let noted = |n: usize| {
        shown
            .iter()
            .any(|x| span(x.lines, len).is_some_and(|(a, b)| (a..=b).contains(&n)))
    };
    let box_l = x(0.5);
    let box_r = x(cols.saturating_sub(1) as f32).max(box_l);
    let text_cols = cols.saturating_sub(GUTTER + 2);
    let top = page.top;
    let row_y = |k: usize| view.body.top() + (k as f32 - top as f32) * CODE_ROW;
    let selected = facts.selection.filter(|s| !s.is_empty()).map(|s| s.ends());
    let mut fills = Vec::new();
    let mut texts = Vec::new();
    for (k, row) in rows.iter().enumerate().skip(page.top).take(vis) {
        let y = row_y(k);
        let cy = y + CODE_ROW / 2.0;
        let band = Rect::from_min_max(pos2(box_l, y), pos2(box_r, y + CODE_ROW));
        // The row's fill under the text, which the selection mixes with.
        let mut under = palette::PANE;
        match *row {
            Row::Line { n, context } => {
                if cur_span.is_some_and(|(a, b)| (a..=b).contains(&n))
                    && shown.iter().any(|s| s.id == cur.id)
                {
                    under = mix_hex(palette::FOCUS, palette::PANE, LINES_MIX);
                    fills.push(Draw::Fill {
                        rect: band,
                        color: under,
                    });
                }
                let num_color = if noted(n) {
                    palette::FOCUS
                } else {
                    palette::FAINT
                };
                texts.push((1, cy, format!("{:>5}", n + 1), num_color, false));
                // A line with colors draws one text per span (unit `055`); a context line stays `DIM`.
                match facts.colors.and_then(|c| c.get(n)).filter(|_| !context) {
                    Some(spans) => {
                        let mut col = 0;
                        for sp in spans {
                            if col >= text_cols {
                                break;
                            }
                            let (t, next) = expand_from(&sp.text, col, text_cols);
                            if !t.trim().is_empty() {
                                texts.push((GUTTER + 1 + col, cy, t, sp.rgb, sp.bold));
                            }
                            col = next;
                        }
                    }
                    None => {
                        let color = if context { palette::DIM } else { palette::INK2 };
                        texts.push((GUTTER + 1, cy, expand(&lines[n], text_cols), color, false));
                    }
                }
            }
            Row::Label { note } => {
                let is_cur = note == cur.id;
                under = mix_hex(palette::FOCUS, palette::PANE, BOX_MIX);
                fills.push(Draw::Fill {
                    rect: band,
                    color: under,
                });
                let n = facts.notes.get(note);
                let head = format!("note {} of {total}", number(note));
                let color = if is_cur { palette::FOCUS } else { palette::DIM };
                texts.push((2, cy, head.clone(), color, true));
                let what = match n.and_then(|n| lines_text(n.lines)) {
                    Some(l) => format!("lines {l}"),
                    None => "whole file".to_string(),
                };
                texts.push((2 + head.chars().count() + 2, cy, what, palette::DIM, false));
            }
            Row::Note { note, k: line } => {
                under = mix_hex(palette::FOCUS, palette::PANE, BOX_MIX);
                fills.push(Draw::Fill {
                    rect: band,
                    color: under,
                });
                let text = facts
                    .notes
                    .get(note)
                    .map(|n| note_lines(&n.text, wrap_w))
                    .and_then(|l| l.get(line).cloned())
                    .unwrap_or_default();
                let color = if note == cur.id {
                    palette::INK
                } else {
                    palette::INK2
                };
                texts.push((2, cy, fit(&text, wrap_w), color, false));
            }
            Row::Message => {
                texts.push((
                    GUTTER + 1,
                    cy,
                    fit(&message, text_cols),
                    palette::FAINT,
                    false,
                ));
            }
        }
        // The selection (unit `057`): the first row from its column, whole rows to the box's right edge, and the last row to its column.
        if let (Some((a, b)), Some(start)) = (selected, text_start(*row))
            && (a.row..=b.row).contains(&k)
        {
            let from = x((start + if k == a.row { a.col } else { 0 }) as f32);
            let to = if k == b.row {
                x((start + b.col) as f32).min(box_r)
            } else {
                box_r
            };
            if to > from {
                fills.push(Draw::Fill {
                    rect: Rect::from_min_max(pos2(from, y), pos2(to, y + CODE_ROW)),
                    color: mix_hex(palette::CURSOR, under, SELECT_MIX),
                });
            }
        }
    }
    page.draws.extend(fills);
    for (c, cy, s, color, bold) in texts {
        put(&mut page, c, cy, &s, color, bold);
    }
    // The frames: around each shown box and its noted lines, 1 point wide, clipped to the body.
    for n in &shown {
        let Some(r0) = rows.iter().position(|r| *r == Row::Label { note: n.id }) else {
            continue;
        };
        let box_end = r0 + note_lines(&n.text, wrap_w).len();
        let end = match span(n.lines, len) {
            Some((_, b)) => rows[r0..]
                .iter()
                .position(|r| matches!(r, Row::Line { n, .. } if *n == b))
                .map_or(box_end, |k| r0 + k),
            None => box_end,
        };
        let frame = Rect::from_min_max(pos2(box_l, row_y(r0)), pos2(box_r, row_y(end + 1)));
        page.frames.push((n.id, frame));
        let color = if n.id == cur.id {
            palette::FOCUS
        } else {
            palette::FAINT
        };
        let body = view.body;
        let edge = |r: Rect| r.intersect(body);
        let mut edges = Vec::new();
        if frame.top() >= body.top() {
            edges.push(Rect::from_min_max(
                frame.left_top(),
                pos2(frame.right(), frame.top() + 1.0),
            ));
        }
        if frame.bottom() <= body.bottom() {
            edges.push(Rect::from_min_max(
                pos2(frame.left(), frame.bottom() - 1.0),
                frame.right_bottom(),
            ));
        }
        edges.push(Rect::from_min_max(
            frame.left_top(),
            pos2(frame.left() + 1.0, frame.bottom()),
        ));
        edges.push(Rect::from_min_max(
            pos2(frame.right() - 1.0, frame.top()),
            frame.right_bottom(),
        ));
        for r in edges {
            let r = edge(r);
            if r.is_positive() {
                page.draws.push(Draw::Fill { rect: r, color });
            }
        }
    }
    page.rows = rows;
    page
}

/// Paints the reader: the pane, then the draw items, clipped to the map region.
pub fn paint(painter: &Painter, view: &ReaderView, page: &Page) {
    let p = painter.with_clip_rect(view.rect);
    p.rect_filled(view.rect, 0.0, rgb(palette::PANE));
    p.rect_filled(view.head, 0.0, rgb(palette::SIDE));
    paint_draws(&p, &page.draws);
}

/// Whether a text gets syntax colors: at most `minimap_core::syntax::MAX_LINES` lines and `MAX_BYTES` bytes, as in the TUI.
pub fn colorable(lines: &[String]) -> bool {
    lines.len() <= minimap_core::syntax::MAX_LINES
        && lines.iter().map(|l| l.len() + 1).sum::<usize>() <= minimap_core::syntax::MAX_BYTES
}

/// What the [`Colorer`] thread sends back.
#[derive(Debug, Clone, PartialEq)]
pub enum Done {
    /// A text's colors: the path, the lines that were colored, and one span list per line.
    Colors {
        path: String,
        lines: Arc<Vec<String>>,
        spans: Arc<Vec<Vec<Span>>>,
    },
    /// The config's theme did not load; the built-in theme colors the text.
    ThemeError(String),
}

/// The reader's highlight thread (unit `055`). It loads the syntaxes and the theme once, and colors one text per request.
pub struct Colorer {
    jobs: Sender<(String, Arc<Vec<String>>)>,
    done: Receiver<Done>,
}

impl Colorer {
    /// Starts the thread with a `.tmTheme` file, or the built-in theme. `wake` runs after each result, so a window can repaint.
    pub fn spawn(theme: Option<PathBuf>, wake: impl Fn() + Send + 'static) -> Colorer {
        let (jobs, job_rx) = mpsc::channel::<(String, Arc<Vec<String>>)>();
        let (done_tx, done) = mpsc::channel();
        std::thread::spawn(move || {
            let (h, err) = Highlighter::new(theme.as_deref());
            if let Some(e) = err {
                let _ = done_tx.send(Done::ThemeError(e));
                wake();
            }
            for (path, lines) in job_rx {
                let text = lines.join("\n");
                let spans = Arc::new(h.highlight(&path, &text).lines);
                if done_tx.send(Done::Colors { path, lines, spans }).is_err() {
                    break;
                }
                wake();
            }
        });
        Colorer { jobs, done }
    }

    /// Asks for a text's colors.
    pub fn request(&self, path: &str, lines: Arc<Vec<String>>) {
        let _ = self.jobs.send((path.to_string(), lines));
    }

    /// The results since the last poll.
    pub fn poll(&self) -> Vec<Done> {
        self.done.try_iter().collect()
    }

    /// Waits for the next result, for tests.
    pub fn wait(&self, timeout: Duration) -> Option<Done> {
        self.done.recv_timeout(timeout).ok()
    }
}
