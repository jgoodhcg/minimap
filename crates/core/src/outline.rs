//! File outlines: top-level definitions and the namespace, parsed with tree-sitter (unit `007`).
//!
//! Only Clojure is registered. Files in other languages have no namespace, and [`generic`] outlines them
//! from their grammars (unit `032`, `crate::syntax`).

use tree_sitter::{Node, Parser};

/// One top-level definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineItem {
    /// The 1-based line of the definition's head.
    pub line: u32,
    /// The head symbol as written, such as `defn-` or `rf/reg-sub`.
    pub kind: String,
    /// The defined name as written, such as `get-berth` or `::berth-id`. A `defmethod` adds its dispatch value.
    pub name: String,
    /// The kind ends in `-`, or the name has `:private` metadata.
    pub private: bool,
    /// The nesting level, 0 for a top-level item. Clojure outlines hold top-level forms only (unit `032` adds nesting).
    pub depth: u8,
}

/// A language that can outline its files.
pub trait Language: Sync {
    /// File extensions without the dot.
    fn extensions(&self) -> &'static [&'static str];
    /// The top-level definitions, in file order.
    fn outline(&self, source: &str) -> Vec<OutlineItem>;
    /// The file's namespace, for languages that have one.
    fn namespace(&self, source: &str) -> Option<String>;
}

static LANGUAGES: &[&dyn Language] = &[&Clojure];

/// The outline of a text in a language without a registered [`Language`], from its grammar's definition scopes.
/// Files over 20,000 lines or 2 MiB have none ([`crate::syntax::too_long`]).
pub fn generic(path: &str, text: &str) -> Vec<OutlineItem> {
    crate::syntax::generic(path, text)
}

/// The registered language for a path, by its extension.
pub fn language_for(path: &str) -> Option<&'static dyn Language> {
    let name = path.rsplit('/').next().unwrap_or(path);
    let (_, ext) = name.rsplit_once('.')?;
    LANGUAGES
        .iter()
        .copied()
        .find(|l| l.extensions().contains(&ext))
}

/// Clojure, ClojureScript, `.cljc`, and Babashka, with `tree-sitter-clojure-orchard`.
///
/// A top-level list form is an item when the name part of its head symbol (after any `alias/`) starts with `def` or
/// `reg-`. Forms inside a top-level reader conditional count too. The item's name is the next symbol or keyword.
pub struct Clojure;

impl Clojure {
    /// True when the parse has an error or a missing node, for checks on real repos.
    pub fn has_errors(source: &str) -> bool {
        Clojure::parse(source).is_none_or(|t| t.root_node().has_error())
    }

    fn parse(source: &str) -> Option<tree_sitter::Tree> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_clojure_orchard::LANGUAGE.into())
            .ok()?;
        parser.parse(source, None)
    }
}

impl Language for Clojure {
    fn extensions(&self) -> &'static [&'static str] {
        &["clj", "cljs", "cljc", "bb"]
    }

    fn outline(&self, source: &str) -> Vec<OutlineItem> {
        let Some(tree) = Clojure::parse(source) else {
            return vec![];
        };
        let src = source.as_bytes();
        let mut items = Vec::new();
        for form in top_level_lists(tree.root_node()) {
            if let Some(item) = definition(form, src) {
                items.push(item);
            }
        }
        items
    }

    fn namespace(&self, source: &str) -> Option<String> {
        let tree = Clojure::parse(source)?;
        let src = source.as_bytes();
        let mut cursor = tree.root_node().walk();
        let ns = tree
            .root_node()
            .named_children(&mut cursor)
            .filter(|n| n.kind() == "list_lit")
            .find(|&n| head(n).is_some_and(|h| h.kind() == "sym_lit" && symbol(h, src) == "ns"))?;
        let name = values(ns).nth(1)?;
        (name.kind() == "sym_lit").then(|| symbol(name, src))
    }
}

/// The top-level list forms, and the list forms inside top-level reader conditionals, in file order.
fn top_level_lists(root: Node) -> Vec<Node> {
    let mut out = Vec::new();
    let mut cursor = root.walk();
    for n in root.named_children(&mut cursor) {
        match n.kind() {
            "list_lit" => out.push(n),
            // `#?(:clj (defn a …) :cljs (defn b …))`
            "read_cond_lit" => out.extend(values(n).filter(|v| v.kind() == "list_lit")),
            // `#?@(:clj [(defn a …) (defn b …)])`
            "splicing_read_cond_lit" => {
                for v in values(n).filter(|v| v.kind() == "vec_lit") {
                    out.extend(values(v).filter(|x| x.kind() == "list_lit"));
                }
            }
            _ => {}
        }
    }
    out
}

/// The `value` children of a collection: its forms, without comments, discards, or metadata.
fn values(n: Node) -> impl Iterator<Item = Node> {
    let mut cursor = n.walk();
    let v: Vec<Node> = n.children_by_field_name("value", &mut cursor).collect();
    v.into_iter()
}

fn head(list: Node) -> Option<Node> {
    values(list).next()
}

fn text<'a>(n: Node, src: &'a [u8]) -> &'a str {
    n.utf8_text(src).unwrap_or("")
}

/// A symbol as written, without its metadata: `ns/name` or `name`.
fn symbol(sym: Node, src: &[u8]) -> String {
    let name = sym.child_by_field_name("name").map_or("", |n| text(n, src));
    match sym.child_by_field_name("namespace") {
        Some(ns) => format!("{}/{name}", text(ns, src)),
        None => name.to_string(),
    }
}

/// True when a symbol has `^:private` or `^{:private true}` metadata.
fn has_private_meta(sym: Node, src: &[u8]) -> bool {
    let mut metas = Vec::new();
    for field in ["meta", "old_meta"] {
        let mut cursor = sym.walk();
        metas.extend(sym.children_by_field_name(field, &mut cursor));
    }
    metas.into_iter().any(|m| {
        let Some(v) = m.child_by_field_name("value") else {
            return false;
        };
        match v.kind() {
            "kwd_lit" => text(v, src) == ":private",
            "map_lit" => {
                let kv: Vec<Node> = values(v).collect();
                kv.chunks(2).any(|p| {
                    p.len() == 2 && text(p[0], src) == ":private" && text(p[1], src) == "true"
                })
            }
            _ => false,
        }
    })
}

/// An outline item for a list form whose head names a definition.
fn definition(list: Node, src: &[u8]) -> Option<OutlineItem> {
    let mut vals = values(list);
    let h = vals.next()?;
    if h.kind() != "sym_lit" {
        return None;
    }
    let local = h.child_by_field_name("name").map(|n| text(n, src))?;
    if !(local.starts_with("def") || local.starts_with("reg-")) {
        return None;
    }
    let kind = symbol(h, src);
    let target = vals.next()?;
    let (mut name, private_meta) = match target.kind() {
        "sym_lit" => (symbol(target, src), has_private_meta(target, src)),
        "kwd_lit" => (text(target, src).to_string(), false),
        _ => return None,
    };
    if local == "defmethod"
        && let Some(dispatch) = vals.next()
    {
        name.push(' ');
        name.push_str(
            &text(dispatch, src)
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
    Some(OutlineItem {
        line: h.start_position().row as u32 + 1,
        private: kind.ends_with('-') || private_meta,
        kind,
        name,
        depth: 0,
    })
}
