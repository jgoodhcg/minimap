//! The event socket (unit `010`): agents send JSON-line events about the files they read, search, and edit.
//!
//! ```json
//! {"v":1,"type":"read","path":"src/a.clj","source":"claude-code","session":"0f9e8d7c-…"}
//! {"v":1,"type":"grep","paths":["src/a.clj","src/b.clj"],"source":"claude-code"}
//! {"v":1,"type":"edit","path":"src/a.clj","lines":[40,58],"source":"claude-code"}
//! {"v":1,"type":"clear","layer":"trail"}
//! {"v":1,"type":"note","path":"src/a.clj","lines":[40,58],"text":"Retries start here."}
//! {"v":1,"type":"clear","layer":"notes"}
//! ```
//!
//! The socket is agent-neutral: Claude Code is one adapter (unit `011`), and others can send the same lines.
//! `source` and `session` (unit `050`) are optional. A parser reads only the fields it knows, so a new field needs no new version.
//! A note (unit `052`) points a person at a file or its lines, with an optional text; `minimap note` sends one.

use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use serde_json::{Value, json};

/// A trail event's kind, weakest first: on one path, an edit wins over a read, and a read over a grep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    Grep,
    Read,
    Edit,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Grep => "grep",
            Kind::Read => "read",
            Kind::Edit => "edit",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// An agent read, found by search, or edited these paths.
    Trail {
        kind: Kind,
        /// One path for a read or an edit; the matched paths for a grep.
        paths: Vec<String>,
        /// An edit's first and last line, 1-based.
        lines: Option<(u32, u32)>,
        /// The sender, such as `claude-code`.
        source: Option<String>,
        /// The sender's session id, such as Claude Code's `session_id` (unit `050`).
        session: Option<String>,
    },
    /// Empty the trail.
    Clear,
    /// An agent's note about a file or its lines (unit `052`).
    Note {
        path: String,
        /// The first and last noted line, 1-based.
        lines: Option<(u32, u32)>,
        /// The note's text, at most [`NOTE_TEXT_MAX`] characters. Empty for a mark without words.
        text: String,
        source: Option<String>,
        session: Option<String>,
    },
    /// Empty the notes.
    ClearNotes,
}

/// A note's longest text, in characters. A longer text is cut, and ends with `…`.
pub const NOTE_TEXT_MAX: usize = 4000;

/// A note's text cut to [`NOTE_TEXT_MAX`] characters.
pub fn note_text(s: &str) -> String {
    if s.chars().count() <= NOTE_TEXT_MAX {
        return s.to_string();
    }
    let mut t: String = s.chars().take(NOTE_TEXT_MAX - 1).collect();
    t.push('…');
    t
}

impl Event {
    /// A read, grep, or edit event without lines, a source, or a session.
    pub fn trail(kind: Kind, paths: Vec<String>) -> Event {
        Event::Trail {
            kind,
            paths,
            lines: None,
            source: None,
            session: None,
        }
    }

    /// The event as one JSON line, without the newline.
    pub fn to_line(&self) -> String {
        let v = match self {
            Event::Clear => json!({"v": 1, "type": "clear", "layer": "trail"}),
            Event::ClearNotes => json!({"v": 1, "type": "clear", "layer": "notes"}),
            Event::Note {
                path,
                lines,
                text,
                source,
                session,
            } => {
                let mut v = json!({"v": 1, "type": "note", "path": path});
                if let Some((a, b)) = lines {
                    v["lines"] = json!([a, b]);
                }
                if !text.is_empty() {
                    v["text"] = json!(text);
                }
                if let Some(s) = source {
                    v["source"] = json!(s);
                }
                if let Some(s) = session {
                    v["session"] = json!(s);
                }
                v
            }
            Event::Trail {
                kind,
                paths,
                lines,
                source,
                session,
            } => {
                let mut v = json!({"v": 1, "type": kind.name()});
                match kind {
                    Kind::Grep => v["paths"] = json!(paths),
                    _ => v["path"] = json!(paths.first().cloned().unwrap_or_default()),
                }
                if let Some((a, b)) = lines {
                    v["lines"] = json!([a, b]);
                }
                if let Some(s) = source {
                    v["source"] = json!(s);
                }
                if let Some(s) = session {
                    v["session"] = json!(s);
                }
                v
            }
        };
        v.to_string()
    }

