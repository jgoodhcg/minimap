//! The live index (unit `034`): a thread watches the working tree and sends a new index when files change.
//!
//! The thread collects file events for [`SETTLE`] after the first event that counts, then re-indexes.
//! Events under `.git/` count only for the git index, `HEAD`, and the refs. Events for gitignored and
//! config-excluded paths do not count, so a build that writes to `target/` stays quiet.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use globset::GlobSet;
use notify::{Event, EventKind, RecursiveMode, Watcher as _};

use crate::index::{Index, IndexOptions, Scan, Stamp, exclude_set, open, scan};

/// The time the thread collects events after the first one, before it re-indexes.
pub const SETTLE: Duration = Duration::from_millis(250);

/// A new index, sent when the file set, a line count, or a file's content changed, or `HEAD` moved.
#[derive(Debug, Clone)]
pub struct Update {
    pub index: Index,
    /// The paths whose content changed since the last index, by size and modification time.
    pub changed: Vec<String>,
    /// `HEAD` points at another commit.
    pub head_moved: bool,
}

/// A running watch. Dropping it stops the thread.
pub struct Watcher {
    rx: mpsc::Receiver<Result<Update, String>>,
    scans: Arc<AtomicUsize>,
    _watch: notify::RecommendedWatcher,
}

impl Watcher {
    /// Watches the repo at `root`, a canonical working-tree root, recursively. `index` is the index the map shows now.
    pub fn spawn(root: PathBuf, options: IndexOptions, index: Index) -> Result<Watcher> {
        Watcher::spawn_with(root, options, index, || {})
    }

    /// As [`Watcher::spawn`], and `wake` runs after each update or error is queued, so a window can repaint (unit `048`).
    pub fn spawn_with(
        root: PathBuf,
        options: IndexOptions,
        index: Index,
        wake: impl Fn() + Send + 'static,
    ) -> Result<Watcher> {
        let repo = open(&root)?;
        let excludes = exclude_set(&options.excludes)?;
        let (events_tx, events) = mpsc::channel();
        let mut watch =
            notify::recommended_watcher(events_tx).context("cannot start the file watcher")?;
        watch
            .watch(&root, RecursiveMode::Recursive)
            .with_context(|| format!("cannot watch {}", root.display()))?;
        let (out, rx) = mpsc::channel();
        // The scan thread sends to a forwarder, which queues each message and then wakes the caller.
        let (tx, forward) = mpsc::channel::<Result<Update, String>>();
        std::thread::spawn(move || {
            for msg in forward {
                if out.send(msg).is_err() {
                    return;
                }
                wake();
            }
        });
        let scans = Arc::new(AtomicUsize::new(0));
        let git_dir = fs::canonicalize(repo.git_dir()).unwrap_or_else(|_| root.join(".git"));
        let counter = scans.clone();
        // A `gix::Repository` cannot move to another thread, so the thread opens its own.
        std::thread::spawn(move || {
            let repo = match open(&root) {
                Ok(r) => r,
                Err(e) => return drop(tx.send(Err(format!("{e:#}")))),
            };
            let thread = Thread {
                repo,
                root,
                git_dir,
                options,
                excludes,
                scans: counter,
            };
            thread.run(index, events, tx)
        });
        Ok(Watcher {
            rx,
            scans,
            _watch: watch,
        })
    }

    /// Takes what arrived since the last poll: the updates merged into one (the newest index, every changed path,
    /// and `head_moved` when any update moved `HEAD`), and the newest error.
    pub fn poll(&self) -> (Option<Update>, Option<String>) {
        let (mut update, mut error): (Option<Update>, _) = (None, None);
        for msg in self.rx.try_iter() {
            match msg {
                Ok(next) => {
                    update = Some(match update {
                        None => next,
                        Some(mut u) => {
                            for p in next.changed {
                                if !u.changed.contains(&p) {
                                    u.changed.push(p);
                                }
                            }
                            Update {
                                index: next.index,
                                changed: u.changed,
                                head_moved: u.head_moved || next.head_moved,
                            }
                        }
                    })
                }
                Err(e) => error = Some(e),
            }
        }
        (update, error)
    }

    /// The re-indexes the thread has run, for tests.
    pub fn scans(&self) -> usize {
        self.scans.load(Ordering::Relaxed)
    }
}

/// The watch thread's state.
struct Thread {
    repo: gix::Repository,
    root: PathBuf,
    git_dir: PathBuf,
    options: IndexOptions,
    excludes: GlobSet,
    scans: Arc<AtomicUsize>,
}

/// What a batch of events asks for.
#[derive(Debug, Default, Clone, Copy)]
struct Wants {
    /// Re-index.
    scan: bool,
    /// `HEAD` or a ref changed: check whether `HEAD` moved.
    refs: bool,
}

