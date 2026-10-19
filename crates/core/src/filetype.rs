//! File-type icons (Nerd Font glyphs) and type colors, as data for every renderer.
//!
//! The glyphs need a Nerd Font. Type colors are low chroma (HSL saturation at most 40 %), so high-chroma state colors
//! stand apart (`docs/design.md` `Color`).
//! The main types in the target repo get well-spaced hues; rarer types share neighborhoods. Tune the values here.

use crate::palette;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Icon {
    pub glyph: char,
    /// The type color, `0xRRGGBB`.
    pub rgb: u32,
}

const fn icon(glyph: char, rgb: u32) -> Icon {
    Icon { glyph, rgb }
}

pub const CLOJURE: Icon = icon('\u{e768}', 0x98c577);
pub const CLOJURESCRIPT: Icon = icon('\u{e76a}', 0x7eafc8);
pub const EDN: Icon = icon('\u{e76a}', 0xc19cd3);
pub const MARKDOWN: Icon = icon('\u{e73e}', 0xdedede);
pub const JSON: Icon = icon('\u{e60b}', 0xd1c7a9);
pub const CONFIG: Icon = icon('\u{e615}', 0x8c9cb1);
pub const TYPESCRIPT: Icon = icon('\u{e628}', 0xa2a9d7);
pub const JAVASCRIPT: Icon = icon('\u{e74e}', 0xcbc586);
pub const PYTHON: Icon = icon('\u{e73c}', 0x74beab);
pub const SQL: Icon = icon('\u{e706}', 0xc8a17e);
pub const SHELL: Icon = icon('\u{e795}', 0x8ec293);
pub const HTML: Icon = icon('\u{e736}', 0xcb8f86);
pub const CSS: Icon = icon('\u{e749}', 0x8aa3c7);
pub const RUST: Icon = icon('\u{e7a8}', 0xd4b5a5);
pub const DOCKER: Icon = icon('\u{e7b0}', 0xa2bbcd);
pub const GIT: Icon = icon('\u{e702}', 0xc57777);
pub const LOCK: Icon = icon('\u{f023}', 0xbababa);
pub const IMAGE: Icon = icon('\u{f1c5}', 0xc6af95);
pub const ARCHIVE: Icon = icon('\u{f1c6}', 0xc2a770);
pub const TEXT: Icon = icon('\u{f0f6}', 0x99a4ad);
pub const FILE: Icon = icon('\u{f016}', 0x89949f);

/// Every icon, for the palette checks.
pub const ALL: [Icon; 21] = [
    CLOJURE,
    CLOJURESCRIPT,
    EDN,
    MARKDOWN,
    JSON,
    CONFIG,
    TYPESCRIPT,
    JAVASCRIPT,
    PYTHON,
    SQL,
    SHELL,
    HTML,
    CSS,
    RUST,
    DOCKER,
    GIT,
    LOCK,
    IMAGE,
    ARCHIVE,
    TEXT,
    FILE,
];

/// Folder glyphs for the file tree.
pub const FOLDER_CLOSED: char = '\u{f07b}';
pub const FOLDER_OPEN: char = '\u{f07c}';

/// The icon for a file name. Special names match first, then the lowercase extension.
pub fn for_name(name: &str) -> Icon {
    match name {
        "Dockerfile" => return DOCKER,
        ".gitignore" | ".gitattributes" | ".gitmodules" => return GIT,
        "package-lock.json" => return LOCK,
        "Makefile" | "Procfile" => return CONFIG,
        "LICENSE" => return TEXT,
        _ => {}
    }
    let ext = match name.rfind('.') {
        Some(d) if d > 0 => name[d + 1..].to_ascii_lowercase(),
        _ => String::new(),
    };
    match ext.as_str() {
        "clj" | "cljc" | "bb" => CLOJURE,
        "cljs" => CLOJURESCRIPT,
        "edn" => EDN,
        "md" | "markdown" => MARKDOWN,
        "json" => JSON,
        "yml" | "yaml" | "toml" | "tf" | "tfvars" | "ini" | "conf" => CONFIG,
        "ts" | "tsx" => TYPESCRIPT,
        "js" | "jsx" | "mjs" | "cjs" => JAVASCRIPT,
        "py" => PYTHON,
        "sql" => SQL,
        "sh" | "bash" | "zsh" => SHELL,
        "html" | "xml" => HTML,
        "css" | "scss" | "sass" => CSS,
        "rs" => RUST,
        "dockerfile" => DOCKER,
        "lock" => LOCK,
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "ico" | "webp" => IMAGE,
        "zip" | "gz" | "tar" | "jar" => ARCHIVE,
        "txt" | "csv" => TEXT,
        _ => FILE,
    }
}

/// The type color of a file: its icon's color. A binary file keeps the dim binary color.
/// Only icons carry it; names stay neutral (`docs/design.md` `Color`).
pub fn type_rgb(name: &str, binary: bool) -> u32 {
    if binary {
        return palette::BINARY;
    }
    for_name(name).rgb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table() {
        let g = |n: &str| for_name(n).glyph;
        assert_eq!(for_name("a.clj"), CLOJURE);
        assert_eq!(for_name("a.CLJC"), CLOJURE);
        assert_eq!(for_name("a.cljs"), CLOJURESCRIPT);
        assert_eq!(for_name("a.edn"), EDN);
        assert_ne!(for_name("a.edn").rgb, for_name("a.cljs").rgb);
        assert_eq!(for_name("README.md"), MARKDOWN);
        assert_eq!(for_name("Dockerfile"), DOCKER);
        assert_eq!(for_name("web.dockerfile"), DOCKER);
        assert_eq!(for_name(".gitignore"), GIT);
        assert_eq!(for_name("package-lock.json"), LOCK);
        assert_eq!(for_name("package.json"), JSON);
        assert_eq!(for_name("yarn.lock"), LOCK);
        assert_eq!(for_name("x.unknown"), FILE);
        assert_eq!(for_name(".env"), FILE);
        assert_eq!(g("Makefile"), '\u{e615}');
    }

    #[test]
    fn type_colors() {
        assert_eq!(type_rgb("a.bin", true), palette::BINARY);
        assert_eq!(type_rgb("a.clj", false), CLOJURE.rgb);
    }

    #[test]
    fn every_type_has_its_own_color() {
        for (i, a) in ALL.iter().enumerate() {
            for b in &ALL[i + 1..] {
                assert!(a.rgb != b.rgb, "{a:?} and {b:?} share a color");
            }
        }
    }
}
