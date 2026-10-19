//! File text for the viewer (unit `029`): the working tree and its `HEAD` (unit `030`), or text held in memory for tests.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

#[derive(Debug, Default, Clone)]
pub enum Content {
    /// No file text, as in `--from-json` mode.
    #[default]
    None,
    /// Read files from this working tree.
    Repo(PathBuf),
    /// Text by repo-relative path, in the working tree and at `HEAD`.
    Memory {
        files: HashMap<String, String>,
        head: HashMap<String, String>,
    },
}

impl Content {
    /// A file's text. `Ok(None)` without a source, or for a path that memory does not hold.
    /// Invalid UTF-8 is replaced.
    pub fn text(&self, path: &str) -> Result<Option<String>> {
        match self {
            Content::None => Ok(None),
            Content::Repo(root) => {
                let bytes =
                    fs::read(root.join(path)).with_context(|| format!("cannot read {path}"))?;
                Ok(Some(String::from_utf8_lossy(&bytes).into_owned()))
            }
            Content::Memory { files, .. } => Ok(files.get(path).cloned()),
        }
    }

    /// A file's text at `HEAD`. `Ok(None)` without a source, or when `HEAD` has no such file.
    pub fn head(&self, path: &str) -> Result<Option<String>> {
        match self {
            Content::None => Ok(None),
            Content::Repo(root) => crate::diff::text_at(root, "HEAD", path),
            Content::Memory { head, .. } => Ok(head.get(path).cloned()),
        }
    }

    /// Text in memory, with the same text at `HEAD`: no uncommitted changes.
    pub fn memory(files: HashMap<String, String>) -> Content {
        Content::Memory {
            head: files.clone(),
            files,
        }
    }

    /// The working tree's root, for a source that has one.
    pub fn root(&self) -> Option<&Path> {
        match self {
            Content::Repo(root) => Some(root),
            _ => None,
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Content::None)
    }
}
