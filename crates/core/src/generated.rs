//! Generated files (unit `041`): lock files and other files that a tool writes.
//!
//! Three marks decide, in this order, and each one that has an opinion about a file overrides the earlier ones:
//!
//! 1. The built-in list of lock-file names ([`LOCK_FILES`]), by exact name.
//! 2. `linguist-generated` in the repo's `.gitattributes` files, as GitHub Linguist reads it.
//! 3. The config's rules: gitignore rules, where a leading `!` unmarks a file.
//!
//! The `CARDS` layout caps a generated file's card height (`LayoutConfig::gen_lines`).

use anyhow::{Context, Result};
use globset::{Glob, GlobBuilder, GlobSet, GlobSetBuilder};

/// Lock files, by exact name. A pattern such as `*-lock.json` would catch files that are not lock files.
pub const LOCK_FILES: [&str; 16] = [
    "package-lock.json",
    "npm-shrinkwrap.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "Cargo.lock",
    "Gemfile.lock",
    "poetry.lock",
    "Pipfile.lock",
    "composer.lock",
    "go.sum",
    "flake.lock",
    "mix.lock",
    "pubspec.lock",
    "Podfile.lock",
    "deno.lock",
    "uv.lock",
];

/// The attribute that `.gitattributes` marks generated files with.
pub const ATTRIBUTE: &str = "linguist-generated";

/// The part of a repo-relative path after its last `/` is a built-in lock-file name.
pub fn is_lock_file(path: &str) -> bool {
    LOCK_FILES.contains(&path.rsplit('/').next().unwrap_or(path))
}

/// Ordered rules over repo-relative paths. The last rule that matches a path decides.
#[derive(Debug, Clone)]
pub struct Rules {
    set: GlobSet,
    /// Per glob of the set: the index of its rule.
    rule_of: Vec<usize>,
    /// Per rule: it marks (`Some(true)`), unmarks (`Some(false)`), or returns to unspecified (`None`, `.gitattributes` only).
    marks: Vec<Option<bool>>,
}

impl Default for Rules {
    fn default() -> Rules {
        Rules {
            set: GlobSet::empty(),
            rule_of: Vec::new(),
            marks: Vec::new(),
        }
    }
}

/// Collects rules, each one or more globs.
struct Builder {
    set: GlobSetBuilder,
    rule_of: Vec<usize>,
    marks: Vec<Option<bool>>,
}

impl Builder {
    fn new() -> Builder {
        Builder {
            set: GlobSetBuilder::new(),
            rule_of: Vec::new(),
            marks: Vec::new(),
        }
    }

    /// Adds one rule. A glob that does not build adds nothing of the rule.
    fn add(&mut self, globs: &[String], mark: Option<bool>) -> Result<(), globset::Error> {
        let built = globs
            .iter()
            .map(|g| GlobBuilder::new(g).literal_separator(true).build())
            .collect::<Result<Vec<Glob>, _>>()?;
        let rule = self.marks.len();
        self.marks.push(mark);
        for g in built {
            self.set.add(g);
            self.rule_of.push(rule);
        }
        Ok(())
    }

    fn build(self) -> Result<Rules, globset::Error> {
        Ok(Rules {
            set: self.set.build()?,
            rule_of: self.rule_of,
            marks: self.marks,
        })
    }
}

impl Rules {
    /// The config's rules, as gitignore rules (unit `041`):
    /// a pattern with no `/` matches that name in any directory; a pattern with a `/`, not only a trailing one, matches from the repo root;
    /// `*` does not cross `/`, and `**` does; a trailing `/` matches directories only; a rule that matches a directory marks every file under it;
    /// a leading `!` unmarks. A rule that does not build is an error that names it.
    pub fn gitignore(patterns: &[String]) -> Result<Rules> {
        let mut b = Builder::new();
        for p in patterns {
            let (mark, rest) = match p.trim().strip_prefix('!') {
                Some(rest) => (false, rest),
                None => (true, p.trim()),
            };
            let dir_only = rest.ends_with('/');
            let pat = rest.trim_end_matches('/');
            if pat.is_empty() {
                anyhow::bail!("bad generated rule {p:?}: it has no pattern");
            }
            let base = if pat.contains('/') {
                pat.trim_start_matches('/').to_string()
            } else {
                format!("**/{pat}")
            };
            let mut globs = vec![format!("{base}/**")];
            if !dir_only {
                globs.push(base);
            }
            b.add(&globs, Some(mark))
                .with_context(|| format!("bad generated rule {p:?}"))?;
        }
        Ok(b.build()?)
    }