impl Thread {
    fn run(
        self,
        index: Index,
        events: mpsc::Receiver<notify::Result<Event>>,
        tx: mpsc::Sender<Result<Update, String>>,
    ) {
        let stamps = index
            .files
            .iter()
            .filter_map(|f| Some((f.path.clone(), stamp(&self.root.join(&f.path))?)))
            .collect();
        let mut last = Scan { index, stamps };
        let mut head = self.head();
        loop {
            let mut filter = match Filter::new(&self, &last.index) {
                Ok(f) => f,
                Err(e) => {
                    let _ = tx.send(Err(format!("{e:#}")));
                    return;
                }
            };
            // Wait for the first event that counts, then collect events for SETTLE.
            let mut wants = Wants::default();
            while !wants.scan {
                match events.recv() {
                    Ok(Ok(ev)) => wants = filter.wants(&self, &ev),
                    Ok(Err(_)) => {}
                    Err(_) => return,
                }
            }
            let deadline = Instant::now() + SETTLE;
            loop {
                let left = deadline.saturating_duration_since(Instant::now());
                match events.recv_timeout(left) {
                    Ok(Ok(ev)) => wants.refs |= filter.wants(&self, &ev).refs,
                    Ok(Err(_)) => {}
                    Err(mpsc::RecvTimeoutError::Timeout) => break,
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
            drop(filter);
            self.scans.fetch_add(1, Ordering::Relaxed);
            let next = match scan(&self.repo, &self.root, &self.options, Some(&last)) {
                Ok(next) => next,
                Err(e) => {
                    if tx.send(Err(format!("{e:#}"))).is_err() {
                        return;
                    }
                    continue;
                }
            };
            let head_moved = wants.refs && {
                let now = self.head();
                let moved = now != head;
                head = now;
                moved
            };
            let changed = changed(&last, &next);
            if next.index.files != last.index.files || !changed.is_empty() || head_moved {
                let update = Update {
                    index: next.index.clone(),
                    changed,
                    head_moved,
                };
                if tx.send(Ok(update)).is_err() {
                    return;
                }
            }
            last = next;
        }
    }

    fn head(&self) -> Option<gix::ObjectId> {
        self.repo.head_id().ok().map(|id| id.detach())
    }
}

/// Decides which events count, for one index.
struct Filter {
    /// The indexed paths and every directory above them.
    files: HashSet<String>,
    dirs: HashSet<String>,
    /// The gitignore rules.
    ignore: gix::worktree::Stack,
}

impl Filter {
    fn new(t: &Thread, index: &Index) -> Result<Filter> {
        let git_index = t
            .repo
            .index_or_empty()
            .context("cannot read the git index")?;
        let ignore = t
            .repo
            .excludes(
                &git_index,
                None,
                gix::worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
            )
            .context("cannot read the gitignore rules")?
            .detach();
        let files: HashSet<String> = index.files.iter().map(|f| f.path.clone()).collect();
        let mut dirs = HashSet::new();
        for p in &files {
            let mut d = p.as_str();
            while let Some((up, _)) = d.rsplit_once('/') {
                if !dirs.insert(up.to_string()) {
                    break;
                }
                d = up;
            }
        }
        Ok(Filter {
            files,
            dirs,
            ignore,
        })
    }

    fn wants(&mut self, t: &Thread, ev: &Event) -> Wants {
        let mut w = Wants::default();
        if ev.need_rescan() {
            w.scan = true;
        }
        if matches!(ev.kind, EventKind::Access(_)) {
            return w;
        }
        for path in &ev.paths {
            if let Ok(rel) = path.strip_prefix(&t.git_dir) {
                let rel = slash_path(rel);
                if rel.ends_with(".lock") {
                    continue;
                }
                let refs = rel == "HEAD" || rel == "packed-refs" || rel.starts_with("refs/");
                w.scan |= refs || rel == "index";
                w.refs |= refs;
                continue;
            }
            let Ok(rel) = path.strip_prefix(&t.root) else {
                continue;
            };
            let rel = slash_path(rel);
            if rel.is_empty() || rel == ".git" || rel.starts_with(".git/") {
                continue;
            }
            // A directory is excluded when a glob matches every name in it, as `public/**` does.
            // The NUL name stands for any name, because file names hold no NUL.
            if t.excludes.is_match(&rel) || t.excludes.is_match(format!("{rel}/\0")) {
                continue;
            }
            if self.files.contains(&rel) || self.dirs.contains(&rel) || !self.ignored(t, &rel, path)
            {
                w.scan = true;
            }
        }
        w
    }

    /// The path is gitignored, or lies in an ignored directory.
    fn ignored(&mut self, t: &Thread, rel: &str, full: &Path) -> bool {
        let dir = fs::symlink_metadata(full).is_ok_and(|m| m.is_dir());
        let mode = if dir {
            gix::index::entry::Mode::DIR
        } else {
            gix::index::entry::Mode::FILE
        };
        self.ignore
            .at_path(rel, Some(mode), &t.repo.objects)
            .is_ok_and(|p| p.is_excluded())
    }
}

/// A repo-relative path with `/` separators.
fn slash_path(rel: &Path) -> String {
    rel.to_string_lossy().replace('\\', "/")
}

fn stamp(full: &Path) -> Option<Stamp> {
    let m = fs::symlink_metadata(full).ok()?;
    (!m.is_dir()).then(|| (m.len(), m.modified().ok()))
}

/// The paths in both indexes whose stamps differ, including files that appeared on disk or went away from it.
fn changed(old: &Scan, new: &Scan) -> Vec<String> {
    let before: HashSet<&str> = old.index.files.iter().map(|f| f.path.as_str()).collect();
    new.index
        .files
        .iter()
        .map(|f| f.path.as_str())
        .filter(|p| before.contains(p) && old.stamps.get(*p) != new.stamps.get(*p))
        .map(str::to_string)
        .collect()
}
