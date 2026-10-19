//! The keymap (unit `033`): key strings, the default bindings, the config merge, and the lookup.
//!
//! The config binds keys per mode, in VS Code-style key strings:
//!
//! ```toml
//! [keys.map]
//! "h" = "pan_left"
//! "a" = "none"           # removes the default binding of a
//!
//! [keys.viewer]
//! "ctrl+d" = "half_page_down"
//! "g g" = "first_line"   # a space separates the keys of a sequence
//! ```
//!
//! A key the config names takes that action. A key the config does not name keeps its default.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, LazyLock};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// A key table: the map (with the file tree), outline browse, or the viewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    Map,
    Outline,
    Viewer,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::Map, Mode::Outline, Mode::Viewer];

    /// The config table name: `[keys.<name>]`.
    pub fn name(self) -> &'static str {
        match self {
            Mode::Map => "map",
            Mode::Outline => "outline",
            Mode::Viewer => "viewer",
        }
    }

    fn defs(self) -> &'static [Def] {
        match self {
            Mode::Map => MAP,
            Mode::Outline => OUTLINE,
            Mode::Viewer => VIEWER,
        }
    }
}

/// An action that a key runs. One name means the same kind of action in each mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    Quit,
    Help,
    Search,
    NextResult,
    PreviousResult,
    ClearSearch,
    ToggleHidden,
    ToggleTree,
    NarrowTree,
    WidenTree,
    CopyPath,
    CopyNamespace,
    CopyCommit,
    ToggleTrail,
    ClearTrail,
    ToggleAgeHeat,
    ToggleChurnHeat,
    BrowseOutline,
    OpenEditor,
    TogglePulse,
    TogglePanel,
    ToggleOverview,
    FirstRow,
    LastRow,
    Down,
    Up,
    PageDown,
    PageUp,
    TreeIn,
    TreeOut,
    Open,
    SelectLeft,
    SelectRight,
    SelectUp,
    SelectDown,
    PanUp,
    PanLeft,
    PanDown,
    PanRight,
    PanUpFar,
    PanLeftFar,
    PanDownFar,
    PanRightFar,
    Close,
    HalfPageDown,
    HalfPageUp,
    FirstLine,
    LastLine,
    CenterCursor,
    CursorToTop,
    CursorToBottom,
    NextDefinition,
    PreviousDefinition,
    NextChange,
    PreviousChange,
    NextBlank,
    PreviousBlank,
    ScrollLeft,
    ScrollRight,
    ScrollHome,
    Find,
    NextMatch,
    PreviousMatch,
    ToggleDiff,
    CopyLocation,
}

impl Command {
    /// The config name.
    pub fn name(self) -> &'static str {
        use Command::*;
        match self {
            Quit => "quit",
            Help => "help",
            Search => "search",
            NextResult => "next_result",
            PreviousResult => "previous_result",
            ClearSearch => "clear_search",
            ToggleHidden => "toggle_hidden",
            ToggleTree => "toggle_tree",
            NarrowTree => "narrow_tree",
            WidenTree => "widen_tree",
            CopyPath => "copy_path",
            CopyNamespace => "copy_namespace",
            CopyCommit => "copy_commit",
            ToggleTrail => "toggle_trail",
            ClearTrail => "clear_trail",
            ToggleAgeHeat => "toggle_age_heat",
            ToggleChurnHeat => "toggle_churn_heat",
            BrowseOutline => "browse_outline",
            OpenEditor => "open_editor",
            TogglePulse => "toggle_pulse",
            TogglePanel => "toggle_panel",
            ToggleOverview => "toggle_overview",
            FirstRow => "first_row",
            LastRow => "last_row",
            Down => "down",
            Up => "up",
            PageDown => "page_down",
            PageUp => "page_up",
            TreeIn => "tree_in",
            TreeOut => "tree_out",
            Open => "open",
            SelectLeft => "select_left",
            SelectRight => "select_right",
            SelectUp => "select_up",
            SelectDown => "select_down",
            PanUp => "pan_up",
            PanLeft => "pan_left",
            PanDown => "pan_down",
            PanRight => "pan_right",
            PanUpFar => "pan_up_far",
            PanLeftFar => "pan_left_far",
            PanDownFar => "pan_down_far",
            PanRightFar => "pan_right_far",
            Close => "close",
            HalfPageDown => "half_page_down",
            HalfPageUp => "half_page_up",
            FirstLine => "first_line",
            LastLine => "last_line",
            CenterCursor => "center_cursor",
            CursorToTop => "cursor_to_top",
            CursorToBottom => "cursor_to_bottom",
            NextDefinition => "next_definition",
            PreviousDefinition => "previous_definition",
            NextChange => "next_change",
            PreviousChange => "previous_change",
            NextBlank => "next_blank",
            PreviousBlank => "previous_blank",
            ScrollLeft => "scroll_left",
            ScrollRight => "scroll_right",
            ScrollHome => "scroll_home",
            Find => "find",
            NextMatch => "next_match",
            PreviousMatch => "previous_match",
            ToggleDiff => "toggle_diff",
            CopyLocation => "copy_location",
        }
    }
}

