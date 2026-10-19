//! Grammar outlines (unit `032`): definitions from the `entity.name.*` scopes of the `two-face` syntax set (bat's syntaxes),
//! for languages without a tree-sitter outline. The syntax set loads once and is shared across threads.
//!
//! A renderer that parses a file for colors feeds its parse to an [`OutlineScan`], so it parses each file once.
//! [`crate::outline::generic`] parses a text for the outline alone. `syntect` and `two-face` parse text and draw nothing.

use std::sync::OnceLock;

use syntect::easy::ScopeRangeIterator;
use syntect::parsing::{ParseState, Scope, ScopeStack, ScopeStackOp, SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

use crate::outline::OutlineItem;

/// Files with more lines or bytes than this get no grammar outline, and the TUI's viewer shows them as plain text.
pub const MAX_LINES: usize = 20_000;
pub const MAX_BYTES: usize = 2 * 1024 * 1024;
/// Items nested deeper than this are left out of a generic outline.
pub const OUTLINE_MAX_DEPTH: u8 = 2;

/// Whether a text is too long for a grammar parse.
pub fn too_long(text: &str) -> bool {
    text.len() > MAX_BYTES || text.lines().count() > MAX_LINES
}

/// The `two-face` syntax set, loaded on first use.
pub fn syntaxes() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(two_face::syntax::extra_newlines)
}

/// The syntax for a path: by file name (such as `Makefile`), then extension, then the first line; else plain text.
pub fn syntax_for(path: &str, first_line: &str) -> &'static SyntaxReference {
    let s = syntaxes();
    let name = path.rsplit('/').next().unwrap_or(path);
    s.find_syntax_by_extension(name)
        .or_else(|| {
            name.rsplit_once('.')
                .and_then(|(_, ext)| s.find_syntax_by_extension(ext))
        })
        .or_else(|| s.find_syntax_by_first_line(first_line))
        .unwrap_or_else(|| s.find_syntax_plain_text())
}

/// The scopes that the outline walk looks for and skips (unit `032`).
struct Scopes {
    entity: Scope,
    /// `entity.name.*` scopes that name no definition: tags, labels, file names, selectors.
    deny: Vec<Scope>,
    /// Contexts where some grammars (TypeScript) tag calls and type references with `entity.name.*`.
    skip: Vec<Scope>,
    heading: Scope,
}

impl Scopes {
    fn new() -> Scopes {
        let s = |x: &str| Scope::new(x).expect("a valid scope");
        Scopes {
            entity: s("entity.name"),
            deny: ["tag", "label", "filename", "selector"]
                .iter()
                .map(|k| s(&format!("entity.name.{k}")))
                .collect(),
            skip: [
                "meta.function-call",
                "meta.parameters",
                "meta.type.annotation",
                "meta.return.type",
            ]
            .iter()
            .map(|x| s(x))
            .collect(),
            heading: s("markup.heading"),
        }
    }

    /// The definition scope a token is in, if it counts for the outline.
    fn definition(&self, stack: &[Scope]) -> Option<Scope> {
        if stack
            .iter()
            .any(|sc| self.skip.iter().any(|k| k.is_prefix_of(*sc)))
        {
            return None;
        }
        stack.iter().rev().copied().find(|sc| {
            self.entity.is_prefix_of(*sc) && !self.deny.iter().any(|d| d.is_prefix_of(*sc))
        })
    }

    /// The level of a Markdown heading around a token: `markup.heading.2.markdown` is 2.
    fn heading_level(&self, stack: &[Scope]) -> Option<u8> {
        stack
            .iter()
            .find(|sc| self.heading.is_prefix_of(**sc))
            .and_then(|sc| sc.build_string().split('.').nth(2)?.parse().ok())
    }
}

/// The outline kind of a definition scope: `entity.name.function.rust` is `fn`, `entity.name.type.interface.ts` is `interface`.
fn kind_of(scope: Scope) -> String {
    fn word(w: &str) -> &str {
        match w {
            "function" => "fn",
            "namespace" | "module" => "module",
            "constant" => "const",
            "alias" => "type",
            other => other,
        }
    }
    let full = scope.build_string();
    let parts: Vec<&str> = full.split('.').skip(2).collect();
    match parts.as_slice() {
        ["type", sub, _, ..] => word(sub).to_string(),
        [first, ..] => word(first).to_string(),
        [] => "item".to_string(),
    }
}

/// One line's outline candidate while the walk collects its name.
struct Found {
    scope: Scope,
    name: String,
    heading: Option<u8>,
    open: bool,
}