    /// The `linguist-generated` lines of a `.gitattributes` file in the repo-relative directory `dir` (`""` for the root).
    /// Patterns follow git's attribute rules: relative to `dir`, at any depth below it without a `/`, never matching the files inside a matched directory.
    /// Negative patterns, `[attr]` macro lines, comments, and lines that do not parse are skipped.
    pub fn gitattributes(dir: &str, text: &str) -> Rules {
        let prefix = if dir.is_empty() {
            String::new()
        } else {
            format!("{dir}/")
        };
        let mut b = Builder::new();
        for line in text.lines() {
            let Some((pattern, attrs)) = split_line(line) else {
                continue;
            };
            // Git forbids negative patterns, and a pattern that ends in `/` matches directories only.
            if pattern.starts_with('!') || pattern.ends_with('/') || pattern.is_empty() {
                continue;
            }
            let Some(mark) = attrs.split_whitespace().rev().find_map(attribute) else {
                continue;
            };
            let glob = if pattern.contains('/') {
                format!("{prefix}{}", pattern.trim_start_matches('/'))
            } else {
                format!("{prefix}**/{pattern}")
            };
            // A pattern that does not build is skipped: a bad `.gitattributes` never stops the index.
            let _ = b.add(&[glob], mark);
        }
        b.build().unwrap_or_default()
    }

    /// The last matching rule's mark: `None` when no rule matches.
    pub fn decide(&self, path: &str) -> Option<Option<bool>> {
        if self.set.is_empty() {
            return None;
        }
        let rule = self
            .set
            .matches(path)
            .into_iter()
            .map(|g| self.rule_of[g])
            .max()?;
        Some(self.marks[rule])
    }
}

/// A `.gitattributes` line's pattern and the rest. `None` for a blank line, a comment, a macro, or a bad quoted pattern.
fn split_line(line: &str) -> Option<(String, &str)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') || line.starts_with("[attr]") {
        return None;
    }
    if let Some(quoted) = line.strip_prefix('"') {
        // A C-style quoted pattern, as git writes names with special characters.
        let mut pattern = String::new();
        let mut chars = quoted.char_indices();
        while let Some((i, c)) = chars.next() {
            match c {
                '"' => return Some((pattern, &quoted[i + 1..])),
                '\\' => match chars.next()?.1 {
                    't' => pattern.push('\t'),
                    'n' => pattern.push('\n'),
                    other => pattern.push(other),
                },
                _ => pattern.push(c),
            }
        }
        return None;
    }
    let end = line.find(char::is_whitespace).unwrap_or(line.len());
    Some((line[..end].to_string(), &line[end..]))
}

/// One attribute token's mark of [`ATTRIBUTE`]: set, or a value other than `false`, marks; `-` or `=false` unmarks;
/// `!` returns to unspecified. `None` for another attribute.
fn attribute(token: &str) -> Option<Option<bool>> {
    if let Some(name) = token.strip_prefix('-') {
        return (name == ATTRIBUTE).then_some(Some(false));
    }
    if let Some(name) = token.strip_prefix('!') {
        return (name == ATTRIBUTE).then_some(None);
    }
    match token.split_once('=') {
        Some((name, value)) => (name == ATTRIBUTE).then_some(Some(value != "false")),
        None => (token == ATTRIBUTE).then_some(Some(true)),
    }
}

/// The marks of one repo: its `.gitattributes` files and the config's rules.
#[derive(Debug, Clone, Default)]
pub struct Marks {
    /// One rule list per `.gitattributes` file, with its directory's depth, shallower files first.
    attributes: Vec<(usize, Rules)>,
    config: Rules,
}

impl Marks {
    /// The marks with the config's rules ([`Rules::gitignore`]) and no `.gitattributes` yet.
    pub fn new(config: &[String]) -> Result<Marks> {
        Ok(Marks {
            attributes: Vec::new(),
            config: Rules::gitignore(config)?,
        })
    }

    /// Adds the `.gitattributes` file at a repo-relative path, such as `web/.gitattributes`.
    pub fn add_attributes(&mut self, path: &str, text: &str) {
        let dir = path.rsplit_once('/').map_or("", |(d, _)| d);
        let depth = if dir.is_empty() {
            0
        } else {
            dir.split('/').count()
        };
        let rules = Rules::gitattributes(dir, text);
        if rules.set.is_empty() {
            return;
        }
        // A stable sort keeps files of one depth in the order they came; their patterns never match the same path.
        self.attributes.push((depth, rules));
        self.attributes.sort_by_key(|(d, _)| *d);
    }

    /// The file at a repo-relative path is generated: the built-in list, then `.gitattributes`, then the config.
    pub fn is_generated(&self, path: &str) -> bool {
        let mut generated = is_lock_file(path);
        // A deeper file's line overrides a shallower one's, and a later line an earlier one.
        let attr = self
            .attributes
            .iter()
            .rev()
            .find_map(|(_, r)| r.decide(path));
        if let Some(Some(v)) = attr {
            generated = v;
        }
        if let Some(Some(v)) = self.config.decide(path) {
            generated = v;
        }
        generated
    }
}

/// The repo-relative path names a `.gitattributes` file.
pub fn is_attributes_file(path: &str) -> bool {
    path.rsplit('/').next() == Some(".gitattributes")
}
