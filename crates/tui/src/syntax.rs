//! Syntax colors for the file viewer (unit `029`): the core's highlighter (`minimap_core::highlight`, unit `055`)
//! with `ratatui` colors. Highlighting runs on one background thread, which loads the syntaxes once.
//! The thread also diffs the file against `HEAD` (unit `030`), and outlines files from their grammars'
//! definition scopes (unit `032`).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use minimap_core::Content;
use minimap_core::diff::FileDiff;
use minimap_core::highlight;
use minimap_core::outline::OutlineItem;
use ratatui::style::Color;
use syntect::parsing::SyntaxReference;

/// Files with more lines or bytes than this show plain text (`minimap_core::syntax`).
pub use minimap_core::syntax::{MAX_BYTES, MAX_LINES, OUTLINE_MAX_DEPTH, too_long};

/// A run of text in one style.
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub text: String,
    pub fg: Color,
    pub bold: bool,
    pub italic: bool,
}

/// A file's colored lines, and its diff against `HEAD`.
#[derive(Debug, Clone, Default)]
pub struct Colored {
    pub path: String,
    /// The syntax's name, such as `Clojure`, or `Plain Text`.
    pub syntax: String,
    /// One list of spans per line, without the line ends. Empty for a file too long for colors.
    pub lines: Vec<Vec<Span>>,
    pub took: Duration,
    /// The uncommitted changes against `HEAD`. `None` without a `HEAD` to read.
    pub diff: Option<Arc<FileDiff>>,
    /// The `HEAD` text's lines, for removed rows: colored when the file is, and plain.
    pub head_lines: Vec<Vec<Span>>,
    pub head_text: Arc<Vec<String>>,
    /// The working-tree text this was made from, so a changed file is prepared again.
    pub text: Option<Arc<String>>,
    /// The outline from the grammar's definition scopes (unit `032`).
    pub outline: Arc<Vec<OutlineItem>>,
}

/// The core's highlighter (unit `055`), with `ratatui` colors.
pub struct Highlighter {
    inner: highlight::Highlighter,
}

fn color(rgb: u32) -> Color {
    let (r, g, b) = minimap_core::palette::rgb(rgb);
    Color::Rgb(r, g, b)
}

impl Highlighter {
    /// Loads the syntaxes and the theme at `theme`, or the built-in theme.
    /// A theme that fails to load gives the built-in theme and the error.
    pub fn new(theme: Option<&Path>) -> (Highlighter, Option<String>) {
        let (inner, err) = highlight::Highlighter::new(theme);
        (Highlighter { inner }, err)
    }

    /// The syntax for a path: by file name (such as `Makefile`), then extension, then the first line; else plain text.
    pub fn syntax(&self, path: &str, first_line: &str) -> &SyntaxReference {
        self.inner.syntax(path, first_line)
    }

    /// The theme's default text color.
    pub fn foreground(&self) -> Option<Color> {
        self.inner.foreground().map(color)
    }

    /// Colors every line of a text, and outlines it, in one parse.
    /// Only foregrounds, bold, and italic are kept; the pane keeps its background.
    pub fn highlight(&self, path: &str, text: &str) -> Colored {
        let h = self.inner.highlight(path, text);
        Colored {
            path: path.to_string(),
            syntax: h.syntax,
            lines: h
                .lines
                .into_iter()
                .map(|spans| {
                    spans
                        .into_iter()
                        .map(|s| Span {
                            text: s.text,
                            fg: color(s.rgb),
                            bold: s.bold,
                            italic: s.italic,
                        })
                        .collect()
                })
                .collect(),
            took: h.took,
            outline: Arc::new(h.outline),
            ..Colored::default()
        }
    }

    /// The outline of a text from its grammar's definition scopes (unit `032`), without colors.
    pub fn outline(&self, path: &str, text: &str) -> Vec<OutlineItem> {
        self.inner.outline(path, text)
    }
}

/// What the background thread sends back.
pub enum Done {
    /// The theme's text color, once the thread has loaded the theme.
    Foreground(Color),
    ThemeError(String),
    Colored(Arc<Colored>),
    /// An outline-only job's outline, by path.
    Outline(String, Arc<Vec<OutlineItem>>),
}

/// One file for the thread: its text, whether to color it, and where to read its `HEAD` text.
pub struct Job {
    pub path: String,
    pub text: Arc<String>,
    pub color: bool,
    pub head: Content,
    /// Only the outline, for a selected file that the viewer does not show (unit `032`).
    pub outline_only: bool,
}

/// Colors a job's text and diffs it against `HEAD`. A path that `HEAD` does not have is all added.
fn prepare(h: &Highlighter, job: &Job) -> Colored {
    let mut c = if job.color {
        h.highlight(&job.path, &job.text)
    } else {
        Colored {
            path: job.path.clone(),
            syntax: "Plain Text".to_string(),
            ..Colored::default()
        }
    };
    c.text = Some(job.text.clone());
    if job.head.is_none() {
        return c;
    }
    let Ok(head) = job.head.head(&job.path) else {
        return c;
    };
    let head = head.unwrap_or_default();
    let diff = FileDiff::compute(&head, &job.text);
    if job.color && !diff.is_empty() {
        c.head_lines = h.highlight(&job.path, &head).lines;
    }
    c.head_text = Arc::new(head.lines().map(str::to_string).collect());
    c.diff = Some(Arc::new(diff));
    c
}

/// The highlighting thread. It loads the syntaxes and the theme when it starts.
pub struct Worker {
    jobs: Sender<Job>,
    done: Receiver<Done>,
}

impl Worker {
    pub fn spawn(theme: Option<PathBuf>) -> Worker {
        let (jobs, job_rx) = mpsc::channel::<Job>();
        let (done_tx, done) = mpsc::channel();
        thread::spawn(move || {
            let (h, err) = Highlighter::new(theme.as_deref());
            if let Some(e) = err {
                let _ = done_tx.send(Done::ThemeError(e));
            }
            if let Some(fg) = h.foreground() {
                let _ = done_tx.send(Done::Foreground(fg));
            }
            for job in job_rx {
                let done = if job.outline_only {
                    Done::Outline(job.path.clone(), Arc::new(h.outline(&job.path, &job.text)))
                } else {
                    Done::Colored(Arc::new(prepare(&h, &job)))
                };
                if done_tx.send(done).is_err() {
                    break;
                }
            }
        });
        Worker { jobs, done }
    }

    pub fn request(&self, job: Job) {
        let _ = self.jobs.send(job);
    }

    /// Everything the thread has finished since the last poll.
    pub fn poll(&self) -> Vec<Done> {
        self.done.try_iter().collect()
    }

    /// Waits for the next result, for tests.
    pub fn wait(&self, timeout: Duration) -> Option<Done> {
        self.done.recv_timeout(timeout).ok()
    }
}
