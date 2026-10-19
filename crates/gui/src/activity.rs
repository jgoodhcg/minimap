//! The agent history in a strip below the map (unit `051`): every agent event that this window received, newest first or oldest first (unit `057`),
//! with the local time, the kind, the file, and the session in its color (unit `057`). Notes show their text, and chips in the head row filter the rows by kind (unit `053`).
//!
//! [`build`] turns the records into draw items and the rects of the clickable rows, and [`paint`] draws them,
//! so tests read and click the strip with no window, as `panel` does. Text has a screen size, so the zoom never changes it.
//! Input goes through `GuiApp::on_event`.

use eframe::egui::{Painter, Rect, pos2, vec2};
use minimap_core::events::Kind as TrailKind;
use minimap_core::filetree::Entry;
use minimap_core::palette;

use crate::color::{SESSION, rgb};
use crate::panel::Draw;
use crate::scene::{Map, trail_rgb};
use crate::text::{fit, thousands};
use crate::tree::{ROW_H, TEXT};

/// The strip's rows: a head row and this many event rows, so it is 180 points high.
pub const ACTIVITY_ROWS: usize = 8;
pub const ACTIVITY_H: f32 = (ACTIVITY_ROWS + 1) as f32 * ROW_H;
/// The map region keeps at least this height; in a short window the strip gets less, and below two rows it hides.
pub const REGION_MIN_H: f32 = 160.0;
/// The strip keeps at most this many rows; the oldest drop.
pub const ACTIVITY_MAX: usize = 5000;
/// The session's shown characters: the start of Claude Code's UUID.
pub const SESSION_CHARS: usize = 8;
/// The head row's label, and the text without events.
pub const LABEL: &str = "AGENT HISTORY";
pub const EMPTY: &str = "No agent events yet. Claude Code's reads, searches, and edits show here.";
/// A note row keeps this many characters of its text's first line.
pub const NOTE_ROW_CHARS: usize = 200;
/// A session's swatch in the head row (unit `057`): a square this many points wide.
pub const SWATCH: f32 = 6.0;

/// What a row records: a trail event's kind, or a note (unit `053`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Act {
    Search,
    Read,
    Edit,
    Note,
}

impl From<TrailKind> for Act {
    fn from(k: TrailKind) -> Act {
        match k {
            TrailKind::Grep => Act::Search,
            TrailKind::Read => Act::Read,
            TrailKind::Edit => Act::Edit,
        }
    }
}

impl Act {
    /// The color of its word, its glow, and its marks: the trail colors, and `FOCUS` for a note.
    pub fn rgb(self) -> u32 {
        match self {
            Act::Search => trail_rgb(TrailKind::Grep),
            Act::Read => trail_rgb(TrailKind::Read),
            Act::Edit => trail_rgb(TrailKind::Edit),
            Act::Note => palette::FOCUS,
        }
    }
}

/// One row: one path of one trail event, or one note.
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    /// The row's number in the window's history, from 0. A press keeps it, so a new row does not change what a click selects.
    pub seq: u64,
    /// The local time when the window received the event, for example `11:41:07`.
    pub clock: String,
    pub kind: Act,
    /// The repo-relative path.
    pub path: String,
    /// An edit's or a note's first and last line.
    pub lines: Option<(u32, u32)>,
    pub session: Option<String>,
    /// A note's id in the window's notes, and the first line of its text, cut to [`NOTE_ROW_CHARS`].
    pub note: Option<u64>,
    pub text: String,
}

/// The kind's word in the strip: the trail's `grep` is a search.
pub fn kind_word(k: Act) -> &'static str {
    match k {
        Act::Read => "read",
        Act::Search => "search",
        Act::Edit => "edit",
        Act::Note => "note",
    }
}

/// The strip's filter (unit `053`): every row, or one kind's rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Filter {
    #[default]
    All,
    Notes,
    Edits,
    Reads,
    Searches,
}

impl Filter {
    /// The chips, in their order in the head row.
    pub const ALL: [Filter; 5] = [
        Filter::All,
        Filter::Notes,
        Filter::Edits,
        Filter::Reads,
        Filter::Searches,
    ];

    pub fn word(self) -> &'static str {
        match self {
            Filter::All => "all",
            Filter::Notes => "notes",
            Filter::Edits => "edits",
            Filter::Reads => "reads",
            Filter::Searches => "searches",
        }
    }

    pub fn matches(self, k: Act) -> bool {
        match self {
            Filter::All => true,
            Filter::Notes => k == Act::Note,
            Filter::Edits => k == Act::Edit,
            Filter::Reads => k == Act::Read,
            Filter::Searches => k == Act::Search,
        }
    }

    /// The text of a filtered strip without rows.
    pub fn empty(self) -> &'static str {
        match self {
            Filter::All => EMPTY,
            Filter::Notes => "No notes yet. An agent sends them with minimap note.",
            Filter::Edits => "No edits yet.",
            Filter::Reads => "No reads yet.",
            Filter::Searches => "No searches yet.",
        }
    }
}

