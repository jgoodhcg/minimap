//! File details for the selection panel: the line shape, the outline, and the namespace (unit `007`).
//!
//! [`Details`] computes them on demand, one file at a time, and caches them in memory. Nothing is computed at startup.
//! [`Loader`] computes many files on a pool of background threads, in a priority order that the caller can change (unit `036`).

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread::{self, JoinHandle};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::index::{IndexFile, is_binary};
use crate::outline::{self, OutlineItem, language_for};
use crate::shape::{self, LineShape, MEDIAN_ABOVE, ShapeRow};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileDetails {
    /// None for binary, missing, and unreadable files.
    pub shape: Option<LineShape>,
    /// Every line, measured (unit `039`): 4 bytes per line, for a renderer that draws each line.
    /// Empty when the details come from JSON, which has only the sampled shape.
    pub measured: Vec<ShapeRow>,
    pub outline: Vec<OutlineItem>,
    pub namespace: Option<String>,
}

impl FileDetails {
    /// The details of a file's content. Binary content has none.
    pub fn compute(path: &str, bytes: &[u8]) -> FileDetails {
        if is_binary(bytes) {
            return FileDetails::default();
        }
        let text = String::from_utf8_lossy(bytes);
        let lang = language_for(path);
        let measured = shape::measure_lines(&text);
        FileDetails {
            shape: Some(LineShape::sample(&measured, MEDIAN_ABOVE)),
            measured,
            outline: lang.map(|l| l.outline(&text)).unwrap_or_default(),
            namespace: lang.and_then(|l| l.namespace(&text)),
        }
    }
}

#[derive(Debug, Default)]
enum Source {
    /// No details, for an index without a repo or details in its JSON.
    #[default]
    None,
    /// Read files from this working tree.
    Repo(PathBuf),
    /// Compute details from text by path, for tests.
    Memory(HashMap<String, String>),
    /// Details from `mockups/fake.json`, by path.
    Preloaded(HashMap<String, FileDetails>),
}

/// Where file details come from, and the cache of computed ones.
#[derive(Debug, Default)]
pub struct Details {
    source: Source,
    cache: HashMap<String, FileDetails>,
}

/// The fields of `mockups/fake.json` that carry details. Unknown fields are ignored.
#[derive(Deserialize)]
struct DetailsJson {
    files: Vec<FileJson>,
}

#[derive(Deserialize)]
struct FileJson {
    p: String,
    #[serde(default)]
    n: u64,
    m: Option<String>,
    #[serde(default)]
    o: Vec<(u32, String, String)>,
    ns: Option<String>,
}

impl Details {
    /// Details read from the files of a working tree.
    pub fn repo(root: PathBuf) -> Details {
        Details {
            source: Source::Repo(root),
            cache: HashMap::new(),
        }
    }

    /// Details computed from text held in memory, by path.
    pub fn memory(files: HashMap<String, String>) -> Details {
        Details {
            source: Source::Memory(files),
            cache: HashMap::new(),
        }
    }

    /// Details from the `m`, `o`, and `ns` fields of an index JSON, by path. A file without them has none.
    pub fn from_json(text: &str) -> Result<Details> {
        let j: DetailsJson = serde_json::from_str(text).context("cannot parse the details JSON")?;
        let mut map = HashMap::new();
        for f in j.files {
            if f.m.is_none() && f.o.is_empty() && f.ns.is_none() {
                continue;
            }
            let outline =
                f.o.into_iter()
                    .map(|(line, kind, name)| OutlineItem {
                        line,
                        private: kind.ends_with('-'),
                        kind,
                        name,
                        depth: 0,
                    })
                    .collect();
            let details = FileDetails {
                shape: f.m.and_then(|m| LineShape::decode(&m, f.n)),
                measured: Vec::new(),
                outline,
                namespace: f.ns,
            };
            map.insert(f.p, details);
        }
        Ok(Details {
            source: Source::Preloaded(map),
            cache: HashMap::new(),
        })
    }