/// One action of a mode: its default keys and what it does.
struct Def(Command, &'static [&'static str], &'static str);

use Command as C;

const MAP: &[Def] = &[
    Def(C::Quit, &["q"], "quit minimap"),
    Def(C::Help, &["?"], "show or hide the help"),
    Def(C::Search, &["/"], "search files in the picker"),
    Def(C::NextResult, &["n"], "the next kept search result"),
    Def(C::PreviousResult, &["N"], "the previous kept search result"),
    Def(C::ClearSearch, &["esc"], "clear the kept search"),
    Def(C::ToggleHidden, &["."], "show or hide hidden files"),
    Def(C::ToggleTree, &["t"], "show or hide the file tree"),
    Def(C::NarrowTree, &["["], "narrow the file tree"),
    Def(C::WidenTree, &["]"], "widen the file tree"),
    Def(C::CopyPath, &["y"], "copy the selected path"),
    Def(
        C::CopyNamespace,
        &["Y"],
        "copy the selected file's namespace",
    ),
    Def(
        C::CopyCommit,
        &["H"],
        "copy the selected file's newest commit hash",
    ),
    Def(C::ToggleTrail, &["1"], "the agent's trail on the map"),
    Def(C::ClearTrail, &["x"], "clear the agent's trail"),
    Def(C::ToggleAgeHeat, &["4"], "age heat on the map"),
    Def(C::ToggleChurnHeat, &["5"], "churn heat on the map"),
    Def(C::BrowseOutline, &["o"], "browse the outline"),
    Def(C::OpenEditor, &["e"], "open the file in the editor"),
    Def(C::TogglePulse, &["m"], "selection pulse on or off"),
    Def(C::TogglePanel, &["p"], "show or hide the selection panel"),
    Def(C::ToggleOverview, &["v"], "show or hide the overview"),
    Def(C::FirstRow, &["g", "home"], "the first file tree row"),
    Def(C::LastRow, &["G", "end"], "the last file tree row"),
    Def(C::Down, &["j"], "the next file tree row"),
    Def(C::Up, &["k"], "the previous file tree row"),
    Def(C::PageDown, &["pagedown"], "a page down the file tree"),
    Def(C::PageUp, &["pageup"], "a page up the file tree"),
    Def(
        C::TreeIn,
        &["l"],
        "open a directory, or go into an open one",
    ),
    Def(C::TreeOut, &["h"], "close a directory, or go up"),
    Def(
        C::Open,
        &["enter"],
        "view the file; open or close a directory",
    ),
    Def(
        C::SelectLeft,
        &["left"],
        "the nearest file to the left on the map",
    ),
    Def(
        C::SelectRight,
        &["right"],
        "the nearest file to the right on the map",
    ),
    Def(C::SelectUp, &["up"], "the nearest file above on the map"),
    Def(
        C::SelectDown,
        &["down"],
        "the nearest file below on the map",
    ),
    Def(C::PanUp, &["w"], "pan the map up"),
    Def(C::PanLeft, &["a"], "pan the map left"),
    Def(C::PanDown, &["s"], "pan the map down"),
    Def(C::PanRight, &["d"], "pan the map right"),
    Def(C::PanUpFar, &["W"], "pan the map up, 4 times as far"),
    Def(C::PanLeftFar, &["A"], "pan the map left, 4 times as far"),
    Def(C::PanDownFar, &["S"], "pan the map down, 4 times as far"),
    Def(C::PanRightFar, &["D"], "pan the map right, 4 times as far"),
];