/// The strip's order (unit `057`): the newest row at the top, or the oldest. The chip at the head row's right toggles it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Order {
    #[default]
    NewestFirst,
    OldestFirst,
}

impl Order {
    /// The chip's text.
    pub fn word(self) -> &'static str {
        match self {
            Order::NewestFirst => "newest first",
            Order::OldestFirst => "oldest first",
        }
    }

    pub fn flip(self) -> Order {
        match self {
            Order::NewestFirst => Order::OldestFirst,
            Order::OldestFirst => Order::NewestFirst,
        }
    }
}

/// The first line of a note's text, cut to [`NOTE_ROW_CHARS`] characters.
pub fn first_line(text: &str) -> String {
    let line = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    line.trim().chars().take(NOTE_ROW_CHARS).collect()
}

/// The rows that a filter shows, from the strip's top in its order.
pub fn shown(records: &[Record], filter: Filter, order: Order) -> Vec<&Record> {
    let rows = records.iter().filter(|r| filter.matches(r.kind));
    match order {
        Order::NewestFirst => rows.rev().collect(),
        Order::OldestFirst => rows.collect(),
    }
}

/// The color of a session's text and swatch (unit `057`): [`SESSION`] by its arrival among `sessions`, or `DIM` without a session.
pub fn session_rgb(sessions: &[String], s: Option<&str>) -> u32 {
    s.filter(|s| !s.is_empty())
        .and_then(|s| sessions.iter().position(|x| x == s))
        .map_or(palette::DIM, |k| SESSION[k % SESSION.len()])
}

/// A session id as the strip shows it: its first [`SESSION_CHARS`] characters, or `—` without one.
pub fn short_session(s: Option<&str>) -> String {
    match s.filter(|s| !s.is_empty()) {
        Some(s) => s.chars().take(SESSION_CHARS).collect(),
        None => "—".to_string(),
    }
}

/// A text cut at the left with `…` to a room in characters, so a path keeps its file name.
pub fn fit_left(s: &str, room: usize) -> String {
    let n = s.chars().count();
    if n <= room {
        return s.to_string();
    }
    if room == 0 {
        return String::new();
    }
    let tail: String = s.chars().skip(n - (room - 1)).collect();
    format!("…{tail}")
}

/// The strip's parts on the screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActivityView {
    /// The whole strip, its 1-point top rule included.
    pub rect: Rect,
    /// The first row: the label and the counts.
    pub head: Rect,
    /// The event rows below the head.
    pub rows: Rect,
}

impl ActivityView {
    pub fn new(rect: Rect) -> ActivityView {
        let head = Rect::from_min_size(rect.min, vec2(rect.width(), ROW_H));
        let rows = Rect::from_min_max(
            pos2(rect.left(), head.bottom()),
            rect.max.max(head.right_bottom()),
        );
        ActivityView { rect, head, rows }
    }

    /// The whole event rows that fit.
    pub fn visible_rows(&self) -> usize {
        (self.rows.height() / ROW_H + 1e-3).floor().max(0.0) as usize
    }
}

/// What the strip shows.
pub struct Facts<'a> {
    pub map: &'a Map,
    /// Oldest first, as received.
    pub records: &'a [Record],
    /// The rows scrolled past at the top.
    pub top: usize,
    /// The distinct sessions received, in arrival order.
    pub sessions: &'a [String],
    pub selection: Option<Entry>,
    /// The row or the chip under the pointer.
    pub hovered: Option<StripHit>,
    /// The kind of rows to show (unit `053`), and their order (unit `057`).
    pub filter: Filter,
    pub order: Order,
    pub char_w: f32,
    /// A message in place of the empty text, such as another minimap that has the trail.
    pub notice: Option<&'a str>,
}

/// A clickable part of the strip: a row with a card, by its number, a filter chip (unit `053`), or the sort chip (unit `057`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StripHit {
    Row(u64),
    Filter(Filter),
    Order,
}

/// The strip's draw items, in paint order, and its clickable rows and chips.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Strip {
    pub draws: Vec<Draw>,
    pub hits: Vec<(Rect, StripHit)>,
}