    /// The event with repo-relative paths (see [`normalize`]). Paths outside the repo drop out,
    /// and a trail event without paths left, or a note outside the repo, is `None`.
    pub fn normalized(self, root: &Path, canonical: &Path) -> Option<Event> {
        match self {
            Event::Clear => Some(Event::Clear),
            Event::ClearNotes => Some(Event::ClearNotes),
            Event::Note {
                path,
                lines,
                text,
                source,
                session,
            } => Some(Event::Note {
                path: normalize(root, canonical, &path)?,
                lines,
                text,
                source,
                session,
            }),
            Event::Trail {
                kind,
                paths,
                lines,
                source,
                session,
            } => {
                let paths: Vec<String> = paths
                    .iter()
                    .filter_map(|p| normalize(root, canonical, p))
                    .collect();
                (!paths.is_empty()).then_some(Event::Trail {
                    kind,
                    paths,
                    lines,
                    source,
                    session,
                })
            }
        }
    }
}

/// Two 1-based lines, in order, from a JSON `[a, b]`. `None` for anything else.
fn line_pair(v: &Value) -> Option<(u32, u32)> {
    let l = v.as_array().filter(|l| l.len() == 2)?;
    let a = u32::try_from(l[0].as_u64()?).ok()?;
    let b = u32::try_from(l[1].as_u64()?).ok()?;
    Some((a.min(b), a.max(b)))
}

/// Parses one line. A line with a `v` other than 1, an unknown type, or a missing path is `None`.
pub fn parse(line: &str) -> Option<Event> {
    let v: Value = serde_json::from_str(line.trim()).ok()?;
    if v.get("v")?.as_u64()? != 1 {
        return None;
    }
    let text = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
    let (source, session) = (text("source"), text("session"));
    let path = || {
        v.get("path")
            .and_then(Value::as_str)
            .filter(|p| !p.is_empty())
            .map(|p| vec![p.to_string()])
    };
    let (kind, paths) = match v.get("type")?.as_str()? {
        "read" => (Kind::Read, path()?),
        "edit" => (Kind::Edit, path()?),
        "grep" => {
            let paths: Vec<String> = v
                .get("paths")?
                .as_array()?
                .iter()
                .filter_map(Value::as_str)
                .filter(|p| !p.is_empty())
                .map(str::to_string)
                .collect();
            if paths.is_empty() {
                return None;
            }
            (Kind::Grep, paths)
        }
        "clear" => {
            return match v.get("layer").map(Value::as_str) {
                None | Some(Some("trail")) => Some(Event::Clear),
                Some(Some("notes")) => Some(Event::ClearNotes),
                _ => None,
            };
        }
        "note" => {
            let path = path()?.remove(0);
            // A note's lines that are not two numbers are left out, and the note stays.
            let lines = v.get("lines").and_then(line_pair);
            return Some(Event::Note {
                path,
                lines,
                text: note_text(&text("text").unwrap_or_default()),
                source,
                session,
            });
        }
        _ => return None,
    };
    let lines = match (kind, v.get("lines").and_then(Value::as_array)) {
        (Kind::Edit, Some(l)) if l.len() == 2 => Some(line_pair(&v["lines"])?),
        _ => None,
    };
    Some(Event::Trail {
        kind,
        paths,
        lines,
        source,
        session,
    })
}

/// A path as a repo-relative path with `/` separators, or `None` for a path outside the repo.
/// An absolute path matches the root in its given form or its canonical form (macOS `/var` is `/private/var`),
/// and else through the canonical form of its nearest existing directory. `.` and `..` resolve by text.
pub fn normalize(root: &Path, canonical: &Path, path: &str) -> Option<String> {
    let p = Path::new(path);
    if p.is_relative() {
        return relative(&lexical(p)?);
    }
    let p = lexical(p)?;
    for r in [root, canonical] {
        if let Ok(rest) = p.strip_prefix(lexical(r)?) {
            return relative(rest);
        }
    }
    // The path can name the root through a link: resolve the nearest directory that exists.
    let mut dir = p.as_path();
    let mut tail = Vec::new();
    while let Some(parent) = dir.parent() {
        tail.push(dir.file_name()?);
        if let Ok(real) = fs::canonicalize(parent) {
            let rest = real.strip_prefix(canonical).ok()?;
            let mut out = rest.to_path_buf();
            out.extend(tail.iter().rev());
            return relative(&out);
        }
        dir = parent;
    }
    None
}

/// Resolves `.` and `..` by text. `None` when `..` leaves a relative path's start.
fn lexical(p: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            other => out.push(other),
        }
    }
    Some(out)
}