/// The display indent of a line, with tabs as 4 columns.
fn indent_of(line: &str) -> usize {
    let mut col = 0;
    for ch in line.chars() {
        match ch {
            ' ' => col += 1,
            '\t' => col += 4 - col % 4,
            _ => break,
        }
    }
    col
}

/// Sets each item's depth from its indent's rank among the items' indents (Markdown: its heading level),
/// and drops items deeper than [`OUTLINE_MAX_DEPTH`].
fn nest(raw: Vec<(OutlineItem, usize, Option<u8>)>) -> Vec<OutlineItem> {
    let mut indents: Vec<usize> = raw.iter().map(|(_, i, _)| *i).collect();
    indents.sort_unstable();
    indents.dedup();
    raw.into_iter()
        .filter_map(|(mut item, indent, heading)| {
            let depth = match heading {
                Some(level) => level.saturating_sub(1),
                None => indents.iter().position(|&i| i == indent).unwrap_or(0) as u8,
            };
            item.depth = depth;
            (depth <= OUTLINE_MAX_DEPTH).then_some(item)
        })
        .collect()
}

/// Collects outline items from a parse, one line at a time: a line's first token in a definition scope,
/// and the tokens after it in the same scope, name an item.
pub struct OutlineScan {
    scopes: Scopes,
    stack: ScopeStack,
    raw: Vec<(OutlineItem, usize, Option<u8>)>,
}

impl Default for OutlineScan {
    fn default() -> OutlineScan {
        OutlineScan::new()
    }
}

impl OutlineScan {
    pub fn new() -> OutlineScan {
        OutlineScan {
            scopes: Scopes::new(),
            stack: ScopeStack::new(),
            raw: Vec::new(),
        }
    }

    /// Takes one line's scope operations from `ParseState::parse_line`, its 0-based number, and its text with the line end.
    /// A line that did not parse is left out.
    pub fn line(&mut self, n: usize, line: &str, ops: &[(usize, ScopeStackOp)]) {
        let body = line.trim_end_matches(['\n', '\r']);
        let mut found: Option<Found> = None;
        for (range, op) in ScopeRangeIterator::new(ops, line) {
            if self.stack.apply(op).is_err() {
                break;
            }
            // A name right after `.` is a member access, even where a grammar misreads it as a definition.
            let after_dot = line[..range.start].trim_end().ends_with('.');
            let token = &line[range];
            match &mut found {
                Some(f) if f.open => {
                    if self.stack.as_slice().contains(&f.scope) {
                        f.name.push_str(token);
                    } else {
                        f.open = false;
                    }
                }
                Some(_) => {}
                None if token.trim().is_empty() || after_dot => {}
                None => {
                    if let Some(scope) = self.scopes.definition(self.stack.as_slice()) {
                        found = Some(Found {
                            scope,
                            name: token.to_string(),
                            heading: self.scopes.heading_level(self.stack.as_slice()),
                            open: true,
                        });
                    }
                }
            }
        }
        let Some(f) = found else {
            return;
        };
        let name = f.name.trim().to_string();
        if name.is_empty() {
            return;
        }
        let kind = kind_of(f.scope);
        // An ATX heading's level is its count of leading `#`; other headings use the grammar's level.
        let hashes = body.trim_start().chars().take_while(|&c| c == '#').count() as u8;
        let heading = if kind != "section" {
            None
        } else if hashes > 0 {
            Some(hashes)
        } else {
            f.heading
        };
        // An impl names its type, so it would read like the type's own item.
        let name = if kind == "impl" {
            format!("impl {name}")
        } else {
            name
        };
        let item = OutlineItem {
            line: n as u32 + 1,
            kind,
            name,
            private: false,
            depth: 0,
        };
        self.raw.push((item, indent_of(body), heading));
    }

    /// The items, nested.
    pub fn finish(self) -> Vec<OutlineItem> {
        nest(self.raw)
    }
}

/// The outline of a text from its grammar's definition scopes, with no colors. A text that is [`too_long`] has none.
pub(crate) fn generic(path: &str, text: &str) -> Vec<OutlineItem> {
    if too_long(text) {
        return Vec::new();
    }
    let syntax = syntax_for(path, text.lines().next().unwrap_or(""));
    let mut state = ParseState::new(syntax);
    let mut scan = OutlineScan::new();
    for (n, line) in LinesWithEndings::from(text).enumerate() {
        if let Ok(ops) = state.parse_line(line, syntaxes()) {
            scan.line(n, line, &ops);
        }
    }
    scan.finish()
}
