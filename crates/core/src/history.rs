//! Per-file git history (unit `008`): commit counts, the newest and oldest change times, and recent commits.
//!
//! The walk visits every commit reachable from `HEAD`, newest first, and skips merge commits.
//! Each other commit's tree is diffed against its parent's, and the root commit's against the empty tree,
//! so a file counts what `git rev-list --no-merges --full-history HEAD -- <path>` counts. Renames are not followed.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use gix::bstr::ByteSlice;
use gix::diff::tree::recorder::Change;
use gix::revision::walk::Sorting;
use gix::traverse::commit::simple::CommitTimeOrder;
use serde::Deserialize;

/// Seconds per day.
pub const DAY: i64 = 86_400;
/// The recent commits kept per file.
pub const RECENT: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    /// The full hex object id.
    pub hash: String,
    /// The committer time, in seconds since the epoch.
    pub time: i64,
    /// The first line of the message.
    pub subject: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileHistory {
    /// The non-merge commits that changed the file.
    pub count: u32,
    /// The newest and oldest of those commit times.
    pub newest: i64,
    pub oldest: i64,
    /// Up to [`RECENT`] commits, newest first, as indices into [`History::commits`].
    pub recent: Vec<usize>,
}

/// The history of the files in an index.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct History {
    /// Ages count whole days back from this time.
    pub now: i64,
    /// The commits that are some file's recent commits.
    pub commits: Vec<Commit>,
    files: HashMap<String, FileHistory>,
}

/// The current time in seconds since the epoch.
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// A clock time of day, `HH:MM:SS`, for seconds since the epoch at an offset from UTC in seconds.
pub fn clock(seconds: i64, offset: i32) -> String {
    let s = (seconds + i64::from(offset)).rem_euclid(86_400);
    format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

/// The local clock time now, `HH:MM:SS`, as the agent history shows it (unit `051`). UTC when the local zone is not known.
pub fn local_clock() -> String {
    let t = gix::date::Time::now_local_or_utc();
    clock(t.seconds, t.offset)
}

impl History {
    /// True when no file has history, as for an index JSON without git fields.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// A file's history. `None` for a file that no commit changed.
    pub fn file(&self, path: &str) -> Option<&FileHistory> {
        self.files.get(path)
    }

    /// A file's recent commits, newest first.
    pub fn recent<'a>(&'a self, f: &'a FileHistory) -> impl Iterator<Item = &'a Commit> + 'a {
        f.recent.iter().map(|&k| &self.commits[k])
    }

    /// Whole days from a time to [`History::now`], at least 0.
    pub fn age_days(&self, time: i64) -> i64 {
        (self.now - time).max(0) / DAY
    }

    /// Walks the history of the repo at `root` for `paths`, the repo-relative paths of the index.
    pub fn walk(root: &Path, paths: &[String], now: i64) -> Result<History> {
        let mut repo = gix::open(root)
            .with_context(|| format!("cannot open the repository at {}", root.display()))?;
        repo.object_cache_size_if_unset(64 * 1024 * 1024);
        let mut h = History {
            now,
            ..History::default()
        };
        let wanted: HashMap<&[u8], &str> =
            paths.iter().map(|p| (p.as_bytes(), p.as_str())).collect();
        let Ok(head) = repo.head_id() else {
            // An unborn branch has no commits.
            return Ok(h);
        };
        let walk = repo
            .rev_walk([head.detach()])
            .sorting(Sorting::ByCommitTime(CommitTimeOrder::NewestFirst))
            .all()?;
        let mut state = gix::diff::tree::State::default();
        for info in walk {
            let info = info?;
            if info.parent_ids.len() > 1 {
                continue;
            }
            let commit = info.object()?;
            let tree = commit.tree()?;
            let parent = match info.parent_ids.first() {
                Some(&p) => Some(repo.find_commit(p)?.tree()?),
                None => None,
            };
            let lhs = parent.as_ref().map_or(&[][..], |t| &t.data[..]);
            let mut changes = gix::diff::tree::Recorder::default();
            gix::diff::tree(
                gix::objs::TreeRefIter::from_bytes(lhs, repo.object_hash()),
                gix::objs::TreeRefIter::from_bytes(&tree.data, repo.object_hash()),
                &mut state,
                &repo.objects,
                &mut changes,
            )?;
            let mut commit_index = None;
            for change in &changes.records {
                let (path, mode) = match change {
                    Change::Addition {
                        path, entry_mode, ..
                    }
                    | Change::Deletion {
                        path, entry_mode, ..
                    }
                    | Change::Modification {
                        path, entry_mode, ..
                    } => (path, entry_mode),
                };
                if mode.is_tree() {
                    continue;
                }
                let Some(&p) = wanted.get(path.as_bytes()) else {
                    continue;
                };
                let time = commit.time()?.seconds;
                let f = h.files.entry(p.to_string()).or_default();
                if f.count == 0 {
                    f.newest = time;
                    f.oldest = time;
                }
                f.count += 1;
                f.newest = f.newest.max(time);
                f.oldest = f.oldest.min(time);
                if f.recent.len() < RECENT {
                    let k = match commit_index {
                        Some(k) => k,
                        None => {
                            let msg = commit.message_raw_sloppy();
                            let subject = msg.lines().next().unwrap_or_default().to_str_lossy();
                            h.commits.push(Commit {
                                hash: info.id.to_string(),
                                time,
                                subject: subject.trim().to_string(),
                            });
                            *commit_index.insert(h.commits.len() - 1)
                        }
                    };
                    f.recent.push(k);
                }
            }
        }
        Ok(h)
    }

    /// History from the `a0`, `a1`, `c`, and `r` fields and the `commits` map of `mockups/fake.json`,
    /// where ages are in days before `now`.
    pub fn from_json(text: &str, now: i64) -> Result<History> {
        #[derive(Deserialize)]
        struct J {
            files: Vec<F>,
            #[serde(default)]
            commits: HashMap<String, (String, i64, String)>,
        }
        #[derive(Deserialize)]
        struct F {
            p: String,
            a0: Option<i64>,
            a1: Option<i64>,
            c: Option<u32>,
            #[serde(default)]
            r: Vec<String>,
        }
        let j: J = serde_json::from_str(text).context("cannot parse the history JSON")?;
        let mut h = History {
            now,
            ..History::default()
        };
        let mut index_of: HashMap<String, usize> = HashMap::new();
        for f in j.files {
            let (Some(a0), Some(a1), Some(c)) = (f.a0, f.a1, f.c) else {
                continue;
            };
            let mut recent = Vec::new();
            for hash in f.r.iter().take(RECENT) {
                let Some((_, age, subject)) = j.commits.get(hash) else {
                    continue;
                };
                let k = *index_of.entry(hash.clone()).or_insert_with(|| {
                    h.commits.push(Commit {
                        hash: hash.clone(),
                        time: now - age * DAY,
                        subject: subject.clone(),
                    });
                    h.commits.len() - 1
                });
                recent.push(k);
            }
            let fh = FileHistory {
                count: c,
                newest: now - a1 * DAY,
                oldest: now - a0 * DAY,
                recent,
            };
            h.files.insert(f.p, fh);
        }
        Ok(h)
    }
}