const OUTLINE: &[Def] = &[
    Def(C::Down, &["j", "down"], "the next outline item"),
    Def(C::Up, &["k", "up"], "the previous outline item"),
    Def(C::Open, &["enter"], "view the file at the item"),
    Def(
        C::OpenEditor,
        &["e"],
        "open the file in the editor at the item",
    ),
    Def(C::Close, &["esc", "o"], "stop browsing the outline"),
    Def(C::Quit, &["q"], "quit minimap"),
];

const VIEWER: &[Def] = &[
    Def(C::Down, &["j", "down"], "the next line"),
    Def(C::Up, &["k", "up"], "the previous line"),
    Def(C::HalfPageDown, &["ctrl+d"], "half a page down"),
    Def(C::HalfPageUp, &["ctrl+u"], "half a page up"),
    Def(C::PageDown, &["ctrl+f", "pagedown"], "a page down"),
    Def(C::PageUp, &["ctrl+b", "pageup"], "a page up"),
    Def(
        C::FirstLine,
        &["g g", "home"],
        "the first line; with a count, that line",
    ),
    Def(
        C::LastLine,
        &["G", "end"],
        "the last line; with a count, that line",
    ),
    Def(
        C::CenterCursor,
        &["z z"],
        "the cursor line to the middle of the view",
    ),
    Def(
        C::CursorToTop,
        &["z t"],
        "the cursor line to the top of the view",
    ),
    Def(
        C::CursorToBottom,
        &["z b"],
        "the cursor line to the bottom of the view",
    ),
    Def(C::NextDefinition, &["] ]"], "the next outline item"),
    Def(C::PreviousDefinition, &["[ ["], "the previous outline item"),
    Def(C::NextChange, &["] c"], "the next uncommitted change"),
    Def(
        C::PreviousChange,
        &["[ c"],
        "the previous uncommitted change",
    ),
    Def(C::NextBlank, &["}"], "the next blank line"),
    Def(C::PreviousBlank, &["{"], "the previous blank line"),
    Def(C::ScrollLeft, &["h", "left"], "scroll 8 columns left"),
    Def(C::ScrollRight, &["l", "right"], "scroll 8 columns right"),
    Def(C::ScrollHome, &["0"], "scroll back to column 0"),
    Def(C::Find, &["/"], "find in the file"),
    Def(C::NextMatch, &["n"], "the next match"),
    Def(C::PreviousMatch, &["N"], "the previous match"),
    Def(
        C::ToggleDiff,
        &["="],
        "switch between the file and its uncommitted diff",
    ),
    Def(C::CopyLocation, &["y"], "copy path:line of the cursor line"),
    Def(
        C::OpenEditor,
        &["e"],
        "open the file in the editor at the cursor line",
    ),
    Def(C::ToggleTree, &["t"], "show or hide the file tree"),
    Def(C::TogglePanel, &["p"], "show or hide the selection panel"),
    Def(C::Help, &["?"], "show or hide the help"),
    Def(C::Close, &["esc", "q"], "back to the map"),
];

/// One key with its modifiers, normalized so a key event and a parsed key string compare equal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
    pub code: KeyCode,
    pub mods: KeyModifiers,
}

/// The modifiers that count. Others (Super, Hyper) do not reach minimap without the kitty protocol.
const MODS: KeyModifiers = KeyModifiers::CONTROL
    .union(KeyModifiers::ALT)
    .union(KeyModifiers::SHIFT);

impl Key {
    pub const ESC: Key = Key {
        code: KeyCode::Esc,
        mods: KeyModifiers::NONE,
    };