impl Strip {
    /// The clickable row or chip under a point.
    pub fn hit(&self, p: eframe::egui::Pos2) -> Option<StripHit> {
        self.hits
            .iter()
            .find(|(r, _)| r.contains(p))
            .map(|&(_, s)| s)
    }

    /// The rect of a clickable row or chip.
    pub fn rect_of(&self, h: StripHit) -> Option<Rect> {
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

/// The file of a row with a card, or `None` for a path with no card (deleted, excluded, or ignored).
pub fn file_of(map: &Map, path: &str) -> Option<usize> {
    match map.entry(path) {
        Some(Entry::File(f)) if map.visible(f) => Some(f),
        _ => None,
    }
}

/// The largest first row: the first row of the last full page of `rows` shown rows.
pub fn max_top(rows: usize, view: &ActivityView) -> usize {
    rows.saturating_sub(view.visible_rows())
}

/// The strip's draw items for a view and the facts.
pub fn build(view: &ActivityView, facts: &Facts) -> Strip {
    let mut out = Strip::default();
    let cell = facts.char_w * TEXT;
    let cols = (view.rect.width() / cell).floor().max(0.0) as usize;
    let w = cols.saturating_sub(2);
    let x = |c: usize| view.rect.left() + c as f32 * cell;
    let put = |out: &mut Strip, c: usize, y: f32, s: &str, color: u32, bold: bool| {
        if !s.is_empty() {
            out.draws.push(Draw::Text {
                at: pos2(x(c), y + ROW_H / 2.0),
                text: s.to_string(),
                color,
                bold,
            });
        }
    };
    // The head row: the label, the filter chips with their counts (unit `053`), and the sessions at the right.
    let mut counts = [0usize; Filter::ALL.len()];
    for r in facts.records {
        for (k, f) in Filter::ALL.iter().enumerate() {
            if f.matches(r.kind) {
                counts[k] += 1;
            }
        }
    }
    let head_y = view.head.top();
    put(&mut out, 1, head_y, &fit(LABEL, w), palette::LABEL, true);
    let mut c = 1 + LABEL.len() + 3;
    for (k, f) in Filter::ALL.iter().enumerate() {
        let chip = format!("{} {}", f.word(), thousands(counts[k] as u64));
        let n = chip.chars().count();
        if c + n > 1 + w {
            break;
        }
        let rect = Rect::from_min_max(pos2(x(c - 1), head_y), pos2(x(c + n + 1), head_y + ROW_H));
        let hit = StripHit::Filter(*f);
        let active = *f == facts.filter;
        if active {
            out.draws.push(Draw::Fill {
                rect,
                color: palette::CURSOR,
            });
        } else if facts.hovered == Some(hit) {
            out.draws.push(Draw::Fill {
                rect,
                color: palette::TILE_HI,
            });
        }
        out.hits.push((rect, hit));
        let color = if active { palette::WHITE } else { palette::DIM };
        put(&mut out, c, head_y, &chip, color, false);
        c += n + 3;
    }
    // The right side (unit `057`): `M newer above · K sessions`, a swatch per session when there is more than one, and the sort chip
    // at the right. The right side drops before the filter chips: first the scroll count, then the sessions, then the sort chip.
    let sessions = facts.sessions.len();
    let mut right = format!(
        "{sessions} {}",
        if sessions == 1 { "session" } else { "sessions" }
    );
    let swatches = if sessions > 1 {
        sessions.min(SESSION.len()) + usize::from(sessions > SESSION.len())
    } else {
        0
    };
    let block = |s: &str| s.chars().count() + if swatches > 0 { 1 + swatches } else { 0 };
    let chip = facts.order.word();
    let chip_n = chip.chars().count();
    if c + chip_n <= 1 + w {
        let chip_at = 1 + w - chip_n;
        // The sessions end 3 cells before the chip's text, as the filter chips are spaced.
        let end = chip_at - 3;
        if facts.top > 0 {
            let newer = match facts.order {
                Order::NewestFirst => "newer",
                Order::OldestFirst => "older",
            };
            let scrolled = format!("{} {newer} above · {right}", facts.top);
            if c + block(&scrolled) <= end {
                right = scrolled;
            }
        }
        if c + block(&right) <= end {
            let at = end - block(&right);
            put(&mut out, at, head_y, &right, palette::FAINT, false);
            let first = at + right.chars().count() + 1;
            for (k, &color) in SESSION.iter().enumerate().take(swatches) {
                let cell_rect = Rect::from_min_max(
                    pos2(x(first + k), head_y),
                    pos2(x(first + k + 1), head_y + ROW_H),
                );
                out.draws.push(Draw::Fill {
                    rect: Rect::from_center_size(cell_rect.center(), vec2(SWATCH, SWATCH)),
                    color,
                });
            }
            if sessions > SESSION.len() {
                put(
                    &mut out,
                    first + SESSION.len(),
                    head_y,
                    "…",
                    palette::FAINT,
                    false,
                );
            }
        }
        let rect = Rect::from_min_max(
            pos2(x(chip_at - 1), head_y),
            pos2(x(chip_at + chip_n + 1), head_y + ROW_H),
        );
        if facts.hovered == Some(StripHit::Order) {
            out.draws.push(Draw::Fill {
                rect,
                color: palette::TILE_HI,
            });
        }
        out.hits.push((rect, StripHit::Order));
        put(&mut out, chip_at, head_y, chip, palette::DIM, false);
    }
    let rows = shown(facts.records, facts.filter, facts.order);
    if rows.is_empty() {
        let text = match facts.notice {
            Some(n) if facts.records.is_empty() => n,
            _ => facts.filter.empty(),
        };
        put(
            &mut out,
            1,
            view.rows.top(),
            &fit(text, w),
            palette::FAINT,
            false,
        );
        return out;
    }
    // Event rows in the strip's order, from the scroll's first row.
    let selected = match facts.selection {
        Some(Entry::File(f)) => Some(f),
        _ => None,
    };
    for (k, r) in rows
        .iter()
        .skip(facts.top)
        .take(view.visible_rows())
        .enumerate()
    {
        let y = view.rows.top() + k as f32 * ROW_H;
        let row = Rect::from_min_max(
            pos2(view.rect.left(), y),
            pos2(view.rect.right(), y + ROW_H),
        );
        let file = file_of(facts.map, &r.path);
        // A note row clicks without a card too, such as a note on a deleted file: the reader shows it (unit `054`).
        let clickable = file.is_some() || r.note.is_some();
        let is_sel = file.is_some() && file == selected;
        if is_sel {
            out.draws.push(Draw::Fill {
                rect: row,
                color: palette::CURSOR,
            });
        } else if clickable && facts.hovered == Some(StripHit::Row(r.seq)) {
            out.draws.push(Draw::Fill {
                rect: row,
                color: palette::TILE_HI,
            });
        }
        if clickable {
            out.hits.push((row, StripHit::Row(r.seq)));
        }
        put(&mut out, 1, y, &r.clock, palette::DIM, false);
        put(&mut out, 11, y, kind_word(r.kind), r.kind.rgb(), false);
        // The session text says who (unit `057`); the kind's color says what happened.
        let session = short_session(r.session.as_deref());
        let session_color = session_rgb(facts.sessions, r.session.as_deref());
        let session_at = (1 + w).saturating_sub(SESSION_CHARS);
        put(&mut out, session_at, y, &session, session_color, false);
        let lines = match r.lines {
            Some((a, b)) if a == b => format!("{a}"),
            Some((a, b)) => format!("{a}–{b}"),
            None => String::new(),
        };
        let path_at = 19;
        let lines_w = if lines.is_empty() {
            0
        } else {
            lines.chars().count() + 2
        };
        let room = session_at.saturating_sub(path_at + 2 + lines_w);
        // A note's path takes at most half of the room after the kind, so its text shows (unit `053`).
        let path_room = if r.kind == Act::Note && !r.text.is_empty() {
            room.min(session_at.saturating_sub(path_at + 2) / 2)
        } else {
            room
        };
        let path = fit_left(&r.path, path_room);
        let path_color = match (file, is_sel) {
            (None, _) => palette::FAINT,
            (_, true) => palette::WHITE,
            _ => palette::INK2,
        };
        put(&mut out, path_at, y, &path, path_color, false);
        let mut at = path_at + path.chars().count() + 2;
        if !lines.is_empty() && room > 0 {
            put(&mut out, at, y, &lines, palette::DIM, false);
            at += lines.chars().count() + 2;
        }
        if r.kind == Act::Note && !r.text.is_empty() {
            let text_room = session_at.saturating_sub(at + 2);
            put(
                &mut out,
                at,
                y,
                &fit(&r.text, text_room),
                palette::INK,
                false,
            );
        }
    }
    out
}

/// Paints the strip: the background, the top rule, and the draw items.
pub fn paint(painter: &Painter, view: &ActivityView, strip: &Strip) {
    let p = painter.with_clip_rect(view.rect);
    p.rect_filled(view.rect, 0.0, rgb(palette::SIDE));
    p.rect_filled(
        Rect::from_min_size(view.rect.min, vec2(view.rect.width(), 1.0)),
        0.0,
        rgb(palette::RULE),
    );
    crate::panel::paint_draws(&p, &strip.draws);
}