/// A walk on a background thread. [`Loader::poll`] returns its result once.
pub struct Loader {
    rx: mpsc::Receiver<(Result<History>, Duration)>,
}

impl Loader {
    /// Starts [`History::walk`] on a new thread.
    pub fn spawn(root: PathBuf, paths: Vec<String>, now: i64) -> Loader {
        Loader::spawn_with(root, paths, now, || {})
    }

    /// Starts [`History::walk`] on a new thread, and runs `wake` once on that thread after the result is sent,
    /// so a window can repaint (unit `044`).
    pub fn spawn_with(
        root: PathBuf,
        paths: Vec<String>,
        now: i64,
        wake: impl FnOnce() + Send + 'static,
    ) -> Loader {
        let (tx, loader) = Loader::channel();
        std::thread::spawn(move || {
            let t0 = Instant::now();
            let result = History::walk(&root, &paths, now);
            let _ = tx.send((result, t0.elapsed()));
            wake();
        });
        loader
    }

    /// A loader that finishes when the sender sends, so tests can hold the loading state.
    pub fn channel() -> (mpsc::Sender<(Result<History>, Duration)>, Loader) {
        let (tx, rx) = mpsc::channel();
        (tx, Loader { rx })
    }

    /// The walk's result and duration when it has finished, else `None`.
    pub fn poll(&self) -> Option<(Result<History>, Duration)> {
        match self.rx.try_recv() {
            Ok(r) => Some(r),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some((
                Err(anyhow::anyhow!("the git history thread stopped")),
                Duration::ZERO,
            )),
        }
    }
}

/// A commit subject without its `type(scope)!:` prefix (`^[a-z]+(\([^)]*\))?!?:\s*`, case-insensitive).
pub fn describe(subject: &str) -> &str {
    let b = subject.as_bytes();
    let mut i = b.iter().take_while(|c| c.is_ascii_alphabetic()).count();
    if i == 0 {
        return subject;
    }
    if b.get(i) == Some(&b'(') {
        match b[i..].iter().position(|&c| c == b')') {
            Some(close) => i += close + 1,
            None => return subject,
        }
    }
    if b.get(i) == Some(&b'!') {
        i += 1;
    }
    if b.get(i) != Some(&b':') {
        return subject;
    }
    subject[i + 1..].trim_start()
}

/// An age in days: `today`, `Nd` below 14, `Nw` below 60, `Nmo` below 730, else `Ny` (`docs/design.md`).
pub fn ago(days: i64) -> String {
    // `Math.round`: halves round up.
    let round = |x: f64| (x + 0.5).floor() as i64;
    if days < 1 {
        "today".to_string()
    } else if days < 14 {
        format!("{days}d")
    } else if days < 60 {
        format!("{}w", round(days as f64 / 7.0))
    } else if days < 730 {
        format!("{}mo", round(days as f64 / 30.0))
    } else {
        format!("{}y", round(days as f64 / 365.0))
    }
}

/// `today`, or the age and `ago`.
pub fn ago_phrase(days: i64) -> String {
    if days < 1 {
        "today".to_string()
    } else {
        format!("{} ago", ago(days))
    }
}

/// A heat layer (`docs/design.md` `Layers`). Layers 4 and 5 are exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Heat {
    /// Days since the last change.
    Age,
    /// The commit count.
    Churn,
}

/// The age ramp: steps at 14, 60, and 365 days or fewer.
pub const AGE_STEPS: [i64; 3] = [14, 60, 365];
/// The churn ramp: steps at 60, 25, and 10 commits or more.
pub const CHURN_STEPS: [u32; 3] = [60, 25, 10];

impl Heat {
    /// The ramp step of a file, 0 for the strongest. `None` past the last step.
    pub fn step(self, h: &History, f: &FileHistory) -> Option<usize> {
        match self {
            Heat::Age => {
                let days = h.age_days(f.newest);
                AGE_STEPS.iter().position(|&d| days <= d)
            }
            Heat::Churn => CHURN_STEPS.iter().position(|&c| f.count >= c),
        }
    }

    /// The legend labels of the ramp steps.
    pub fn labels(self) -> [&'static str; 3] {
        match self {
            Heat::Age => ["<2w", "<2mo", "<1y"],
            Heat::Churn => ["60+", "25+", "10+"],
        }
    }
}