    /// A key from its code and modifiers.
    /// On a character, the Shift flag does not count, because the character is already shifted.
    /// With Ctrl, a letter's case does not count, because terminals send the same bytes for both.
    pub fn new(code: KeyCode, mods: KeyModifiers) -> Key {
        let mut mods = mods & MODS;
        let code = match code {
            KeyCode::Char(c) => {
                mods.remove(KeyModifiers::SHIFT);
                if mods.contains(KeyModifiers::CONTROL) {
                    KeyCode::Char(c.to_ascii_lowercase())
                } else {
                    KeyCode::Char(c)
                }
            }
            KeyCode::BackTab => {
                mods.remove(KeyModifiers::SHIFT);
                KeyCode::BackTab
            }
            c => c,
        };
        Key { code, mods }
    }

    pub fn from_event(ev: &KeyEvent) -> Key {
        Key::new(ev.code, ev.modifiers)
    }

    /// Parses one key: a character, or a named key, with modifiers joined by `+` (`ctrl+d`, `shift+tab`).
    pub fn parse(s: &str) -> Result<Key, String> {
        let mut chars = s.chars();
        match (chars.next(), chars.next()) {
            (None, _) => return Err("an empty key".to_string()),
            (Some(c), None) => return Ok(Key::new(KeyCode::Char(c), KeyModifiers::NONE)),
            _ => {}
        }
        let parts: Vec<&str> = s.split('+').collect();
        let (name, mod_names) = parts.split_last().expect("split gives one part");
        if name.is_empty() {
            return Err(format!(
                "`{s}` has no key after `+` (write `plus` for the + key)"
            ));
        }
        let mut mods = KeyModifiers::NONE;
        for m in mod_names {
            mods |= match m.to_ascii_lowercase().as_str() {
                "ctrl" => KeyModifiers::CONTROL,
                "shift" => KeyModifiers::SHIFT,
                "alt" | "option" | "opt" | "meta" => KeyModifiers::ALT,
                _ => {
                    return Err(format!("`{m}` is not a modifier (use ctrl, shift, or alt)"));
                }
            };
        }
        let mut code = named(name).ok_or_else(|| format!("`{name}` is not a key name"))?;
        if mods.contains(KeyModifiers::SHIFT) {
            match code {
                KeyCode::Char(c) if c.is_alphabetic() => {
                    let mut up = c.to_uppercase();
                    code = KeyCode::Char(up.next().unwrap_or(c));
                    mods.remove(KeyModifiers::SHIFT);
                }
                KeyCode::Char(_) => {
                    return Err(format!(
                        "`{s}`: write the shifted character itself, not shift with `{name}`"
                    ));
                }
                KeyCode::Tab => {
                    code = KeyCode::BackTab;
                    mods.remove(KeyModifiers::SHIFT);
                }
                _ => {}
            }
        }
        Ok(Key::new(code, mods))
    }

    /// The key as the config writes it: `ctrl+d`, `shift+tab`, `space`, `G`.
    pub fn config(&self) -> String {
        let mut s = String::new();
        if self.mods.contains(KeyModifiers::CONTROL) {
            s.push_str("ctrl+");
        }
        if self.mods.contains(KeyModifiers::ALT) {
            s.push_str("alt+");
        }
        if self.mods.contains(KeyModifiers::SHIFT) {
            s.push_str("shift+");
        }
        s.push_str(&match self.code {
            KeyCode::Char(' ') => "space".to_string(),
            KeyCode::Char('+') => "plus".to_string(),
            KeyCode::Char(c) => c.to_string(),
            KeyCode::BackTab => "shift+tab".to_string(),
            KeyCode::F(n) => format!("f{n}"),
            code => name_of(code).to_string(),
        });
        s
    }

