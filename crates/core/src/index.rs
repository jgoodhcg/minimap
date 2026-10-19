//! The file index: the tracked files of a repo and the untracked files that git does not ignore,
//! with line counts and binary flags.

use std::collections::HashMap;
use std::fs;
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::SystemTime;

use anyhow::{Context, Result};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Deserializer, Serialize};

use crate::generated::{self, Marks};

/// The number of leading bytes that the binary check reads.
const BINARY_PROBE: usize = 8 * 1024;

/// Options for [`Index::from_repo`].
#[derive(Debug, Default, Clone)]
pub struct IndexOptions {
    /// Globs that match repo-relative paths. A matching file is left out of the index.
    pub excludes: Vec<String>,
    /// The config's generated-file rules (unit `041`, [`crate::generated::Rules::gitignore`]).
    pub generated: Vec<String>,
}

/// One file: tracked, or untracked and not ignored. Its JSON form is [`FileJson`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "FileJson", into = "FileJson")]
pub struct IndexFile {
    /// The repo-relative path, with `/` separators.
    pub path: String,
    /// The line count. It is 0 for binary and missing files.
    pub lines: u64,
    /// The first 8 KiB of the file contain a NUL byte.
    pub binary: bool,
    /// The file is tracked but absent from the working tree.
    pub missing: bool,
    /// The file is gitignored and shown by an allowlist.
    pub ignored: bool,
    /// The file is not in the git index, and git does not ignore it (unit `034`).
    pub untracked: bool,
    /// A tool writes the file (unit `041`): a built-in lock-file name, `linguist-generated` in `.gitattributes`, or a config rule.
    pub generated: bool,
}

/// The JSON shape of one file, as `mockups/fake.json` has it. Flags are `1` when set and absent when not.
/// `g` is there only when the file differs from the built-in lock-file list: `1` for another generated file,
/// `0` for a lock-file name that `.gitattributes` or the config unmarks. So a dump keeps every mark.
#[derive(Serialize, Deserialize)]
struct FileJson {
    p: String,
    #[serde(default)]
    n: u64,
    #[serde(
        default,
        skip_serializing_if = "is_false",
        serialize_with = "flag_out",
        deserialize_with = "flag_in"
    )]
    b: bool,
    #[serde(
        default,
        skip_serializing_if = "is_false",
        serialize_with = "flag_out",
        deserialize_with = "flag_in"
    )]
    x: bool,
    #[serde(
        default,
        skip_serializing_if = "is_false",
        serialize_with = "flag_out",
        deserialize_with = "flag_in"
    )]
    ig: bool,
    #[serde(
        default,
        skip_serializing_if = "is_false",
        serialize_with = "flag_out",
        deserialize_with = "flag_in"
    )]
    u: bool,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "opt_flag_out",
        deserialize_with = "opt_flag_in"
    )]
    g: Option<bool>,
}

impl From<FileJson> for IndexFile {
    fn from(j: FileJson) -> IndexFile {
        IndexFile {
            generated: j.g.unwrap_or_else(|| generated::is_lock_file(&j.p)),
            path: j.p,
            lines: j.n,
            binary: j.b,
            missing: j.x,
            ignored: j.ig,
            untracked: j.u,
        }
    }
}

impl From<IndexFile> for FileJson {
    fn from(f: IndexFile) -> FileJson {
        let by_name = generated::is_lock_file(&f.path);
        FileJson {
            g: (f.generated != by_name).then_some(f.generated),
            p: f.path,
            n: f.lines,
            b: f.binary,
            x: f.missing,
            ig: f.ignored,
            u: f.untracked,
        }
    }
}

impl IndexFile {
    /// Makes a file entry with only a path.
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            lines: 0,
            binary: false,
            missing: false,
            ignored: false,
            untracked: false,
            generated: false,
        }
    }
}

/// The file index of one repo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Index {
    /// The name of the repo directory.
    pub root: String,
    /// The files, in git index order for a repo, or in file order for a JSON dump.
    pub files: Vec<IndexFile>,
    /// The number of files that the excludes dropped. It is not part of the JSON.
    #[serde(skip)]
    pub excluded: usize,
}

/// The JSON shape of `mockups/fake.json`. Unknown fields are ignored.
#[derive(Serialize, Deserialize)]
struct IndexJson {
    root: String,
    files: Vec<IndexFile>,
    #[serde(default)]
    commits: serde_json::Map<String, serde_json::Value>,
}

