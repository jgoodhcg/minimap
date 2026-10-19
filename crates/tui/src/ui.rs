//! The screen: top bar, file tree, map or file viewer, selection panel, status bar, and the help overlay.
//!
//! It ports the drawing in `mockups/v5-template.html` (`drawMap`, `drawTree`, `drawSide`, `drawTop`, `drawStatus`)
//! to a Ratatui `Buffer`.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use ratatui::widgets::Widget;

use std::collections::HashSet;

use minimap_core::events::Kind;
use minimap_core::history::{Heat, ago, ago_phrase, describe};
use minimap_core::layout::Header;
use minimap_core::outline::OutlineItem;
use minimap_core::shape::LineShape;

use crate::app::{App, GitHistory, View};
use crate::filetree::Entry;
use crate::keymap::{Command as K, Key, Table};
use crate::search::Search;
use crate::shape::{BIT, Braille};
use crate::text::{fit, fit_left, kilo, thousands, wrap};
use crate::viewer::{self, Sign, Viewer};
use crate::{icons, theme};
use minimap_core::diff::Tag;

/// Writes cells into a buffer, clipped to a rectangle, in screen coordinates relative to `area`.
struct Canvas<'a> {
    buf: &'a mut Buffer,
    area: Rect,
    clip: (i32, i32, i32, i32),
}

impl<'a> Canvas<'a> {
    fn new(buf: &'a mut Buffer, area: Rect) -> Self {
        let clip = (0, 0, i32::from(area.width), i32::from(area.height));
        Canvas { buf, area, clip }
    }

    fn clip(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let (w, h) = (i32::from(self.area.width), i32::from(self.area.height));
        self.clip = (x0.max(0), y0.max(0), x1.min(w), y1.min(h));
    }

    fn put(
        &mut self,
        x: i32,
        y: i32,
        ch: Option<char>,
        fg: Option<Color>,
        bg: Option<Color>,
        bold: Option<bool>,
    ) {
        let (x0, y0, x1, y1) = self.clip;
        if x < x0 || y < y0 || x >= x1 || y >= y1 {
            return;
        }
        let pos = (self.area.x + x as u16, self.area.y + y as u16);
        let Some(cell) = self.buf.cell_mut(pos) else {
            return;
        };
        if let Some(ch) = ch {
            cell.set_char(ch);
        }
        if let Some(fg) = fg {
            cell.fg = fg;
        }
        if let Some(bg) = bg {
            cell.bg = bg;
        }
        match bold {
            Some(true) => cell.modifier.insert(Modifier::BOLD),
            Some(false) => cell.modifier.remove(Modifier::BOLD),
            None => {}
        }
    }

    /// Writes one char with every style part: the viewer's syntax colors use italic too.
    #[allow(clippy::too_many_arguments)]
    fn styled(&mut self, x: i32, y: i32, ch: char, fg: Color, bg: Color, bold: bool, italic: bool) {
        self.put(x, y, Some(ch), Some(fg), Some(bg), Some(bold));
        let (x0, y0, x1, y1) = self.clip;
        if x < x0 || y < y0 || x >= x1 || y >= y1 {
            return;
        }
        let pos = (self.area.x + x as u16, self.area.y + y as u16);
        if let Some(cell) = self.buf.cell_mut(pos) {
            if italic {
                cell.modifier.insert(Modifier::ITALIC);
            } else {
                cell.modifier.remove(Modifier::ITALIC);
            }
        }
    }

    /// Writes `s` and returns its length in cells. Callers fit `s` to the room first.
    fn text(&mut self, x: i32, y: i32, s: &str, fg: Color, bg: Option<Color>, bold: bool) -> i32 {
        let mut n = 0;
        for (k, ch) in s.chars().enumerate() {
            self.put(x + k as i32, y, Some(ch), Some(fg), bg, Some(bold));
            n += 1;
        }
        n
    }

    fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, bg: Color) {
        for j in 0..h {
            for k in 0..w {
                self.put(
                    x + k,
                    y + j,
                    Some(' '),
                    Some(theme::INK),
                    Some(bg),
                    Some(false),
                );
            }
        }
    }

    fn frame(&mut self, x: i32, y: i32, w: i32, h: i32, border: Color, bg: Color) {
        self.fill(x, y, w, h, bg);
        for k in 1..w - 1 {
            self.put(x + k, y, Some('─'), Some(border), None, None);
            self.put(x + k, y + h - 1, Some('─'), Some(border), None, None);
        }
        for j in 1..h - 1 {
            self.put(x, y + j, Some('│'), Some(border), None, None);
            self.put(x + w - 1, y + j, Some('│'), Some(border), None, None);
        }
        self.put(x, y, Some('┌'), Some(border), None, None);
        self.put(x + w - 1, y, Some('┐'), Some(border), None, None);
        self.put(x, y + h - 1, Some('└'), Some(border), None, None);
        self.put(x + w - 1, y + h - 1, Some('┘'), Some(border), None, None);
    }
}

/// The whole screen as one widget. Call [`App::resize`] with the same area first.
pub struct Screen<'a>(pub &'a App);

impl Widget for Screen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let app = self.0;
        let v = app.view();
        let mut c = Canvas::new(buf, area);
        c.fill(0, 0, v.cols, v.rows, theme::PANE);
        match &app.viewer {
            Some(vw) => draw_viewer(&mut c, app, vw, v),
            None => draw_map(&mut c, app, v),
        }
        if v.tw > 0 {
            draw_tree(&mut c, app, v);
        }
        if v.rw > 0 {
            draw_side(&mut c, app, v);
        }
        draw_top(&mut c, app, v);
        draw_status(&mut c, app, v);
        if app.help {
            draw_help(&mut c, app, v);
        }
    }
}

/// A header's text: the repo name for the root, else the directory's own name (a compacted chain shows as
/// `src/quayside`), then `/`, fitted to the block width.
fn header_text(app: &App, h: &Header) -> String {
    let name = if h.depth == 0 {
        &app.tree.root_name
    } else {
        &app.tree.nodes[h.node].name
    };
    fit(&format!("{name}/"), h.w.max(0) as usize)
}

/// The width of a header's text, without its rule. A click on these cells selects the directory.
pub fn header_text_width(app: &App, h: &Header) -> i32 {
    header_text(app, h).chars().count() as i32
}

/// Draws a header and its rule (`docs/design.md` `Map`). A selected directory's header text pulses.
fn draw_header(c: &mut Canvas, app: &App, h: &Header, sx: i32, sy: i32) {
    let text = header_text(app, h);
    let sel = app.selected_dir() == Some(h.node);
    let bg = sel.then(|| theme::selection_bg(app.pulse_level()));
    // Headers are neutral: color belongs to file types (`docs/design.md` `Color`).
    let (fg, bold) = match h.depth {
        0 => (theme::FOCUS, true),
        1 | 2 => (theme::INK, true),
        _ => (theme::INK2, false),
    };
    let n = c.text(sx, sy, &text, fg, bg, bold);
    let ch = match h.depth {
        0 => return,
        1 => '━',
        _ => '─',
    };
    for x in n + 1..h.w - 1 {
        c.put(sx + x, sy, Some(ch), Some(theme::RULE), None, None);
    }
}