    /// The key as hints show it: `^d`, `⏎`, `←`, `esc`, `G`. `long` shows Enter as `enter`, as the help does.
    pub fn label(&self, long: bool) -> String {
        let name = match self.code {
            KeyCode::Char(' ') => "space".to_string(),
            KeyCode::Char(c) => c.to_string(),
            KeyCode::Enter if !long => "⏎".to_string(),
            KeyCode::Left => "←".to_string(),
            KeyCode::Right => "→".to_string(),
            KeyCode::Up => "↑".to_string(),
            KeyCode::Down => "↓".to_string(),
            KeyCode::BackTab => "shift+tab".to_string(),
            KeyCode::F(n) => format!("f{n}"),
            code => name_of(code).to_string(),
        };
        let mut s = String::new();
        if self.mods.contains(KeyModifiers::ALT) {
            s.push_str("alt+");
        }
        if self.mods.contains(KeyModifiers::SHIFT) {
            s.push_str("shift+");
        }
        if self.mods.contains(KeyModifiers::CONTROL) {
            s.push('^');
        }
        s + &name
    }
}

/// A named key, or a single character, in any case for names.
fn named(name: &str) -> Option<KeyCode> {
    let mut chars = name.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return Some(KeyCode::Char(c));
    }
    let lower = name.to_ascii_lowercase();
    Some(match lower.as_str() {
        "enter" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Esc,
        "tab" => KeyCode::Tab,
        "space" => KeyCode::Char(' '),
        "backspace" => KeyCode::Backspace,
        "delete" => KeyCode::Delete,
        "insert" => KeyCode::Insert,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "plus" => KeyCode::Char('+'),
        f => match f.strip_prefix('f').and_then(|n| n.parse::<u8>().ok()) {
            Some(n @ 1..=12) => KeyCode::F(n),
            _ => return None,
        },
    })
}

/// The config name of a named key.
fn name_of(code: KeyCode) -> &'static str {
    match code {
        KeyCode::Enter => "enter",
        KeyCode::Esc => "esc",
        KeyCode::Tab => "tab",
        KeyCode::Backspace => "backspace",
        KeyCode::Delete => "delete",
        KeyCode::Insert => "insert",
        KeyCode::Home => "home",
        KeyCode::End => "end",
        KeyCode::PageUp => "pageup",
        KeyCode::PageDown => "pagedown",
        KeyCode::Up => "up",
        KeyCode::Down => "down",
        KeyCode::Left => "left",
        KeyCode::Right => "right",
        _ => "?",
    }
}

/// Parses a key string: one key, or a sequence of keys separated by spaces (`g g`, `space f`).
pub fn parse_seq(s: &str) -> Result<Vec<Key>, String> {
    let keys = s
        .split_whitespace()
        .map(Key::parse)
        .collect::<Result<Vec<_>, _>>()?;
    if keys.is_empty() {
        return Err("an empty key".to_string());
    }
    Ok(keys)
}

/// A key sequence as the config writes it.
pub fn seq_config(seq: &[Key]) -> String {
    seq.iter().map(Key::config).collect::<Vec<_>>().join(" ")
}

/// A key sequence as hints show it: `gg`, `]c`, `^d`; a sequence of longer names keeps its spaces (`space f`).
pub fn seq_label(seq: &[Key], long: bool) -> String {
    let labels: Vec<String> = seq.iter().map(|k| k.label(long)).collect();
    if labels.iter().all(|l| l.chars().count() == 1) {
        labels.concat()
    } else {
        labels.join(" ")
    }
}

/// What a key sequence does in a mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lookup {
    Run(Command),
    /// The sequence starts a longer binding: wait for the next key.
    Wait,
    None,
}

/// The bindings of one mode.
#[derive(Debug, Clone)]
pub struct Table {
    /// In order: the config's bindings, then the remaining defaults in definition order.
    bindings: Vec<(Vec<Key>, Command)>,
    index: HashMap<Vec<Key>, Command>,
    prefixes: HashSet<Vec<Key>>,
    /// Default sequences that the config removed or rebound, for `minimap keys`.
    removed: Vec<(Vec<Key>, Command)>,
}

impl Table {
    pub fn lookup(&self, seq: &[Key]) -> Lookup {
        match self.index.get(seq) {
            Some(&c) => Lookup::Run(c),
            None if self.prefixes.contains(seq) => Lookup::Wait,
            None => Lookup::None,
        }
    }

    /// The key sequences bound to an action, first first.
    pub fn keys(&self, cmd: Command) -> impl Iterator<Item = &[Key]> {
        self.bindings
            .iter()
            .filter(move |(_, c)| *c == cmd)
            .map(|(s, _)| s.as_slice())
    }