/// A relative path as a repo-relative string. The root itself and paths that leave it are `None`.
fn relative(p: &Path) -> Option<String> {
    let mut parts = Vec::new();
    for c in p.components() {
        match c {
            Component::Normal(s) => parts.push(s.to_string_lossy().into_owned()),
            Component::CurDir => {}
            _ => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// The 64-bit FNV-1a hash of a string, for socket names.
pub fn fnv1a(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// macOS `sun_path` holds 104 bytes, with a NUL at the end.
pub const SOCKET_PATH_MAX: usize = 103;

/// The event socket of a repo: `$TMPDIR/minimap-$USER-<16 hex digits of the FNV-1a hash of the canonical root>-events.sock`,
/// in `/tmp` when that path is too long for a socket.
pub fn socket_path(root: &Path) -> PathBuf {
    let user = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
    let canonical = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    socket_path_in(&std::env::temp_dir(), &user, &canonical)
}

/// [`socket_path`] with the temp directory, the user, and the canonical root given.
pub fn socket_path_in(tmp: &Path, user: &str, canonical_root: &Path) -> PathBuf {
    let hash = fnv1a(&canonical_root.to_string_lossy());
    let name = format!("minimap-{user}-{hash:016x}-events.sock");
    let path = tmp.join(&name);
    if path.as_os_str().len() > SOCKET_PATH_MAX {
        Path::new("/tmp").join(name)
    } else {
        path
    }
}

/// Why a listener did not start.
#[derive(Debug)]
pub enum ListenError {
    /// Another minimap listens on this repo's socket.
    AlreadyListening,
    Io(io::Error),
}

impl std::fmt::Display for ListenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListenError::AlreadyListening => write!(f, "another minimap listens for this repo"),
            ListenError::Io(e) => write!(f, "cannot listen for events: {e}"),
        }
    }
}

impl std::error::Error for ListenError {}

/// How long the listener waits for a connection's next line before it takes the next connection.
const READ_TIMEOUT: Duration = Duration::from_millis(200);

/// The socket of one repo, read on a thread. Dropping it removes the socket file.
pub struct Listener {
    rx: mpsc::Receiver<Event>,
    path: PathBuf,
    stop: Arc<AtomicBool>,
}

impl Listener {
    /// Binds the repo's socket with mode 0600 and reads events on a thread. Paths become repo-relative.
    pub fn spawn(root: &Path) -> Result<Listener, ListenError> {
        Listener::spawn_with(root, || {})
    }

    /// As [`Listener::spawn`], and `wake` runs on the listener's thread after each event is queued, so a window can repaint (unit `047`).
    pub fn spawn_with(
        root: &Path,
        wake: impl Fn() + Send + 'static,
    ) -> Result<Listener, ListenError> {
        let path = socket_path(root);
        if path.exists() {
            if UnixStream::connect(&path).is_ok() {
                return Err(ListenError::AlreadyListening);
            }
            // Nobody listens: the file is left from a minimap that stopped.
            fs::remove_file(&path).map_err(ListenError::Io)?;
        }
        let socket = match UnixListener::bind(&path) {
            Ok(s) => s,
            // Another minimap bound it first.
            Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
                return Err(ListenError::AlreadyListening);
            }
            Err(e) => return Err(ListenError::Io(e)),
        };
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).map_err(ListenError::Io)?;
        let root = root.to_path_buf();
        let canonical = fs::canonicalize(&root).unwrap_or_else(|_| root.clone());
        let (tx, rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        std::thread::spawn(move || {
            for conn in socket.incoming() {
                if stopped.load(Ordering::Relaxed) {
                    break;
                }
                let Ok(conn) = conn else {
                    continue;
                };
                let _ = conn.set_read_timeout(Some(READ_TIMEOUT));
                for line in BufReader::new(conn).lines() {
                    let Ok(line) = line else {
                        break;
                    };
                    let event = parse(&line).and_then(|e| e.normalized(&root, &canonical));
                    if let Some(e) = event {
                        if tx.send(e).is_err() {
                            return;
                        }
                        wake();
                    }
                }
            }
        });
        Ok(Listener { rx, path, stop })
    }

    /// The events that arrived since the last poll, in order.
    pub fn poll(&self) -> Vec<Event> {
        self.rx.try_iter().collect()
    }

    /// Waits for the next event, for tests.
    pub fn wait(&self, timeout: Duration) -> Option<Event> {
        self.rx.recv_timeout(timeout).ok()
    }

    /// The socket file.
    pub fn socket(&self) -> &Path {
        &self.path
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // A connection wakes the thread, which sees the stop flag and closes the socket.
        let _ = UnixStream::connect(&self.path);
        let _ = fs::remove_file(&self.path);
    }
}

/// How long [`send`] waits to write its line.
pub const SEND_TIMEOUT: Duration = Duration::from_millis(50);

/// Sends one event to the repo's socket. Callers decide whether to be silent about an error.
pub fn send(root: &Path, event: &Event) -> io::Result<()> {
    let mut conn = UnixStream::connect(socket_path(root))?;
    conn.set_write_timeout(Some(SEND_TIMEOUT))?;
    let mut line = event.to_line();
    line.push('\n');
    conn.write_all(line.as_bytes())
}