/// The middle column: the map region (`drawMap` in the v5 mockup). Only directories have text. Each file is one cell.
fn draw_map(c: &mut Canvas, app: &App, v: View) {
    let (x0, y0, x1, y1) = (v.mx, v.my, v.mx + v.mw, v.my + v.mh);
    c.clip(x0, y0, x1, y1);
    let (ox, oy) = (x0 - app.cam_x, y0 - app.cam_y);
    if let Some(n) = app.selected_dir() {
        let h = app.header(n);
        c.fill(ox + h.x, oy + h.y, h.w, h.h, theme::BLOCK_BG);
    }
    for h in &app.layout.headers {
        let (sx, sy) = (ox + h.x, oy + h.y);
        if sy < y0 || sy >= y1 || sx >= x1 || sx + h.w <= x0 {
            continue;
        }
        if app.tree.nodes[h.node].hidden && !app.show_hidden {
            continue;
        }
        draw_header(c, app, h, sx, sy);
    }
    let sel_bg = theme::selection_bg(app.pulse_level());
    for (i, &(x, y)) in app.layout.tiles.iter().enumerate() {
        let (sx, sy) = (ox + x, oy + y);
        if sx < x0 || sx >= x1 || sy < y0 || sy >= y1 || !app.file_visible(i) {
            // A hidden or ignored file's slot stays empty.
            continue;
        }
        let name = &app.tree.files[i].name;
        let glyph = if app.icons {
            icons::for_name(name).glyph
        } else {
            '■'
        };
        // The glyph shows the file type; the background shows state (`docs/design.md` `Color`).
        let sel = app.selected_file() == Some(i);
        let fg = if sel {
            theme::WHITE
        } else {
            icons::type_color(name, app.index.files[i].binary)
        };
        // Priority: selection pulse, hover, search match, CC focus, CC edit, mine, CC read, CC grep, git status, heat.
        // The focus, mine, and git status layers do not exist yet.
        let trail = app.trail_mark(i);
        let bg = if sel {
            Some(sel_bg)
        } else if app.hover == Some(Entry::File(i)) {
            Some(theme::TILE_HI)
        } else if app.search.as_ref().is_some_and(|s| s.is_marked(i)) {
            Some(theme::state_bg(theme::SEARCH_HI))
        } else if let Some((m, fresh)) = trail {
            Some(theme::trail_bg(m.kind, fresh))
        } else {
            // Heat is the lowest cell state.
            app.heat_step(i).map(theme::heat_bg)
        };
        c.put(sx, sy, Some(glyph), Some(fg), bg, Some(sel));
    }
    draw_overview(c, app, v);
}

/// The whole map's shape as one braille dot per visible file, and the viewport (`drawOverview`).
/// A cell with a trail file draws its dots in the trail color, edit before read before grep, inside the viewport too.
/// It shows no selection, no other layers, and no text.
fn draw_overview(c: &mut Canvas, app: &App, v: View) {
    let Some(o) = app.overview() else {
        return;
    };
    c.fill(o.x, o.y, o.bw, o.bh, theme::MINI_BG);
    let mut bits = vec![0u8; (o.w * o.h) as usize];
    let mut trail: Vec<Option<Kind>> = vec![None; bits.len()];
    for (i, &(x, y)) in app.layout.tiles.iter().enumerate() {
        if !app.file_visible(i) {
            continue;
        }
        let dx = (f64::from(x) * o.s * 2.0).floor() as i32;
        let dy = (f64::from(y) * o.s * 4.0).floor() as i32;
        let (cx, cy) = (dx >> 1, dy >> 2);
        if cx < o.w && cy < o.h {
            let k = (cy * o.w + cx) as usize;
            bits[k] |= BIT[(dy & 3) as usize][(dx & 1) as usize];
            if let Some((m, _)) = app.trail_mark(i) {
                trail[k] = trail[k].max(Some(m.kind));
            }
        }
    }
    let (vx0, vy0, vx1, vy1) = o.viewport((app.cam_x, app.cam_y), v.mw, v.mh);
    for cy in 0..o.h {
        for cx in 0..o.w {
            let inside = cx >= vx0 && cx <= vx1 && cy >= vy0 && cy <= vy1;
            let edge = inside && (cx == vx0 || cx == vx1 || cy == vy0 || cy == vy1);
            let b = bits[(cy * o.w + cx) as usize];
            let ch = if b > 0 {
                char::from_u32(0x2800 + u32::from(b)).unwrap_or(' ')
            } else {
                ' '
            };
            let fg = match trail[(cy * o.w + cx) as usize] {
                Some(kind) => theme::trail_color(kind),
                None if inside => theme::INK2,
                None => theme::MINI_DOT,
            };
            let bg = if edge {
                theme::VIEW_EDGE
            } else if inside {
                theme::VIEW_IN
            } else {
                theme::MINI_BG
            };
            c.put(
                o.x + 1 + cx,
                o.y + 1 + cy,
                Some(ch),
                Some(fg),
                Some(bg),
                Some(false),
            );
        }
    }
}

/// The viewer in the map region (units 029 and 030): line numbers, the sign column, then the text with syntax colors,
/// the cursor row, and the find marks. In diff mode, added and removed rows get the diff backgrounds. Lines cut at the
/// right edge.
fn draw_viewer(c: &mut Canvas, app: &App, vw: &Viewer, v: View) {
    let (x0, y0, x1, y1) = (v.mx, v.my, v.mx + v.mw, v.my + v.mh);
    c.clip(x0, y0, x1, y1);
    let digits = vw.lines.len().to_string().chars().count().max(3) as i32;
    let sign_x = x0 + digits + 1;
    let text_x = sign_x + 2;
    let colored = if vw.plain {
        None
    } else {
        app.colored_for(&vw.path)
    };
    let plain = app.text_fg.unwrap_or(theme::INK);
    let find_bg = theme::mix(theme::FOCUS, theme::PANE, theme::STATE_MIX);
    let add_bg = theme::mix(theme::DIFF_ADD, theme::PANE, theme::DIFF_MIX);
    let remove_bg = theme::mix(theme::DIFF_REMOVE, theme::PANE, theme::DIFF_MIX);
    for r in 0..v.mh {
        let row = vw.top + r as usize;
        if row >= vw.len() {
            break;
        }
        let y = y0 + r;
        let cur = row == vw.cursor;
        let tag = vw.row_tag(row);
        let row_bg = match (cur, vw.diff_mode, tag) {
            (true, _, _) => theme::VIEW_IN,
            (false, true, Tag::Added) => add_bg,
            (false, true, Tag::Removed) => remove_bg,
            _ => theme::PANE,
        };
        if row_bg != theme::PANE {
            c.fill(x0, y, v.mw, 1, row_bg);
        }
        let num = match vw.row_line(row) {
            Some(l) => format!("{:>w$}", l + 1, w = digits as usize),
            None => " ".repeat(digits as usize),
        };
        let num_fg = if cur { theme::INK } else { theme::FAINT };
        c.text(x0, y, &num, num_fg, Some(row_bg), false);
        let sign = match vw.sign(row) {
            Sign::None => None,
            Sign::Added => Some(('▎', theme::DIFF_ADD)),
            Sign::RemovedBelow => Some(('▁', theme::DIFF_REMOVE)),
            Sign::RemovedAbove => Some(('▔', theme::DIFF_REMOVE)),
            Sign::Plus => Some(('+', theme::DIFF_ADD)),
            Sign::Minus => Some(('-', theme::DIFF_REMOVE)),
        };
        if let Some((ch, fg)) = sign {
            c.styled(sign_x, y, ch, fg, row_bg, true, false);
        }
        // Each char with its style: from the syntax colors when they have arrived, else plain.
        // A removed row takes its colors from the HEAD text.
        let spans = match vw.row_old(row) {
            Some(o) => colored.and_then(|k| k.head_lines.get(o)),
            None => vw
                .row_line(row)
                .and_then(|l| colored.and_then(|k| k.lines.get(l))),
        };
        let chars: Vec<(char, Color, bool, bool)> = match spans {
            Some(spans) => spans
                .iter()
                .flat_map(|sp| sp.text.chars().map(|ch| (ch, sp.fg, sp.bold, sp.italic)))
                .collect(),
            None => vw
                .row_text(row)
                .chars()
                .map(|ch| (ch, plain, false, false))
                .collect(),
        };
        let marks = vw.matches_in(row);
        let mut col = 0usize;
        for (k, &(ch, fg, bold, italic)) in chars.iter().enumerate() {
            let w = viewer::char_width(ch, col);
            let at = col;
            col += w;
            if at < vw.left || w == 0 {
                continue;
            }
            let x = text_x + (at - vw.left) as i32;
            if x >= x1 {
                break;
            }
            let bg = if marks.iter().any(|&(a, b)| k >= a && k < b) {
                find_bg
            } else {
                row_bg
            };
            if ch == '\t' {
                for dx in 0..w as i32 {
                    c.styled(x + dx, y, ' ', fg, bg, false, false);
                }
                continue;
            }
            c.styled(x, y, ch, fg, bg, bold, italic);
            if w == 2 {
                c.styled(x + 1, y, ' ', fg, bg, false, false);
            }
        }
    }
}