    /// Computes a file's details once, if the source needs it.
    pub fn load(&mut self, file: &IndexFile) {
        if self.cache.contains_key(&file.path) {
            return;
        }
        let bytes = match &self.source {
            Source::None | Source::Preloaded(_) => return,
            _ if file.binary || file.missing => None,
            Source::Repo(root) => fs::read(root.join(&file.path)).ok(),
            Source::Memory(files) => files.get(&file.path).map(|t| t.clone().into_bytes()),
        };
        let details = bytes.map_or_else(FileDetails::default, |b| {
            FileDetails::compute(&file.path, &b)
        });
        self.cache.insert(file.path.clone(), details);
    }

    /// Drops a file's computed details, so the next [`Details::load`] reads the file again (unit `034`).
    pub fn forget(&mut self, path: &str) {
        self.cache.remove(path);
    }

    /// A file's details, after [`Details::load`].
    pub fn get(&self, path: &str) -> Option<&FileDetails> {
        match &self.source {
            Source::None => None,
            Source::Repo(_) | Source::Memory(_) => self.cache.get(path),
            Source::Preloaded(map) => map.get(path),
        }
    }
}

/// A file's details for the [`Loader`]: [`FileDetails::compute`], and the grammar outline ([`outline::generic`])
/// for a text file in a language without a tree-sitter outline.
pub fn compute_file(path: &str, bytes: &[u8]) -> FileDetails {
    let mut d = FileDetails::compute(path, bytes);
    if d.shape.is_some() && language_for(path).is_none() {
        d.outline = outline::generic(path, &String::from_utf8_lossy(bytes));
    }
    d
}

/// The loader's queue, shared with its workers.
#[derive(Default)]
struct Queue {
    /// Paths not yet started, highest priority first.
    pending: VecDeque<String>,
    running: HashSet<String>,
    /// Paths whose details were computed and not forgotten.
    done: HashSet<String>,
    stop: bool,
}

struct Shared {
    queue: Mutex<Queue>,
    ready: Condvar,
}

/// How a worker reads a file. `None` gives empty details, as for a missing file.
type Read = Arc<dyn Fn(&str) -> Option<Vec<u8>> + Send + Sync>;

/// Computes details for many files on background threads (unit `036`).
///
/// [`Loader::request`] sets which files to compute and in what order. [`Loader::poll`] takes the results,
/// and [`Loader::get`] reads them after that. The `wake` callback runs once per result, on the thread that made it,
/// so a GUI can ask for a repaint.
pub struct Loader {
    shared: Arc<Shared>,
    workers: Vec<JoinHandle<()>>,
    rx: mpsc::Receiver<(String, FileDetails)>,
    /// Preloaded details answer requests at once, with no workers.
    preloaded: Option<HashMap<String, FileDetails>>,
    /// Preloaded results that [`Loader::poll`] has not returned yet.
    instant: Vec<(String, FileDetails)>,
    cache: HashMap<String, FileDetails>,
    wake: Arc<dyn Fn() + Send + Sync>,
}

impl Loader {
    /// A loader with `available_parallelism - 1` workers, at least 1, over the source of `details`:
    /// a working tree, text in memory, or preloaded details.
    pub fn spawn(details: Details, wake: impl Fn() + Send + Sync + 'static) -> Loader {
        let n = thread::available_parallelism().map_or(1, |n| n.get().saturating_sub(1).max(1));
        Loader::with_workers(details, wake, n)
    }

    /// A loader with `workers` threads. With 0, requests wait in the queue, for tests.
    pub fn with_workers(
        details: Details,
        wake: impl Fn() + Send + Sync + 'static,
        workers: usize,
    ) -> Loader {
        let shared = Arc::new(Shared {
            queue: Mutex::new(Queue::default()),
            ready: Condvar::new(),
        });
        let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(wake);
        let (tx, rx) = mpsc::channel();
        let (read, preloaded): (Option<Read>, _) = match details.source {
            Source::None => (None, Some(HashMap::new())),
            Source::Preloaded(map) => (None, Some(map)),
            Source::Repo(root) => (
                Some(Arc::new(move |p: &str| fs::read(root.join(p)).ok())),
                None,
            ),
            Source::Memory(files) => (
                Some(Arc::new(move |p: &str| {
                    files.get(p).map(|t| t.clone().into_bytes())
                })),
                None,
            ),
        };
        let workers = match read {
            None => Vec::new(),
            Some(read) => (0..workers)
                .map(|_| {
                    let (shared, read, tx, wake) =
                        (shared.clone(), read.clone(), tx.clone(), wake.clone());
                    thread::spawn(move || work(&shared, &read, &tx, &*wake))
                })
                .collect(),
        };
        Loader {
            shared,
            workers,
            rx,
            preloaded,
            instant: Vec::new(),
            cache: HashMap::new(),
            wake,
        }
    }

