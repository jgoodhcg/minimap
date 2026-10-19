//! Syntax colors (unit `055`): `syntect` with the `two-face` syntax set (bat's syntaxes) and a `.tmTheme`,
//! as `0xRRGGBB` values, so the TUI's viewer (unit `029`) and the GUI's note reader (unit `054`) color text alike.
//! One parse gives a text's colors and its grammar outline (unit `032`). The built-in theme is tokyonight-night (`crates/core/assets/`).

use std::io::Cursor;
use std::path::Path;
use std::time::{Duration, Instant};

use syntect::highlighting::{
    FontStyle, HighlightIterator, HighlightState, Highlighter as ThemeHighlighter, Theme, ThemeSet,
};
use syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

use crate::outline::OutlineItem;
use crate::palette;
use crate::syntax::{OutlineScan, syntax_for, syntaxes};

/// The default theme: tokyonight-night from tokyonight.nvim (Apache-2.0, see `crates/core/assets/`).
const DEFAULT_THEME: &[u8] = include_bytes!("../assets/tokyonight_night.tmTheme");

/// A run of text in one style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    /// The foreground as `0xRRGGBB`.
    pub rgb: u32,
    pub bold: bool,
    pub italic: bool,
}

/// A text's colored lines and its grammar outline.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Highlighted {
    /// The syntax's name, such as `Clojure`, or `Plain Text`.
    pub syntax: String,
    /// One list of spans per line, without the line ends. Empty when the parse was for the outline only.
    pub lines: Vec<Vec<Span>>,
    /// The outline from the grammar's definition scopes (unit `032`).
    pub outline: Vec<OutlineItem>,
    pub took: Duration,
}

/// The syntaxes and a theme.
pub struct Highlighter {
    syntaxes: &'static SyntaxSet,
    theme: Theme,
}

fn default_theme() -> Theme {
    ThemeSet::load_from_reader(&mut Cursor::new(DEFAULT_THEME)).expect("the built-in theme loads")
}

fn hex(c: syntect::highlighting::Color) -> u32 {
    (u32::from(c.r) << 16) | (u32::from(c.g) << 8) | u32::from(c.b)
}

impl Highlighter {
    /// Loads the syntaxes and the theme at `theme`, or the built-in theme.
    /// A theme that fails to load gives the built-in theme and the error.
    pub fn new(theme: Option<&Path>) -> (Highlighter, Option<String>) {
        let (theme, err) = match theme {
            None => (default_theme(), None),
            Some(p) => match ThemeSet::get_theme(p) {
                Ok(t) => (t, None),
                Err(e) => (
                    default_theme(),
                    Some(format!("Cannot load the theme {}: {e}", p.display())),
                ),
            },
        };
        let h = Highlighter {
            syntaxes: syntaxes(),
            theme,
        };
        (h, err)
    }

    /// The syntax for a path: by file name (such as `Makefile`), then extension, then the first line; else plain text.
    pub fn syntax(&self, path: &str, first_line: &str) -> &SyntaxReference {
        syntax_for(path, first_line)
    }

    /// The theme's default text color.
    pub fn foreground(&self) -> Option<u32> {
        self.theme.settings.foreground.map(hex)
    }

    /// Colors every line of a text, and outlines it, in one parse.
    /// Only foregrounds, bold, and italic are kept; the front end keeps its background.
    pub fn highlight(&self, path: &str, text: &str) -> Highlighted {
        self.parse(path, text, true)
    }

    /// The outline of a text from its grammar's definition scopes (unit `032`), without colors.
    pub fn outline(&self, path: &str, text: &str) -> Vec<OutlineItem> {
        self.parse(path, text, false).outline
    }

    fn parse(&self, path: &str, text: &str, colors: bool) -> Highlighted {
        let t0 = Instant::now();
        let syntax = self.syntax(path, text.lines().next().unwrap_or(""));
        let plain = self.foreground().unwrap_or(palette::INK);
        let theme = ThemeHighlighter::new(&self.theme);
        let mut hstate = HighlightState::new(&theme, ScopeStack::new());
        let mut pstate = ParseState::new(syntax);
        let mut scan = OutlineScan::new();
        let mut lines = Vec::new();
        for (n, line) in LinesWithEndings::from(text).enumerate() {
            let body = line.trim_end_matches(['\n', '\r']);
            let Ok(ops) = pstate.parse_line(line, self.syntaxes) else {
                if colors {
                    lines.push(vec![Span {
                        text: body.to_string(),
                        rgb: plain,
                        bold: false,
                        italic: false,
                    }]);
                }
                continue;
            };
            if colors {
                let spans = HighlightIterator::new(&mut hstate, &ops, line, &theme)
                    .filter_map(|(style, s)| {
                        let s = s.trim_end_matches(['\n', '\r']);
                        (!s.is_empty()).then(|| Span {
                            text: s.to_string(),
                            rgb: hex(style.foreground),
                            bold: style.font_style.contains(FontStyle::BOLD),
                            italic: style.font_style.contains(FontStyle::ITALIC),
                        })
                    })
                    .collect();
                lines.push(spans);
            }
            scan.line(n, line, &ops);
        }
        Highlighted {
            syntax: syntax.name.clone(),
            lines,
            outline: scan.finish(),
            took: t0.elapsed(),
        }
    }
}