/// A key hint: `keys words` pieces joined by ` · `. A piece without keys drops out, so a hint names only bound keys (unit `033`).
fn hint(pieces: &[(String, &str)]) -> String {
    pieces
        .iter()
        .filter(|(keys, _)| !keys.is_empty())
        .map(|(keys, words)| {
            if words.is_empty() {
                keys.clone()
            } else {
                format!("{keys} {words}")
            }
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

/// The file tree's key hint (`j/k · l/h · t hide · [ ] width` by default).
fn tree_hint(t: &Table) -> String {
    hint(&[
        (t.join(&[K::Down, K::Up], "/"), ""),
        (t.join(&[K::TreeIn, K::TreeOut], "/"), ""),
        (t.label(K::ToggleTree), "hide"),
        (t.join(&[K::NarrowTree, K::WidenTree], " "), "width"),
    ])
}

/// The left column: `FILES` and the visible file count, the tree rows, then the key hint (`drawTree`).
fn draw_tree(c: &mut Canvas, app: &App, v: View) {
    let w = v.tw;
    c.clip(0, 1, w + 1, v.rows - 1);
    c.fill(0, 1, w, v.rows - 2, theme::SIDE);
    let (edge, edge_color) = if app.dragging_tree_edge() {
        ('┃', theme::DIM)
    } else {
        ('│', theme::RULE)
    };
    for y in 1..v.rows - 1 {
        c.put(w, y, Some(edge), Some(edge_color), None, None);
    }
    if let Some(s) = app.search.as_ref().filter(|s| s.active) {
        draw_picker(c, app, s, v);
        return;
    }
    c.text(
        1,
        1,
        &fit("FILES", (w - 2).max(0) as usize),
        theme::LABEL,
        None,
        true,
    );
    let ft = &app.filetree;
    let count = thousands(ft.count(0, app.show_hidden) as u64);
    c.text(
        w - 1 - count.chars().count() as i32,
        1,
        &count,
        theme::FAINT,
        None,
        false,
    );

    let (y, h) = v.tree_rows();
    let rows = app.tree_rows();
    let cur = ft.cursor_index(&rows);
    for (k, row) in rows.iter().enumerate().skip(ft.top).take(h as usize) {
        let ry = y + (k - ft.top) as i32;
        let sel = cur == Some(k);
        let bg = sel.then_some(theme::CURSOR);
        if sel {
            c.fill(0, ry, w, 1, theme::CURSOR);
        }
        let ix = 1 + 2 * row.depth as i32;
        let name_x = if app.icons { ix + 4 } else { ix + 2 };
        match row.entry {
            Entry::Dir(n) => {
                let open = ft.is_open(n);
                let top_level = row.depth == 0;
                let color = if top_level { theme::INK } else { theme::INK2 };
                let fg = if sel { theme::WHITE } else { color };
                c.put(
                    ix,
                    ry,
                    Some(if open { '▾' } else { '▸' }),
                    Some(if sel { theme::WHITE } else { theme::DIM }),
                    bg,
                    Some(false),
                );
                if app.icons {
                    let glyph = if open {
                        icons::FOLDER_OPEN
                    } else {
                        icons::FOLDER_CLOSED
                    };
                    c.put(ix + 2, ry, Some(glyph), Some(fg), bg, Some(false));
                }
                let count = thousands(ft.count(n, app.show_hidden) as u64);
                let count_x = w - 1 - count.chars().count() as i32;
                let room = (count_x - 1 - name_x).max(0) as usize;
                let name = fit(&format!("{}/", app.tree.nodes[n].name), room);
                c.text(name_x, ry, &name, fg, bg, top_level || sel);
                c.text(
                    count_x,
                    ry,
                    &count,
                    if sel { theme::INK2 } else { theme::DIM },
                    bg,
                    false,
                );
            }
            Entry::File(f) => {
                let name = &app.tree.files[f].name;
                let binary = app.index.files[f].binary;
                if let Some((m, _)) = app.trail_mark(f) {
                    c.put(
                        0,
                        ry,
                        Some('▌'),
                        Some(theme::trail_color(m.kind)),
                        bg,
                        Some(false),
                    );
                }
                if app.icons {
                    let glyph = icons::for_name(name).glyph;
                    let color = icons::type_color(name, binary);
                    c.put(ix + 2, ry, Some(glyph), Some(color), bg, Some(false));
                }
                let room = (w - 1 - name_x).max(0) as usize;
                let fg = if sel { theme::WHITE } else { theme::INK2 };
                c.text(name_x, ry, &fit(name, room), fg, bg, sel);
            }
        }
    }
    c.text(
        1,
        v.rows - 2,
        &fit(&tree_hint(&app.keymap.map), (w - 2).max(0) as usize),
        theme::FAINT,
        None,
        false,
    );
}

/// The picker in the left column (unit 026): `SEARCH` and `N / M`, the results best at the bottom, then the prompt.
fn draw_picker(c: &mut Canvas, app: &App, s: &Search, v: View) {
    let w = v.tw;
    c.text(
        1,
        1,
        &fit("SEARCH", (w - 2).max(0) as usize),
        theme::LABEL,
        None,
        true,
    );
    let count = format!(
        "{} / {}",
        thousands(s.results.len() as u64),
        thousands(s.total as u64)
    );
    c.text(
        w - 1 - count.chars().count() as i32,
        1,
        &count,
        theme::FAINT,
        None,
        false,
    );
    let (y, h) = v.tree_rows();
    for (r, hit) in s.visible().iter().enumerate() {
        let ry = y + h - 1 - r as i32;
        let sel = s.cursor == Some(s.scroll + r);
        let bg = sel.then_some(theme::CURSOR);
        if sel {
            c.fill(0, ry, w, 1, theme::CURSOR);
        }
        let tf = &app.tree.files[hit.index];
        let mut x = 1;
        if app.icons {
            let glyph = icons::for_name(&tf.name).glyph;
            let color = icons::type_color(&tf.name, app.index.files[hit.index].binary);
            c.put(x, ry, Some(glyph), Some(color), bg, Some(false));
            x += 2;
        }
        // The file name, then its directory, with the matched characters in the search color.
        let chars: Vec<char> = tf.path.chars().collect();
        let name_start = chars.len() - tf.name.chars().count();
        let matched = |k: usize| hit.positions.binary_search(&k).is_ok();
        let name_fg = if sel { theme::WHITE } else { theme::INK };
        let room = (w - 1 - x).max(0) as usize;
        let name: Vec<(char, usize)> = (name_start..chars.len()).map(|k| (chars[k], k)).collect();
        x += highlighted(c, x, ry, &name, room, name_fg, bg, &matched);
        let room = (w - 1 - x - 2).max(0) as usize;
        if name_start > 1 && room > 1 {
            let dir: Vec<(char, usize)> = (0..name_start - 1).map(|k| (chars[k], k)).collect();
            let dir_fg = if sel { theme::INK2 } else { theme::DIM };
            let shown = if dir.len() > room {
                // Keep the end of the directory, nearest the file.
                let mut d = vec![('…', usize::MAX)];
                d.extend_from_slice(&dir[dir.len() - (room - 1)..]);
                d
            } else {
                dir
            };
            highlighted(c, x + 2, ry, &shown, room, dir_fg, bg, &matched);
        }
    }
    let prompt = format!("> {}▏", s.query);
    let pw = (w - 2).max(0) as usize;
    let prompt = if prompt.chars().count() > pw {
        fit_left(&prompt, pw)
    } else {
        prompt
    };
    c.text(1, v.rows - 2, &prompt, theme::WHITE, None, true);
}

/// Writes chars, each with its index in the path, fitted to `room`. Matched chars are bold in the search color.
/// Returns the cells written.
#[allow(clippy::too_many_arguments)]
fn highlighted(
    c: &mut Canvas,
    x: i32,
    y: i32,
    chars: &[(char, usize)],
    room: usize,
    fg: Color,
    bg: Option<Color>,
    matched: &dyn Fn(usize) -> bool,
) -> i32 {
    let cut = chars.len() > room;
    let n = if cut {
        room.saturating_sub(1)
    } else {
        chars.len()
    };
    for (k, &(ch, i)) in chars.iter().take(n).enumerate() {
        let hit = matched(i);
        let color = if hit { theme::SEARCH_HI } else { fg };
        c.put(x + k as i32, y, Some(ch), Some(color), bg, Some(hit));
    }
    if cut && room > 0 {
        c.put(x + n as i32, y, Some('…'), Some(fg), bg, Some(false));
    }
    n as i32 + i32::from(cut && room > 0)
}

/// Writes lines down the selection panel (`mkWriter` in the v5 mockup).
struct Column {
    x: i32,
    w: i32,
    y: i32,
    y1: i32,
}

impl Column {
    fn ok(&self) -> bool {
        self.y < self.y1 - 1
    }

    fn room(&self) -> usize {
        self.w.max(0) as usize
    }

    fn line(&mut self, c: &mut Canvas, s: &str, fg: Color, bold: bool) {
        if self.ok() {
            c.text(self.x, self.y, &fit(s, self.room()), fg, None, bold);
        }
        self.y += 1;
    }

    /// Word-wrapped lines, at most `max` of them.
    fn para(&mut self, c: &mut Canvas, s: &str, fg: Color, max: usize) {
        for l in wrap(s, self.room()).iter().take(max) {
            self.line(c, l, fg, false);
        }
    }

    /// A section label, with an optional faint note right-aligned.
    fn label(&mut self, c: &mut Canvas, s: &str, right: &str) {
        if self.ok() {
            c.text(
                self.x,
                self.y,
                &fit(s, self.room()),
                theme::LABEL,
                None,
                true,
            );
            let n = right.chars().count() as i32;
            c.text(
                self.x + self.w - n,
                self.y,
                right,
                theme::FAINT,
                None,
                false,
            );
        }
        self.y += 1;
    }

    fn rule(&mut self, c: &mut Canvas) {
        if self.ok() {
            for k in -1..=self.w {
                c.put(self.x + k, self.y, Some('─'), Some(theme::RULE), None, None);
            }
        }
        self.y += 1;
    }

    fn hint(&self, c: &mut Canvas, s: &str) {
        if s.is_empty() {
            return;
        }
        c.text(
            self.x,
            self.y1 - 2,
            &fit(s, self.room()),
            theme::FAINT,
            None,
            false,
        );
    }
}

/// The right column: the selected file or directory (`drawSide`).
fn draw_side(c: &mut Canvas, app: &App, v: View) {
    let (x0, y0, y1) = (v.cols - v.rw, 1, v.rows - 1);
    c.clip(x0, y0, v.cols, y1);
    c.fill(x0, y0, v.rw, y1 - y0, theme::SIDE);
    for y in y0..y1 {
        c.put(x0, y, Some('│'), Some(theme::RULE), None, None);
    }
    let mut w = Column {
        x: x0 + 2,
        w: v.rw - 4,
        y: y0 + 1,
        y1,
    };
    draw_repo_stats(c, app, &mut w);
    w.rule(c);
    match app.selection() {
        Some(Entry::File(f)) => draw_file_info(c, app, f, &mut w),
        Some(Entry::Dir(n)) => draw_dir_info(c, app, n, &mut w),
        None => w.line(
            c,
            "Select a file in the tree or on the map.",
            theme::FAINT,
            false,
        ),
    }
}

/// The repo name, then `N files · M dirs · K lines` right-aligned on the same row,
/// or on the next row when both do not fit (unit `014`).
fn draw_repo_stats(c: &mut Canvas, app: &App, w: &mut Column) {
    let (root, stats) = (&app.tree.root_name, app.header_stats());
    if w.ok() {
        c.text(w.x, w.y, &fit(root, w.room()), theme::INK, None, true);
    }
    if (root.chars().count() + 1 + stats.chars().count()) as i32 > w.w {
        w.y += 1;
    }
    if w.ok() {
        let s = fit(&stats, w.room());
        let x = w.x + w.w - s.chars().count() as i32;
        c.text(x, w.y, &s, theme::DIM, None, false);
    }
    w.y += 1;
}

/// The rows kept free at the bottom of the panel for the key hint.
const KEYS_ROWS: i32 = 2;
/// The shape's line-number gutter, in cells.
const GUTTER: i32 = 5;

/// Icon and name, directory, namespace, `N lines`, then the shape, the outline, and the git group (`drawSelection`).
fn draw_file_info(c: &mut Canvas, app: &App, f: usize, w: &mut Column) {
    let tf = &app.tree.files[f];
    let file = &app.index.files[f];
    let details = app.file_details(f);
    let x = icon_prefix(c, app, &tf.name, file.binary, w.x, w.y, w.ok());
    if w.ok() {
        let room = (w.x + w.w - x).max(0) as usize;
        c.text(x, w.y, &fit(&tf.name, room), theme::INK, None, true);
    }
    w.y += 1;
    let dir = tf.path.rsplit_once('/').map_or(".", |(d, _)| d);
    w.para(c, &format!("{dir}/"), theme::DIM, 2);
    if let Some(ns) = details.and_then(|d| d.namespace.as_deref()) {
        w.para(c, ns, theme::INK2, 2);
    }
    let mut size = if file.binary {
        "binary".to_string()
    } else {
        format!("{} lines", thousands(file.lines))
    };
    if file.untracked {
        size.push_str(" · untracked");
    }
    w.line(c, &size, theme::DIM, false);
    if let Some((m, _)) = app.trail_mark(f) {
        let age = app.now.saturating_sub(m.at);
        w.para(
            c,
            &crate::trail::describe(m, age),
            theme::trail_color(m.kind),
            2,
        );
    }
    w.y += 1;

    let git_rows = git_rows(app, f);
    let outline: &[OutlineItem] = app.outline(f);
    let browse = app.outline_browse();
    // While the viewer shows this file, the shape marks the lines in view and the outline follows the cursor.
    let viewing = app.viewing(f);
    let current = viewing.and_then(|(_, _, cur)| outline.iter().rposition(|o| o.line <= cur));
    let sel_line = match viewing {
        Some((_, _, cur)) => Some(cur),
        None => browse.and_then(|k| outline.get(k)).map(|o| o.line),
    };
    if let Some(shape) = details.and_then(|d| d.shape.as_ref()) {
        let avail = w.y1 - w.y - git_rows - KEYS_ROWS - 1;
        if shape.lines > 0 && avail > 7 {
            let marks = ShapeMarks {
                sel_line,
                in_view: viewing.map(|(a, b, _)| (a, b)),
                changed: app.changed_lines(f),
            };
            draw_shape(c, w, shape, outline, &marks, avail);
        }
    }
    let outline_rows = w.y1 - w.y - git_rows - KEYS_ROWS - 1;
    if !outline.is_empty() && outline_rows >= 2 {
        let (o, vk, m) = (&app.keymap.outline, &app.keymap.viewer, &app.keymap.map);
        let keys = if browse.is_some() {
            hint(&[
                (o.join(&[K::Down, K::Up], "/"), ""),
                (o.label(K::Open), "view"),
                (o.label(K::Close), ""),
            ])
        } else if viewing.is_some() {
            hint(&[(
                vk.join(&[K::NextDefinition, K::PreviousDefinition], " "),
                "jump",
            )])
        } else {
            hint(&[(m.label(K::BrowseOutline), "to browse")])
        };
        let label = if keys.is_empty() {
            "OUTLINE".to_string()
        } else {
            format!("OUTLINE · {keys}")
        };
        draw_outline(c, app, w, outline, browse.or(current), &label, outline_rows);
    }
    if git_rows > 0 {
        w.y = (w.y + 1).max(w.y1 - git_rows - KEYS_ROWS);
        draw_git(c, app, f, w);
    }
    let keys = if viewing.is_some() {
        let t = &app.keymap.viewer;
        hint(&[
            (t.label(K::OpenEditor), "editor"),
            (t.label(K::CopyLocation), "copy line"),
            (t.label(K::Help), "keys"),
        ])
    } else {
        let t = &app.keymap.map;
        hint(&[
            (t.label(K::Open), "view"),
            (t.label(K::BrowseOutline), "outline"),
            (t.label(K::CopyPath), "path"),
            (t.label(K::Help), "keys"),
        ])
    };
    w.hint(c, &keys);
}

/// The rows the git group takes: its label and summary, then the recent commits. 0 without a history source.
fn git_rows(app: &App, f: usize) -> i32 {
    match &app.git {
        GitHistory::None => 0,
        GitHistory::Loading(_) | GitHistory::Failed(_) => 2,
        GitHistory::Ready(h) => {
            2 + app
                .file_history(f)
                .map_or(0, |fh| h.recent(fh).count() as i32)
        }
    }
}

/// GIT: `N commits`, `changed X ago · created Y ago`, then the recent commits as age and description.
fn draw_git(c: &mut Canvas, app: &App, f: usize, w: &mut Column) {
    match &app.git {
        GitHistory::None => {}
        GitHistory::Loading(_) => {
            w.label(c, "GIT", "loading");
            w.line(c, "Reading the git history…", theme::FAINT, false);
        }
        GitHistory::Failed(e) => {
            w.label(c, "GIT", "");
            w.line(c, &format!("Git history failed: {e}"), theme::FAINT, false);
        }
        GitHistory::Ready(h) => {
            let Some(fh) = app.file_history(f) else {
                w.label(c, "GIT", "0 commits");
                w.line(c, "No commits yet.", theme::FAINT, false);
                return;
            };
            let count = if fh.count == 1 {
                "1 commit".to_string()
            } else {
                format!("{} commits", thousands(u64::from(fh.count)))
            };
            w.label(c, "GIT", &count);
            let summary = format!(
                "changed {} · created {}",
                ago_phrase(h.age_days(fh.newest)),
                ago_phrase(h.age_days(fh.oldest))
            );
            w.line(c, &summary, theme::DIM, false);
            for commit in h.recent(fh) {
                if !w.ok() {
                    break;
                }
                let age = format!("{:>5}", ago(h.age_days(commit.time)));
                c.text(w.x, w.y, &age, theme::INK2, None, false);
                let room = (w.w - 7).max(0) as usize;
                c.text(
                    w.x + 7,
                    w.y,
                    &fit(describe(&commit.subject), room),
                    theme::DIM,
                    None,
                    false,
                );
                w.y += 1;
            }
        }
    }
}

/// What the shape marks, as 1-based lines.
struct ShapeMarks {
    /// The highlighted outline item, or the viewer's cursor.
    sel_line: Option<u32>,
    /// The viewer's first and last lines in view.
    in_view: Option<(u32, u32)>,
    /// Lines with uncommitted changes (unit 030).
    changed: Vec<u32>,
}

/// SHAPE: the whole file as braille, with line numbers at the first, middle, and last rows, and markers at the right:
/// `◀` for the highlighted outline item or the viewer's cursor, `▐` for uncommitted changes, `·` for a definition.
/// While the viewer shows the file, the rows of the lines in view get the viewport fill, as the overview marks the map.
fn draw_shape(
    c: &mut Canvas,
    w: &mut Column,
    shape: &LineShape,
    outline: &[OutlineItem],
    marks: &ShapeMarks,
    avail: i32,
) {
    let rows = ((f64::from(avail) * 0.4).floor() as i32).clamp(4, 12);
    let cols = (w.w - GUTTER - 2).min(40);
    if cols < 4 {
        return;
    }
    w.label(c, "SHAPE", "");
    let b = Braille::new(shape, cols as usize, rows as usize);
    let (mx, my) = (w.x + GUTTER, w.y);
    let sel_row = marks.sel_line.map(|l| b.row_of(l));
    let view_rows = marks.in_view.map(|(a, z)| (b.row_of(a), b.row_of(z)));
    let def_rows: HashSet<usize> = outline.iter().map(|o| b.row_of(o.line)).collect();
    let changed_rows: HashSet<usize> = marks.changed.iter().map(|&l| b.row_of(l)).collect();
    let last = b.rows_used - 1;
    let mid = last / 2;
    for r in 0..rows as usize {
        let y = my + r as i32;
        if y >= w.y1 - 1 {
            break;
        }
        let fg = if sel_row == Some(r) {
            theme::WHITE
        } else {
            theme::MINI_DOT
        };
        let bg = match view_rows {
            Some((a, z)) if r >= a && r <= z => theme::VIEW_IN,
            _ => theme::MINI_BG,
        };
        for cx in 0..cols as usize {
            let ch = b.glyph(cx, r);
            c.put(mx + cx as i32, y, Some(ch), Some(fg), Some(bg), Some(false));
        }
        if r == 0 || r == last || (r == mid && mid > 0 && mid < last) {
            let n = if r == last {
                shape.lines
            } else {
                b.first_line(r)
            };
            let s = n.to_string();
            let x = w.x + GUTTER - 1 - s.chars().count() as i32;
            c.text(x, y, &s, theme::FAINT, None, false);
        }
        let marker = if sel_row == Some(r) {
            Some(('◀', theme::WHITE))
        } else if changed_rows.contains(&r) {
            Some(('▐', theme::DIFF_ADD))
        } else if def_rows.contains(&r) {
            Some(('·', theme::FAINT))
        } else {
            None
        };
        if let Some((ch, fg)) = marker {
            c.put(mx + cols + 1, y, Some(ch), Some(fg), None, Some(true));
        }
    }
    w.y = my + rows + 1;
}

/// OUTLINE: `line  name` rows, private items faint. The highlighted row (the browsed item, or the definition at the
/// viewer's cursor) stays in view.
fn draw_outline(
    c: &mut Canvas,
    app: &App,
    w: &mut Column,
    outline: &[OutlineItem],
    browse: Option<usize>,
    label: &str,
    outline_rows: i32,
) {
    w.label(c, label, &outline.len().to_string());
    let avail = (outline_rows - 1).max(1) as usize;
    // A list longer than the room keeps its last row for `… K more`.
    let shown = if outline.len() > avail {
        (avail - 1).max(1)
    } else {
        avail
    };
    let mut top = app.outline_top.get().min(outline.len().saturating_sub(1));
    if let Some(k) = browse {
        if k < top {
            top = k;
        } else if k >= top + shown {
            top = k + 1 - shown;
        }
    }
    app.outline_top.set(top);
    let end = outline.len().min(top + shown);
    for (k, item) in outline.iter().enumerate().take(end).skip(top) {
        let sel = browse == Some(k);
        let fg = if sel {
            theme::WHITE
        } else if item.private {
            theme::FAINT
        } else {
            theme::DIM
        };
        let bg = sel.then_some(theme::CURSOR);
        if sel && w.ok() {
            c.fill(w.x - 1, w.y, w.w + 2, 1, theme::CURSOR);
        }
        if w.ok() {
            // A nested item's name indents 2 spaces per depth (unit 032).
            let indent = "  ".repeat(usize::from(item.depth));
            let row = fit(
                &format!("{:>4}  {indent}{}", item.line, item.name),
                w.room(),
            );
            c.text(w.x, w.y, &row, fg, bg, false);
        }
        w.y += 1;
    }
    if end < outline.len() {
        w.line(
            c,
            &format!("  … {} more", outline.len() - end),
            theme::FAINT,
            false,
        );
    }
}

/// Draws a file's type icon at `(x, y)` when icons are on, and returns the x where its name starts.
fn icon_prefix(
    c: &mut Canvas,
    app: &App,
    name: &str,
    binary: bool,
    x: i32,
    y: i32,
    draw: bool,
) -> i32 {
    if !app.icons {
        return x;
    }
    if draw {
        let glyph = icons::for_name(name).glyph;
        let color = icons::type_color(name, binary);
        c.put(x, y, Some(glyph), Some(color), None, Some(false));
    }
    x + 2
}

/// Path, counts, lines, then the largest files (`drawDirInfo`).
fn draw_dir_info(c: &mut Canvas, app: &App, n: usize, w: &mut Column) {
    let node = &app.tree.nodes[n];
    w.line(c, &format!("{}/", node.path), theme::INK, true);
    let count = app.filetree.count(n, app.show_hidden);
    w.line(
        c,
        &format!(
            "{} files · {} subdirectories",
            thousands(count as u64),
            node.kids.len()
        ),
        theme::DIM,
        false,
    );
    // The files under the directory, in tree order: each node's files, then its children.
    let mut files = Vec::new();
    let mut stack = vec![n];
    while let Some(m) = stack.pop() {
        let nm = &app.tree.nodes[m];
        files.extend(nm.files.iter().copied().filter(|&f| app.file_visible(f)));
        stack.extend(nm.kids.iter().rev());
    }
    let lines: u64 = files.iter().map(|&f| app.index.files[f].lines).sum();
    w.line(c, &format!("{} lines", kilo(lines)), theme::DIM, false);
    w.y += 1;
    w.label(c, "LARGEST FILES", "lines");
    let mut text: Vec<usize> = files
        .into_iter()
        .filter(|&f| !app.index.files[f].binary)
        .collect();
    text.sort_by_key(|&f| std::cmp::Reverse(app.index.files[f].lines));
    let skip = node.path.len() + 1;
    for f in text {
        if w.y >= w.y1 - 2 {
            break;
        }
        let tf = &app.tree.files[f];
        let x = icon_prefix(c, app, &tf.name, false, w.x, w.y, true);
        let room = (w.x + w.w - 8 - x).max(0) as usize;
        c.text(
            x,
            w.y,
            &fit(&tf.path[skip..], room),
            theme::INK2,
            None,
            false,
        );
        let lines = app.index.files[f].lines.to_string();
        let x = w.x + w.w - lines.len() as i32;
        c.text(x, w.y, &lines, theme::DIM, None, false);
        w.y += 1;
    }
    let t = &app.keymap.map;
    let keys = hint(&[
        (t.label(K::TreeIn), "open"),
        (t.label(K::TreeOut), "close"),
        (t.join(&[K::Down, K::Up], "/"), "move"),
    ]);
    w.hint(c, &keys);
}

/// The breadcrumb of the selection on the left, with its last part bright.
/// On the right: the flash message, else the hovered path (`drawTop`).
fn draw_top(c: &mut Canvas, app: &App, v: View) {
    c.clip(0, 0, v.cols, v.rows);
    c.fill(0, 0, v.cols, 1, theme::BAR);
    let (right, rcol) = match (app.flash_text(), app.hover) {
        (Some(m), _) => (m.to_string(), theme::INK),
        (None, Some(Entry::File(f))) => (app.tree.files[f].path.clone(), theme::INK2),
        (None, Some(Entry::Dir(n))) => (app.tree.dir_label(n), theme::INK2),
        (None, None) => (String::new(), theme::DIM),
    };
    let right = fit(&right, (v.cols - 24).max(0) as usize);
    let rn = right.chars().count() as i32;
    let avail = (v.cols - rn - 3).max(0) as usize;

    let bc = match app.selection() {
        Some(Entry::File(f)) => app.tree.file_crumb(f),
        Some(Entry::Dir(n)) => app.tree.dir_crumb(n),
        None => app.tree.root_crumb(),
    };
    let crumb = format!(" {}", bc.text());
    let mut chars: Vec<char> = crumb.chars().collect();
    if chars.len() > avail {
        let cut = (chars.len() + 2).saturating_sub(avail).min(chars.len());
        chars = [' ', '…']
            .into_iter()
            .chain(chars[cut..].iter().copied())
            .collect();
    }
    chars.truncate(avail);
    let crumb: String = chars.iter().collect();
    c.text(0, 0, &crumb, theme::DIM, None, false);
    let bright: Vec<char> = bc.bright().chars().collect();
    if let Some(li) = rfind_chars(&chars, &bright) {
        let s: String = bright.iter().take(avail.saturating_sub(li)).collect();
        c.text(li as i32, 0, &s, theme::INK, None, true);
    }
    c.text(v.cols - rn - 1, 0, &right, rcol, None, false);
}

fn rfind_chars(hay: &[char], needle: &[char]) -> Option<usize> {
    if needle.is_empty() || needle.len() > hay.len() {
        return None;
    }
    (0..=hay.len() - needle.len())
        .rev()
        .find(|&i| hay[i..i + needle.len()] == *needle)
}

/// Toggles on the left, as the mockup's layer toggles, and the key hint on the right (`drawStatus`).
fn draw_status(c: &mut Canvas, app: &App, v: View) {
    let y = v.rows - 1;
    c.clip(0, 0, v.cols, v.rows);
    c.fill(0, y, v.cols, 1, theme::BAR);
    if app.picker_open() {
        let x = c.text(0, y, " SEARCH ", theme::WHITE, Some(theme::SEARCH_BG), true);
        let hint = " ↑↓ ^p ^n move · ⏎ keep · esc back ";
        c.text(x, y, hint, theme::DIM, None, false);
        return;
    }
    if let Some(vw) = &app.viewer {
        draw_viewer_status(c, vw, y);
        return;
    }
    let t = &app.keymap.map;
    if app.outline_browse().is_some() {
        let x = c.text(0, y, " OUTLINE ", theme::BAR, Some(theme::FOCUS), true);
        let o = &app.keymap.outline;
        let keys = hint(&[
            (o.join(&[K::Down, K::Up], "/"), "move"),
            (o.label(K::Open), "view at line"),
            (o.label(K::OpenEditor), "edit"),
            (o.label(K::Close), "back"),
        ]);
        c.text(x, y, &format!(" {keys} "), theme::DIM, None, false);
        return;
    }
    let mut x = 0;
    if let Some(s) = &app.search {
        // A kept search (after Enter): the query and the count explain the marks on the map (unit `024`).
        x += c.text(
            x,
            y,
            &format!(" /{} ", s.query),
            theme::WHITE,
            Some(theme::SEARCH_BG),
            true,
        );
        let keys = hint(&[
            (t.join(&[K::NextResult, K::PreviousResult], "/"), ""),
            (t.label(K::ClearSearch), "clear"),
        ]);
        let kept = format!(
            " {} of {}{}{keys} ",
            thousands(s.kept.len() as u64),
            thousands(s.kept_of as u64),
            if keys.is_empty() { "" } else { " · " },
        );
        x += c.text(x, y, &kept, theme::DIM, None, false);
    }
    let toggle = |c: &mut Canvas, x: i32, cmd: K, label: &str, on: bool, on_color: Color| {
        let (mark, color) = if on {
            ('●', on_color)
        } else {
            ('○', theme::FAINT)
        };
        let key = t.label(cmd);
        let text = if key.is_empty() {
            format!(" {mark} {label} ")
        } else {
            format!(" {key} {mark} {label} ")
        };
        c.text(x, y, &text, color, None, false)
    };
    // Layer toggles first, then the active heat ramp's legend (`docs/design.md` `Layers`).
    x += toggle(c, x, K::ToggleTrail, "trail", app.trail_on, theme::READ);
    x += toggle(
        c,
        x,
        K::ToggleAgeHeat,
        "age",
        app.heat == Some(Heat::Age),
        theme::HEAT,
    );
    x += toggle(
        c,
        x,
        K::ToggleChurnHeat,
        "churn",
        app.heat == Some(Heat::Churn),
        theme::HEAT,
    );
    if let Some(heat) = app.heat {
        for (step, label) in heat.labels().into_iter().enumerate() {
            c.fill(x + 1, y, 2, 1, theme::heat_bg(step));
            x += 3 + c.text(x + 3, y, label, theme::DIM, None, false);
        }
        x += 1;
    }
    let toggles = [
        (K::ToggleHidden, "hidden", app.show_hidden),
        (K::TogglePulse, "pulse", app.pulse),
        (K::TogglePanel, "panel", app.view().rw > 0),
        (K::ToggleOverview, "overview", app.overview_on),
    ];
    for (cmd, label, on) in toggles {
        x += toggle(c, x, cmd, label, on, theme::INK);
    }
    let keys = hint(&[(t.label(K::Help), "keys"), (t.label(K::Quit), "quit")]);
    let n = keys.chars().count() as i32;
    if n > 0 && x + n + 2 < v.cols {
        c.text(v.cols - n - 1, y, &keys, theme::DIM, None, false);
    }
}

/// The viewer's status: ` VIEW ` and `line / total` with the keys, or the find prompt and its match count.
fn draw_viewer_status(c: &mut Canvas, vw: &Viewer, y: i32) {
    if let Some(q) = &vw.prompt {
        let mut x = c.text(0, y, " FIND ", theme::WHITE, Some(theme::SEARCH_BG), true);
        x += c.text(x, y, &format!(" /{q}▏"), theme::WHITE, None, true);
        let n = if q.is_empty() { 0 } else { vw.match_count(q) };
        let count = format!("  {} matches · ⏎ find · esc cancel ", thousands(n as u64));
        c.text(x, y, &count, theme::DIM, None, false);
        return;
    }
    let badge = if vw.diff_mode { " DIFF " } else { " VIEW " };
    let mut x = c.text(0, y, badge, theme::BAR, Some(theme::SEL_HI), true);
    let pos = format!(
        " {} / {} ",
        thousands(vw.cursor_line() as u64 + 1),
        thousands(vw.lines.len() as u64)
    );
    x += c.text(x, y, &pos, theme::INK, None, false);
    if vw.plain {
        x += c.text(x, y, "plain ", theme::FAINT, None, false);
    }
    let changes = vw.diff.as_deref().filter(|d| !d.is_empty());
    if let Some(d) = changes {
        x += c.text(x, y, &format!("+{}", d.added), theme::DIFF_ADD, None, true);
        x += c.text(
            x,
            y,
            &format!(" −{} ", d.removed),
            theme::DIFF_REMOVE,
            None,
            true,
        );
    }
    let t = &vw.keys.viewer;
    let tail = [
        (t.label(K::CopyLocation), "copy"),
        (t.label(K::OpenEditor), "edit"),
        (t.label(K::Close), "map"),
    ];
    let find = [
        (t.label(K::Find), "find"),
        (t.label(K::NextDefinition), "def"),
    ];
    let pieces: Vec<(String, &str)> = match (vw.diff_mode, changes.is_some()) {
        (true, _) => [
            (t.join(&[K::NextChange, K::PreviousChange], " "), "hunk"),
            (t.label(K::ToggleDiff), "file"),
            (t.join(&[K::Down, K::Up], "/"), ""),
        ]
        .into_iter()
        .chain(tail)
        .collect(),
        (false, true) => [
            (t.label(K::ToggleDiff), "diff"),
            (t.label(K::NextChange), "hunk"),
        ]
        .into_iter()
        .chain(find)
        .chain(tail)
        .collect(),
        (false, false) => {
            let moves = [
                t.join(&[K::Down, K::Up], "/"),
                t.join(&[K::HalfPageDown, K::HalfPageUp], "/"),
            ]
            .into_iter()
            .filter(|k| !k.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
            [(moves, "")].into_iter().chain(find).chain(tail).collect()
        }
    };
    let keys = hint(&pieces);
    // After the change counts the hint starts with a separator, as the counts end the position part.
    let text = if changes.is_some() {
        format!("· {keys} ")
    } else {
        format!(" {keys} ")
    };
    c.text(x, y, &text, theme::DIM, None, false);
}

/// "a, b, or c": a list of phrases with commas and "or".
fn or_list(items: &[&str]) -> String {
    match items {
        [] => String::new(),
        [one] => one.to_string(),
        [a, b] => format!("{a} or {b}"),
        [rest @ .., last] => format!("{}, or {last}", rest.join(", ")),
    }
}

/// The help rows, from the keymap (unit `033`): the key column, then what the keys do.
/// A row whose action has no key keeps its words with an empty key column.
fn help_rows(app: &App) -> Vec<(String, String)> {
    let (m, o, vk) = (&app.keymap.map, &app.keymap.outline, &app.keymap.viewer);
    let row = |k: String, d: &str| (k, d.to_string());
    let first_long = |t: &Table, c: K| {
        t.keys(c)
            .next()
            .map_or_else(String::new, |s| crate::keymap::seq_label(s, true))
    };
    let mut rows = vec![
        row(m.join(&[K::Down, K::Up], " / "), "move in the file tree"),
        row(
            m.join(&[K::FirstRow, K::LastRow], " / "),
            "first or last row of the file tree",
        ),
        row(
            m.join(&[K::TreeIn, K::TreeOut], " / "),
            "open or close a directory, or go in or up",
        ),
        row(
            first_long(m, K::Open),
            "view the file; on a directory, open or close it",
        ),
    ];
    let select = [K::SelectLeft, K::SelectRight, K::SelectUp, K::SelectDown];
    let arrows = ["←", "→", "↑", "↓"];
    let select_keys = if select.iter().zip(arrows).all(|(&c, a)| m.label(c) == a) {
        "arrows".to_string()
    } else {
        m.join(&select, " ")
    };
    rows.push(row(select_keys, "move across the map to the nearest file"));
    let pan = [K::PanUp, K::PanLeft, K::PanDown, K::PanRight];
    let far = [K::PanUpFar, K::PanLeftFar, K::PanDownFar, K::PanRightFar];
    let single = pan.iter().all(|&c| m.label(c).chars().count() == 1);
    let pan_keys = m.join(&pan, if single { "" } else { " " });
    let shifted = pan
        .iter()
        .zip(far)
        .all(|(&p, f)| !m.label(p).is_empty() && m.label(f) == m.label(p).to_uppercase());
    if shifted {
        rows.push(row(pan_keys, "pan the map (shift: faster)"));
    } else {
        rows.push(row(pan_keys, "pan the map"));
        let single = far.iter().all(|&c| m.label(c).chars().count() == 1);
        rows.push(row(
            m.join(&far, if single { "" } else { " " }),
            "pan the map 4 times as far",
        ));
    }
    let kept = m.join(&[K::NextResult, K::PreviousResult], ", ");
    let search = if kept.is_empty() {
        "search files, fzf style".to_string()
    } else {
        format!("search files, fzf style ({kept}: kept results)")
    };
    let browse = [
        (o.join(&[K::Down, K::Up], "/"), "move"),
        (o.label(K::Close), "back"),
    ]
    .into_iter()
    .filter(|(k, _)| !k.is_empty())
    .map(|(k, w)| format!("{k} {w}"))
    .collect::<Vec<_>>()
    .join(", ");
    let browse = if browse.is_empty() {
        "browse the outline".to_string()
    } else {
        format!("browse the outline ({browse})")
    };
    rows.extend([
        row(m.label(K::ToggleTree), "show or hide the file tree"),
        row(
            m.join(&[K::NarrowTree, K::WidenTree], " / "),
            "narrow or widen the file tree",
        ),
        row(m.label(K::TogglePanel), "show or hide the selection panel"),
        row(m.label(K::ToggleOverview), "show or hide the overview"),
        row(m.label(K::Search), &search),
        row(m.label(K::BrowseOutline), &browse),
        row(
            m.label(K::OpenEditor),
            "open the file in your editor at the line",
        ),
        row(
            m.join(&[K::ToggleTrail, K::ClearTrail], " / "),
            "the agent's trail on the map, or clear it",
        ),
        row(
            m.join(&[K::ToggleAgeHeat, K::ToggleChurnHeat], " / "),
            "age or churn heat on the map",
        ),
        row(m.label(K::CopyPath), "copy the path"),
        row(m.label(K::CopyNamespace), "copy the namespace"),
        row(m.label(K::CopyCommit), "copy the newest commit hash"),
        row(m.label(K::ToggleHidden), "show or hide hidden files"),
        row(m.label(K::TogglePulse), "selection pulse on or off"),
        row(m.label(K::Help), "show or hide this help"),
    ]);
    // Esc always closes the help and the picker. The viewer and the outline close with it while their tables bind it.
    let esc_closes = |t: &Table| t.keys(K::Close).any(|s| s == [Key::ESC]);
    let mut closes = vec!["this help"];
    if esc_closes(vk) {
        closes.push("the viewer");
    }
    if esc_closes(o) {
        closes.push("the outline");
    }
    closes.push("the search");
    rows.push(row(
        "esc".to_string(),
        &format!("close {}", or_list(&closes)),
    ));
    let quit = [m.label(K::Quit), "ctrl+c".to_string()]
        .into_iter()
        .filter(|k| !k.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    rows.push(row(quit, "quit"));
    rows.push(row(String::new(), ""));
    let last = vk.label(K::LastLine);
    let motions = [
        vk.join(&[K::Down, K::Up], "/"),
        vk.join(&[K::HalfPageDown, K::HalfPageUp], "/"),
        vk.join(&[K::PageDown, K::PageUp], "/"),
        vk.label(K::FirstLine),
        last.clone(),
        if last.is_empty() {
            String::new()
        } else {
            format!("12{last}")
        },
        vk.join(&[K::CenterCursor, K::CursorToTop, K::CursorToBottom], "/"),
        vk.join(&[K::PreviousBlank, K::NextBlank], " "),
        vk.join(&[K::ScrollLeft, K::ScrollRight], "/"),
        vk.label(K::ScrollHome),
    ]
    .into_iter()
    .filter(|k| !k.is_empty())
    .collect::<Vec<_>>()
    .join(" ");
    rows.push(row("viewer".to_string(), &motions));
    let matches = vk.join(&[K::NextMatch, K::PreviousMatch], "/");
    let find = match (vk.label(K::Find), matches.is_empty()) {
        (f, _) if f.is_empty() => String::new(),
        (f, true) => f,
        (f, false) => format!("{f} find, {matches}"),
    };
    let find_words = if matches.is_empty() { "find" } else { "" };
    rows.push(row(
        String::new(),
        &hint(&[
            (
                vk.join(&[K::NextDefinition, K::PreviousDefinition], " "),
                "definitions",
            ),
            (find, find_words),
            (vk.label(K::CopyLocation), "copy path:line"),
        ]),
    ));
    rows.push(row(
        String::new(),
        &hint(&[
            (vk.label(K::ToggleDiff), "uncommitted diff"),
            (
                vk.join(&[K::NextChange, K::PreviousChange], " "),
                "next or previous change",
            ),
        ]),
    ));
    rows.push(row(
        String::new(),
        &hint(&[
            (vk.label(K::ToggleTree), "tree"),
            (vk.label(K::TogglePanel), "panel"),
            (vk.labels(K::Close, " or ", false), "back to the map"),
        ]),
    ));
    rows.push(row(String::new(), ""));
    for (k, d) in [
        ("mouse", "click a file or a directory name to select it"),
        ("", "drag the map to pan it"),
        ("", "click or drag the overview to move the view"),
        ("", "drag the tree edge to resize the tree"),
        ("", "the wheel scrolls the tree"),
        ("", ""),
        ("keymap", "minimap keys prints the keymap"),
    ] {
        rows.push(row(k.to_string(), d));
    }
    rows
}

fn draw_help(c: &mut Canvas, app: &App, v: View) {
    c.clip(0, 0, v.cols, v.rows);
    let rows = help_rows(app);
    let w = 66.min(v.cols - 4);
    let h = (rows.len() as i32 + 4).min(v.rows - 2);
    let (x, y) = ((v.cols - w) / 2, ((v.rows - h) / 2).max(1));
    if w < 20 || h < 5 {
        return;
    }
    c.frame(x, y, w, h, theme::INK2, theme::POP);
    c.text(x + 2, y + 1, "KEYS", theme::FOCUS, Some(theme::POP), true);
    let help = app.keymap.map.label(K::Help);
    let close = if help.is_empty() {
        "esc close".to_string()
    } else {
        format!("esc or {help} close")
    };
    c.text(
        x + w - 2 - close.chars().count() as i32,
        y + 1,
        &close,
        theme::DIM,
        Some(theme::POP),
        false,
    );
    for (j, (k, d)) in rows.iter().take((h - 4) as usize).enumerate() {
        let row = y + 3 + j as i32;
        c.text(x + 2, row, &fit(k, 14), theme::INK, Some(theme::POP), true);
        c.text(
            x + 17,
            row,
            &fit(d, (w - 19) as usize),
            theme::DIM,
            Some(theme::POP),
            false,
        );
    }
}