    /// Replaces the paths not yet started with `paths`, in this priority order.
    /// Paths that are done or in progress are skipped.
    pub fn request(&mut self, paths: impl IntoIterator<Item = String>) {
        if let Some(map) = &self.preloaded {
            let mut q = self.shared.queue.lock().expect("the loader queue");
            for p in paths {
                if q.done.insert(p.clone()) {
                    let d = map.get(&p).cloned().unwrap_or_default();
                    self.instant.push((p, d));
                    (self.wake)();
                }
            }
            return;
        }
        let mut q = self.shared.queue.lock().expect("the loader queue");
        let mut seen = HashSet::new();
        let pending: VecDeque<String> = paths
            .into_iter()
            .filter(|p| !q.done.contains(p) && !q.running.contains(p) && seen.insert(p.clone()))
            .collect();
        q.pending = pending;
        drop(q);
        self.shared.ready.notify_all();
    }

    /// The finished results since the last poll. They stay readable with [`Loader::get`].
    pub fn poll(&mut self) -> Vec<(String, FileDetails)> {
        let mut out = std::mem::take(&mut self.instant);
        out.extend(self.rx.try_iter());
        for (p, d) in &out {
            self.cache.insert(p.clone(), d.clone());
        }
        out
    }

    /// A file's details, once [`Loader::poll`] has taken them.
    pub fn get(&self, path: &str) -> Option<&FileDetails> {
        self.cache.get(path)
    }

    /// Drops a file's result, so a later request computes it again (for the live index).
    pub fn forget(&mut self, path: &str) {
        self.cache.remove(path);
        self.shared
            .queue
            .lock()
            .expect("the loader queue")
            .done
            .remove(path);
    }

    /// The paths not yet started, in order.
    pub fn pending(&self) -> Vec<String> {
        let q = self.shared.queue.lock().expect("the loader queue");
        q.pending.iter().cloned().collect()
    }

    /// True while paths wait or run.
    pub fn busy(&self) -> bool {
        let q = self.shared.queue.lock().expect("the loader queue");
        !q.pending.is_empty() || !q.running.is_empty()
    }
}

impl Drop for Loader {
    /// Stops the workers after their current file, and waits for them.
    fn drop(&mut self) {
        if let Ok(mut q) = self.shared.queue.lock() {
            q.stop = true;
            q.pending.clear();
        }
        self.shared.ready.notify_all();
        for w in self.workers.drain(..) {
            let _ = w.join();
        }
    }
}

/// A worker: takes the first pending path, computes it, sends it, and wakes the caller, until the loader stops.
fn work(
    shared: &Shared,
    read: &Read,
    tx: &mpsc::Sender<(String, FileDetails)>,
    wake: &(dyn Fn() + Send + Sync),
) {
    loop {
        let path = {
            let mut q = shared.queue.lock().expect("the loader queue");
            loop {
                if q.stop {
                    return;
                }
                if let Some(p) = q.pending.pop_front() {
                    q.running.insert(p.clone());
                    break p;
                }
                q = shared.ready.wait(q).expect("the loader queue");
            }
        };
        let details = read(&path).map_or_else(FileDetails::default, |b| compute_file(&path, &b));
        {
            let mut q = shared.queue.lock().expect("the loader queue");
            q.running.remove(&path);
            q.done.insert(path.clone());
        }
        if tx.send((path, details)).is_err() {
            return;
        }
        wake();
    }
}