    /// The hint label of an action's first key, or an empty string when it has none.
    pub fn label(&self, cmd: Command) -> String {
        self.keys(cmd)
            .next()
            .map_or_else(String::new, |s| seq_label(s, false))
    }

    /// The labels of all of an action's keys, joined by `sep`.
    pub fn labels(&self, cmd: Command, sep: &str, long: bool) -> String {
        self.keys(cmd)
            .map(|s| seq_label(s, long))
            .collect::<Vec<_>>()
            .join(sep)
    }

    /// The first keys of several actions, joined by `sep`. Actions without a key drop out.
    pub fn join(&self, cmds: &[Command], sep: &str) -> String {
        cmds.iter()
            .map(|&c| self.label(c))
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join(sep)
    }

    /// Every binding, as `(sequence, action)`, in order.
    pub fn bindings(&self) -> &[(Vec<Key>, Command)] {
        &self.bindings
    }

    /// Builds a mode's table from its defaults and the config's `[keys.<mode>]` table.
    fn build(mode: Mode, config: &BTreeMap<String, String>) -> Result<Table, Vec<String>> {
        let at = |ks: &str| format!("[keys.{}] \"{ks}\"", mode.name());
        let mut errors = Vec::new();
        let mut changes: Vec<(Vec<Key>, Option<Command>, &str)> = Vec::new();
        for (ks, action) in config {
            let seq = match parse_seq(ks) {
                Ok(s) => s,
                Err(e) => {
                    errors.push(format!("{}: {e}", at(ks)));
                    continue;
                }
            };
            let cmd = if action == "none" {
                None
            } else {
                match mode.defs().iter().find(|d| d.0.name() == action) {
                    Some(d) => Some(d.0),
                    None => {
                        let names: Vec<&str> = mode.defs().iter().map(|d| d.0.name()).collect();
                        let guess = names
                            .iter()
                            .map(|n| (distance(n, action), *n))
                            .filter(|&(d, _)| d <= 3)
                            .min()
                            .map_or_else(String::new, |(_, n)| format!(" Did you mean {n}?"));
                        errors.push(format!(
                            "{} = \"{action}\": not an action of the {} mode.{guess} Its actions: {}, or none",
                            at(ks),
                            mode.name(),
                            names.join(", ")
                        ));
                        continue;
                    }
                }
            };
            if mode == Mode::Viewer && is_count_digit(seq[0]) {
                errors.push(format!(
                    "{}: 1–9 start a count in the viewer and cannot be bound",
                    at(ks)
                ));
                continue;
            }
            if let Some((_, _, other)) = changes.iter().find(|(s, _, _)| *s == seq) {
                errors.push(format!(
                    "{}: the same key as \"{other}\" in this table",
                    at(ks)
                ));
                continue;
            }
            changes.push((seq, cmd, ks));
        }
        let mut bindings: Vec<(Vec<Key>, Command)> = changes
            .iter()
            .filter_map(|(s, c, _)| c.map(|c| (s.clone(), c)))
            .collect();
        let mut removed = Vec::new();
        for d in mode.defs() {
            for ks in d.1 {
                let seq = parse_seq(ks).expect("default keys parse");
                if changes.iter().any(|(s, _, _)| *s == seq) {
                    removed.push((seq, d.0));
                } else {
                    bindings.push((seq, d.0));
                }
            }
        }
        let index: HashMap<Vec<Key>, Command> = bindings.iter().cloned().collect();
        let mut prefixes = HashSet::new();
        for (seq, cmd) in &bindings {
            for n in 1..seq.len() {
                let prefix = &seq[..n];
                if let Some(other) = index.get(prefix) {
                    errors.push(format!(
                        "[keys.{}]: \"{}\" ({}) is bound and also starts \"{}\" ({}). Set one of them to \"none\"",
                        mode.name(),
                        seq_config(prefix),
                        other.name(),
                        seq_config(seq),
                        cmd.name()
                    ));
                }
                prefixes.insert(prefix.to_vec());
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        Ok(Table {
            bindings,
            index,
            prefixes,
            removed,
        })
    }
}

/// The edit distance between two names, for "did you mean".
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut row = vec![i + 1];
        for (j, &cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            row.push((prev[j] + cost).min(prev[j + 1] + 1).min(row[j] + 1));
        }
        prev = row;
    }
    prev[b.len()]
}