/// Finds the working-tree root of the git repo that contains `path`.
pub fn repo_root(path: &Path) -> Result<PathBuf> {
    let repo = gix::discover(path)
        .with_context(|| format!("no git repository at or above {}", path.display()))?;
    let root = repo
        .workdir()
        .with_context(|| format!("the repository at {} has no working tree", path.display()))?;
    fs::canonicalize(root).with_context(|| format!("cannot resolve {}", root.display()))
}

/// Builds a glob set that matches repo-relative paths. `*` does not cross `/`; `**` does.
pub fn exclude_set(globs: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for g in globs {
        let glob = GlobBuilder::new(g)
            .literal_separator(true)
            .build()
            .with_context(|| format!("bad exclude glob {g:?}"))?;
        builder.add(glob);
    }
    Ok(builder.build()?)
}

/// A file's size and modification time. A change to either means that its content changed.
pub type Stamp = (u64, Option<SystemTime>);

/// An index with the stamp of each file that the working tree has, for the next [`scan`].
#[derive(Debug, Clone)]
pub struct Scan {
    pub index: Index,
    pub stamps: HashMap<String, Stamp>,
}

impl Index {
    /// Lists the files of the repo that contains `path`: the tracked files and the untracked files that git does not ignore
    /// (the same set as `git ls-files --cached --others --exclude-standard`), minus the files that match an exclude glob.
    pub fn from_repo(path: &Path, options: &IndexOptions) -> Result<Index> {
        let root = repo_root(path)?;
        let repo = open(&root)?;
        Ok(scan(&repo, &root, options, None)?.index)
    }

    /// Loads an index dump or `mockups/fake.json`. A built-in lock-file name is generated unless its `g` is `0` (see [`FileJson`]).
    pub fn from_json(text: &str) -> Result<Index> {
        let j: IndexJson = serde_json::from_str(text).context("cannot parse the index JSON")?;
        Ok(Index {
            root: j.root,
            files: j.files,
            excluded: 0,
        })
    }

    /// Writes the index as JSON in the `mockups/fake.json` shape.
    pub fn to_json(&self) -> String {
        let j = IndexJson {
            root: self.root.clone(),
            files: self.files.clone(),
            commits: Default::default(),
        };
        serde_json::to_string(&j).expect("an index always serializes")
    }
}

/// Opens the repo at a working-tree root.
pub fn open(root: &Path) -> Result<gix::Repository> {
    gix::open(root).with_context(|| format!("cannot open the repository at {}", root.display()))
}

/// Indexes the repo at `root`. A file whose stamp equals its stamp in `prev` keeps its line count
/// and binary flag without a read, so a re-index reads only the files that changed.
/// Every scan reads the listed `.gitattributes` files again and marks generated files (unit `041`).
pub fn scan(
    repo: &gix::Repository,
    root: &Path,
    options: &IndexOptions,
    prev: Option<&Scan>,
) -> Result<Scan> {
    let index = repo.index_or_empty().context("cannot read the git index")?;
    let excludes = exclude_set(&options.excludes)?;
    let mut marks = Marks::new(&options.generated)?;

    let mut paths: Vec<(String, bool)> = Vec::with_capacity(index.entries().len());
    for entry in index.entries() {
        let p = entry.path(&index).to_string();
        // A conflicted path has one entry per stage. The map needs each path once.
        if paths.last().map(|(last, _)| last) != Some(&p) {
            paths.push((p, false));
        }
    }
    paths.extend(untracked(repo, &index)?.into_iter().map(|p| (p, true)));
    // Git index order: by path bytes. `String` order is byte order.
    paths.sort();
    // The attributes apply to every listed file, so an excluded `.gitattributes` still counts. A file that does not read adds nothing.
    for (p, _) in paths
        .iter()
        .filter(|(p, _)| generated::is_attributes_file(p))
    {
        if let Ok(bytes) = fs::read(root.join(p)) {
            marks.add_attributes(p, &String::from_utf8_lossy(&bytes));
        }
    }

    let old: HashMap<&str, (&IndexFile, &Stamp)> = prev
        .map(|p| {
            p.index
                .files
                .iter()
                .filter_map(|f| Some((f.path.as_str(), (f, p.stamps.get(&f.path)?))))
                .collect()
        })
        .unwrap_or_default();
    let mut files = Vec::with_capacity(paths.len());
    let mut stamps = HashMap::with_capacity(paths.len());
    let mut excluded = 0;
    for (p, untracked) in paths {
        if excludes.is_match(&p) {
            excluded += 1;
            continue;
        }
        let (mut file, stamp) = read_file(root, &p, old.get(p.as_str()).copied())?;
        if untracked && file.missing {
            // The file went away after the walk listed it.
            continue;
        }
        file.untracked = untracked;
        file.generated = marks.is_generated(&p);
        if let Some(stamp) = stamp {
            stamps.insert(p, stamp);
        }
        files.push(file);
    }

    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(Scan {
        index: Index {
            root: name,
            files,
            excluded,
        },
        stamps,
    })
}

