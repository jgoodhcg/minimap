//! The user config: per-repo exclude globs and generated-file rules, the viewer's theme, the editor for `e`, the keymap, and the GUI font.
//!
//! ```toml
//! [map]                       # every repo
//! generated = ["*.min.js"]    # gitignore rules; `!` unmarks a file (unit 041)
//!
//! [repos."/Users/example/projects/big-repo"]   # also every `git worktree` of it (unit 056)
//! exclude = ["web/public/icons/**"]
//! generated = ["schema/*.json", "!package-lock.json"]
//!
//! [viewer]
//! theme = "~/themes/other.tmTheme"
//!
//! [editor]
//! preset = "nvim"   # nvim | vim | vscode | emacs | custom
//! place = "tab"     # tab | right | down
//!
//! [keys.map]        # also [keys.outline] and [keys.viewer]
//! "h" = "pan_left"
//! "a" = "none"
//!
//! [gui]
//! font = "~/Library/Fonts/FiraCodeNerdFontMono-Regular.ttf"
//! ```

use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub repos: BTreeMap<String, RepoConfig>,
    #[serde(default)]
    pub viewer: ViewerConfig,
    #[serde(default)]
    pub editor: EditorConfig,
    /// The `[keys.<mode>]` tables (unit `033`): key string → action name. The TUI checks them.
    #[serde(default)]
    pub keys: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(default)]
    pub gui: GuiConfig,
    #[serde(default)]
    pub map: MapConfig,
}

/// The `[map]` table (unit `041`): options for every repo.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct MapConfig {
    /// Generated-file rules for every repo, before each repo's own rules (see [`crate::generated::Rules::gitignore`]).
    #[serde(default)]
    pub generated: Vec<String>,
}

/// The `[gui]` table (unit `035`).
#[derive(Debug, Default, Clone, Deserialize)]
pub struct GuiConfig {
    /// A Nerd Font `.ttf` for the GUI's text and icons. A leading `~/` expands.
    pub font: Option<String>,
}

/// The `[editor]` table (unit `031`). The TUI checks the values.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct EditorConfig {
    /// `nvim`, `vim`, `vscode`, `emacs`, or `custom`. Default: from `$VISUAL`, then `$EDITOR`, else `nvim`.
    pub preset: Option<String>,
    /// Where herdr opens a terminal editor: `tab`, `right`, or `down`. Default: `tab`.
    pub place: Option<String>,
    /// For `custom`: the command, with `{path}`, `{line}`, `{col}`, and `{root}` in its arguments.
    pub command: Option<Vec<String>>,
    /// For `custom`: run the command in minimap's terminal until it exits.
    pub terminal: Option<bool>,
}

/// The `[viewer]` table (unit `029`).
#[derive(Debug, Default, Clone, Deserialize)]
pub struct ViewerConfig {
    /// A `.tmTheme` file for syntax colors, in place of the built-in theme. A leading `~/` expands.
    pub theme: Option<String>,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct RepoConfig {
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Generated-file rules for this repo (unit `041`), after the `[map]` rules, so they override them.
    #[serde(default)]
    pub generated: Vec<String>,
}

/// `$XDG_CONFIG_HOME/minimap/config.toml`, or `~/.config/minimap/config.toml`.
pub fn default_path() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(x) if !x.is_empty() => PathBuf::from(x),
        _ => home()?.join(".config"),
    };
    Some(base.join("minimap").join("config.toml"))
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
}

/// Expands a leading `~/`.
pub fn expand_home(path: &str) -> PathBuf {
    match (path.strip_prefix("~/"), home()) {
        (Some(rest), Some(h)) => h.join(rest),
        _ => PathBuf::from(path),
    }
}

impl Config {
    /// Loads the config.
    /// With no explicit path, it reads the default path, and a missing file gives an empty config.
    /// An explicit path must exist, so a typo does not silently drop the excludes.
    pub fn load(explicit: Option<&Path>) -> Result<Config> {
        let Some(path) = explicit.map(Path::to_path_buf).or_else(default_path) else {
            return Ok(Config::default());
        };
        match fs::read_to_string(&path) {
            Ok(text) => Config::parse(&text).with_context(|| format!("in {}", path.display())),
            Err(e) if e.kind() == ErrorKind::NotFound && explicit.is_none() => {
                Ok(Config::default())
            }
            Err(e) => Err(e).with_context(|| format!("cannot read the config {}", path.display())),
        }
    }

    pub fn parse(text: &str) -> Result<Config> {
        toml::from_str(text).context("cannot parse the config")
    }

    /// The viewer theme's path, with `~/` expanded.
    pub fn viewer_theme(&self) -> Option<PathBuf> {
        self.viewer.theme.as_deref().map(expand_home)
    }

    /// The GUI font's path, with `~/` expanded.
    pub fn gui_font(&self) -> Option<PathBuf> {
        self.gui.font.as_deref().map(expand_home)
    }

    /// The exclude globs of every `[repos."<path>"]` table whose path is `root`.
    pub fn excludes_for(&self, root: &Path) -> Vec<String> {
        self.repos_for(root)
            .flat_map(|r| r.exclude.iter().cloned())
            .collect()
    }

    /// The generated-file rules for `root`: the `[map]` rules, then those of every `[repos."<path>"]` table whose path is `root`.
    pub fn generated_for(&self, root: &Path) -> Vec<String> {
        let mut rules = self.map.generated.clone();
        rules.extend(
            self.repos_for(root)
                .flat_map(|r| r.generated.iter().cloned()),
        );
        rules
    }

    /// The `[repos."<path>"]` tables whose path is `root`, or the main working tree of `root`'s repository (unit `056`).
    /// A key for a checkout covers its linked worktrees; a key for a worktree covers that worktree only.
    fn repos_for(&self, root: &Path) -> impl Iterator<Item = &RepoConfig> {
        let root = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
        let main = main_worktree(&root).filter(|m| *m != root);
        self.repos
            .iter()
            .filter(move |(key, _)| {
                let p = expand_home(key);
                let p = fs::canonicalize(&p).unwrap_or(p);
                p == root || main.as_deref() == Some(&p)
            })
            .map(|(_, r)| r)
    }
}

/// The main working tree of the repository at `root`: the parent of its common git dir.
/// `None` outside a repository, or for a bare one.
fn main_worktree(root: &Path) -> Option<PathBuf> {
    let repo = gix::discover(root).ok()?;
    // The common dir of a linked worktree ends in `/../..`, so resolve it before taking the parent.
    let common = fs::canonicalize(repo.common_dir()).ok()?;
    Some(common.parent()?.to_path_buf())
}