/// An unmodified `1`–`9`, which starts a count in the viewer.
pub fn is_count_digit(k: Key) -> bool {
    matches!(k.code, KeyCode::Char('1'..='9')) && k.mods.is_empty()
}

/// The key tables of every mode.
#[derive(Debug, Clone)]
pub struct Keymap {
    pub map: Table,
    pub outline: Table,
    pub viewer: Table,
}

static DEFAULT: LazyLock<Arc<Keymap>> = LazyLock::new(|| {
    Arc::new(Keymap::from_config(&BTreeMap::new()).expect("the default keymap is valid"))
});

impl Keymap {
    /// The default keymap, shared.
    pub fn shared_default() -> Arc<Keymap> {
        DEFAULT.clone()
    }

    /// The defaults with the config's `[keys]` tables. Every problem gives one message that names the table and the key.
    pub fn from_config(
        config: &BTreeMap<String, BTreeMap<String, String>>,
    ) -> Result<Keymap, Vec<String>> {
        let mut errors = Vec::new();
        for name in config.keys() {
            if !Mode::ALL.iter().any(|m| m.name() == name) {
                errors.push(format!(
                    "[keys.{name}]: not a mode. The modes: map, outline, viewer"
                ));
            }
        }
        let empty = BTreeMap::new();
        let mut table = |mode: Mode| {
            Table::build(mode, config.get(mode.name()).unwrap_or(&empty))
                .map_err(|e| errors.extend(e))
                .ok()
        };
        let (map, outline, viewer) = (table(Mode::Map), table(Mode::Outline), table(Mode::Viewer));
        match (map, outline, viewer) {
            (Some(map), Some(outline), Some(viewer)) if errors.is_empty() => Ok(Keymap {
                map,
                outline,
                viewer,
            }),
            _ => Err(errors),
        }
    }

    pub fn table(&self, mode: Mode) -> &Table {
        match mode {
            Mode::Map => &self.map,
            Mode::Outline => &self.outline,
            Mode::Viewer => &self.viewer,
        }
    }

    /// The keymap as config TOML, for `minimap keys`: every binding with its action's description,
    /// removed defaults as `"none"`, and actions without a key as comments. The output parses back to the same keymap.
    pub fn to_toml(&self) -> String {
        let mut out = String::from(
            "# The minimap keymap: the defaults with the config's changes (`docs/design.md` `Keymap`).\n\
             # Copy a line into a [keys.<mode>] table of the config file and change it. \"none\" removes a binding.\n",
        );
        for mode in Mode::ALL {
            let t = self.table(mode);
            out.push_str(&format!("\n[keys.{}]\n", mode.name()));
            let about = |c: Command| mode.defs().iter().find(|d| d.0 == c).map_or("", |d| d.2);
            let line = |seq: &[Key], value: &str, note: &str| {
                let entry = format!("{} = \"{value}\"", toml_string(&seq_config(seq)));
                format!("{entry:<32} # {note}\n")
            };
            for d in mode.defs() {
                for seq in t.keys(d.0) {
                    out.push_str(&line(seq, d.0.name(), d.2));
                }
            }
            for (seq, cmd) in &t.removed {
                if !t.index.contains_key(seq) {
                    let note = format!("removed default: {}", cmd.name());
                    out.push_str(&line(seq, "none", &note));
                }
            }
            for d in mode.defs() {
                if t.keys(d.0).next().is_none() {
                    out.push_str(&format!("# no key: {} ({})\n", d.0.name(), about(d.0)));
                }
            }
        }
        out
    }
}

/// A TOML basic string.
fn toml_string(s: &str) -> String {
    let mut q = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => q.push_str("\\\""),
            '\\' => q.push_str("\\\\"),
            c => q.push(c),
        }
    }
    q.push('"');
    q
}