/// The untracked files and symlinks that git does not ignore, as `git ls-files --others --exclude-standard` lists them.
/// Nested repositories are left out.
fn untracked(repo: &gix::Repository, index: &gix::index::State) -> Result<Vec<String>> {
    use gix::dir::entry::{Kind, Status};
    let options = repo
        .dirwalk_options()
        .context("cannot read the repository's file system options")?
        .emit_untracked(gix::dir::walk::EmissionMode::Matching)
        .emit_ignored(None)
        .emit_tracked(false)
        .emit_pruned(false)
        .emit_empty_directories(false)
        .recurse_repositories(false);
    let mut collect = gix::dir::walk::delegate::Collect::default();
    let interrupt = AtomicBool::new(false);
    repo.dirwalk(index, None::<&str>, &interrupt, options, &mut collect)
        .context("cannot list the untracked files")?;
    Ok(collect
        .unorded_entries
        .into_iter()
        .filter(|(e, _)| {
            e.status == Status::Untracked && matches!(e.disk_kind, Some(Kind::File | Kind::Symlink))
        })
        .map(|(e, _)| e.rela_path.to_string())
        .collect())
}

/// Reads one file from the working tree, with its stamp. A file whose stamp equals `prev`'s keeps `prev`'s counts.
fn read_file(
    root: &Path,
    path: &str,
    prev: Option<(&IndexFile, &Stamp)>,
) -> Result<(IndexFile, Option<Stamp>)> {
    let full = root.join(path);
    let mut file = IndexFile::new(path);
    let meta = match fs::symlink_metadata(&full) {
        Ok(m) => m,
        Err(e) if e.kind() == ErrorKind::NotFound || e.kind() == ErrorKind::NotADirectory => {
            file.missing = true;
            return Ok((file, None));
        }
        Err(e) => return Err(e).with_context(|| format!("cannot read {}", full.display())),
    };
    if meta.is_dir() {
        // A tracked path that is now a directory is not a file in the working tree.
        file.missing = true;
        return Ok((file, None));
    }
    let stamp = (meta.len(), meta.modified().ok());
    if let Some((old, old_stamp)) = prev
        && *old_stamp == stamp
    {
        file.lines = old.lines;
        file.binary = old.binary;
        return Ok((file, Some(stamp)));
    }
    let bytes = if meta.is_symlink() {
        // Git stores a symlink as a blob that holds the target path.
        fs::read_link(&full)?
            .to_string_lossy()
            .into_owned()
            .into_bytes()
    } else {
        let mut buf = Vec::with_capacity(meta.len() as usize);
        fs::File::open(&full)
            .and_then(|mut f| f.read_to_end(&mut buf))
            .with_context(|| format!("cannot read {}", full.display()))?;
        buf
    };
    if is_binary(&bytes) {
        file.binary = true;
    } else {
        file.lines = count_lines(&bytes);
    }
    Ok((file, Some(stamp)))
}

/// A file is binary when its first 8 KiB contain a NUL byte.
pub fn is_binary(bytes: &[u8]) -> bool {
    bytes[..bytes.len().min(BINARY_PROBE)].contains(&0)
}

/// Counts `\n` bytes, plus 1 when the text is not empty and does not end in `\n`.
pub fn count_lines(bytes: &[u8]) -> u64 {
    let newlines = bytes.iter().filter(|&&b| b == b'\n').count() as u64;
    let tail = u64::from(bytes.last().is_some_and(|&b| b != b'\n'));
    newlines + tail
}

fn is_false(b: &bool) -> bool {
    !*b
}

fn flag_out<S: serde::Serializer>(_: &bool, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_u8(1)
}

fn opt_flag_out<S: serde::Serializer>(b: &Option<bool>, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_u8(u8::from(b.unwrap_or(false)))
}

fn opt_flag_in<'de, D: Deserializer<'de>>(d: D) -> Result<Option<bool>, D::Error> {
    flag_in(d).map(Some)
}

/// Accepts `1`/`0` and `true`/`false`.
fn flag_in<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    match serde_json::Value::deserialize(d)? {
        serde_json::Value::Bool(b) => Ok(b),
        serde_json::Value::Number(n) => Ok(n.as_f64() != Some(0.0)),
        serde_json::Value::Null => Ok(false),
        other => Err(serde::de::Error::custom(format!(
            "expected a flag, found {other}"
        ))),
    }
}
