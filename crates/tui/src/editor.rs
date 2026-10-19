//! Editor handoff (unit `031`): `e` opens a file at a line in the user's editor.
//!
//! Editors are adapters. nvim runs in a reused herdr tab or split, and later opens go to its `--listen` socket.
//! vim runs in the same kind of place and gets `:drop` typed into it. VS Code, Emacs, and custom commands run detached.
//! Without herdr, terminal editors take over minimap's terminal until they exit.
//!
//! The command lines are built by plain functions, and [`Editor::open`] runs them through a [`Runner`],
//! so tests check every step without starting a process.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use minimap_core::config::EditorConfig;

/// The editor for `e`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Preset {
    Nvim,
    Vim,
    Vscode,
    Emacs,
    /// A command template. With `terminal`, it takes over minimap's terminal until it exits.
    Custom {
        command: Vec<String>,
        terminal: bool,
    },
}

impl Preset {
    pub fn name(&self) -> &str {
        match self {
            Preset::Nvim => "nvim",
            Preset::Vim => "vim",
            Preset::Vscode => "VS Code",
            Preset::Emacs => "Emacs",
            Preset::Custom { command, .. } => command.first().map_or("the editor", String::as_str),
        }
    }

    /// The preset for an editor command such as `$EDITOR`: by the file name of its first word.
    pub fn from_command(cmd: &str) -> Option<Preset> {
        let first = cmd.split_whitespace().next()?;
        let name = Path::new(first).file_name()?.to_str()?;
        match name {
            "nvim" => Some(Preset::Nvim),
            "vim" | "vi" => Some(Preset::Vim),
            "code" => Some(Preset::Vscode),
            "emacs" | "emacsclient" => Some(Preset::Emacs),
            _ => None,
        }
    }
}

/// Where herdr opens a terminal editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    Tab,
    Right,
    Down,
}

/// The editor settings, from the `[editor]` config and the environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub preset: Preset,
    pub place: Place,
}

impl Settings {
    /// Resolves the config. With no preset, `$VISUAL`, then `$EDITOR`, name one; else nvim.
    pub fn resolve(
        cfg: &EditorConfig,
        visual: Option<&str>,
        editor: Option<&str>,
    ) -> Result<Settings, String> {
        let preset = match cfg.preset.as_deref() {
            None => [visual, editor]
                .into_iter()
                .flatten()
                .find_map(Preset::from_command)
                .unwrap_or(Preset::Nvim),
            Some("nvim") => Preset::Nvim,
            Some("vim") => Preset::Vim,
            Some("vscode") => Preset::Vscode,
            Some("emacs") => Preset::Emacs,
            Some("custom") => match &cfg.command {
                Some(c) if !c.is_empty() => Preset::Custom {
                    command: c.clone(),
                    terminal: cfg.terminal.unwrap_or(false),
                },
                _ => return Err("[editor] preset = \"custom\" needs a command.".to_string()),
            },
            Some(other) => {
                return Err(format!(
                    "Unknown [editor] preset {other:?}: use nvim, vim, vscode, emacs, or custom."
                ));
            }
        };
        let place = match cfg.place.as_deref() {
            None | Some("tab") => Place::Tab,
            Some("right") => Place::Right,
            Some("down") => Place::Down,
            Some(other) => {
                return Err(format!(
                    "Unknown [editor] place {other:?}: use tab, right, or down."
                ));
            }
        };
        Ok(Settings { preset, place })
    }
}

/// The herdr pane and workspace that minimap runs in, from `HERDR_ENV`, `HERDR_WORKSPACE_ID`, and `HERDR_PANE_ID`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Herdr {
    pub workspace: String,
    pub pane: String,
}

impl Herdr {
    pub fn from_env() -> Option<Herdr> {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        if var("HERDR_ENV").as_deref() != Some("1") {
            return None;
        }
        Some(Herdr {
            workspace: var("HERDR_WORKSPACE_ID")?,
            pane: var("HERDR_PANE_ID")?,
        })
    }
}

/// One file to open: an absolute path, a 1-based line and column, and the repo root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub path: PathBuf,
    pub line: usize,
    pub col: usize,
    pub root: PathBuf,
}

impl Request {
    fn path_str(&self) -> String {
        self.path.to_string_lossy().into_owned()
    }
}

/// What an open did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The editor has the file.
    Opened,
    /// Run this command in minimap's terminal: leave the screen, wait for the command to exit, then come back.
    Suspend(Vec<String>),
}

/// Runs commands. The binary uses [`System`]; tests record the commands and script the results.
pub trait Runner {
    /// Runs a command, waits, and returns its stdout. A failed command returns its first stderr line.
    fn run(&mut self, argv: &[String]) -> Result<String, String>;
    /// Starts a command and does not wait for it.
    fn spawn(&mut self, argv: &[String]) -> Result<(), String>;
    /// Removes a stale socket file.
    fn remove_file(&mut self, path: &Path);
}

/// The real runner.
pub struct System;

impl Runner for System {
    fn run(&mut self, argv: &[String]) -> Result<String, String> {
        let out = Command::new(&argv[0])
            .args(&argv[1..])
            .stdin(Stdio::null())
            .output()
            .map_err(|e| format!("{}: {e}", argv[0]))?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).into_owned())
        } else {
            let err = String::from_utf8_lossy(&out.stderr);
            let first = err.lines().next().unwrap_or("").trim().to_string();
            Err(if first.is_empty() {
                format!("{} failed ({})", argv[0], out.status)
            } else {
                format!("{}: {first}", argv[0])
            })
        }
    }

    fn spawn(&mut self, argv: &[String]) -> Result<(), String> {
        Command::new(&argv[0])
            .args(&argv[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("{}: {e}", argv[0]))
    }

    fn remove_file(&mut self, path: &Path) {
        let _ = std::fs::remove_file(path);
    }
}

/// The editor's place for the session: its herdr pane and, for a tab, its tab.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlaceState {
    pub pane: Option<String>,
    pub tab: Option<String>,
}

/// The editor handoff for a session.
pub struct Editor {
    /// The settings, or why the config is wrong.
    pub settings: Result<Settings, String>,
    pub herdr: Option<Herdr>,
    pub runner: Box<dyn Runner>,
    pub state: PlaceState,
}

impl Editor {
    /// No editor: `e` flashes why.
    pub fn none() -> Editor {
        Editor {
            settings: Err("No editor in this mode.".to_string()),
            herdr: None,
            runner: Box::new(System),
            state: PlaceState::default(),
        }
    }

    /// The editor's name, for the flash message.
    pub fn name(&self) -> &str {
        match &self.settings {
            Ok(s) => s.preset.name(),
            Err(_) => "the editor",
        }
    }

    /// Opens a file at a line.
    pub fn open(&mut self, req: &Request) -> Result<Outcome, String> {
        let settings = self.settings.clone()?;
        match (&settings.preset, self.herdr.clone()) {
            (Preset::Nvim, Some(h)) => self.open_nvim(&h, settings.place, req),
            (Preset::Vim, Some(h)) => self.open_vim(&h, settings.place, req),
            (Preset::Nvim | Preset::Vim, None) => {
                Ok(Outcome::Suspend(terminal_command(&settings.preset, req)))
            }
            (Preset::Custom { terminal: true, .. }, _) => {
                Ok(Outcome::Suspend(terminal_command(&settings.preset, req)))
            }
            (preset, _) => {
                self.runner.spawn(&detached_command(preset, req))?;
                Ok(Outcome::Opened)
            }
        }
    }

    fn open_nvim(&mut self, h: &Herdr, place: Place, req: &Request) -> Result<Outcome, String> {
        let socket = nvim_socket(&req.root);
        if self.runner.run(&nvim_probe(&socket)).is_ok() {
            self.runner.run(&nvim_send(&socket, req))?;
        } else {
            let pane = self.pane(h, place, &req.root)?;
            self.runner.remove_file(&socket);
            self.runner
                .run(&pane_run(&pane, &nvim_start(&socket, req)))?;
        }
        self.focus(h, place)?;
        Ok(Outcome::Opened)
    }

    fn open_vim(&mut self, h: &Herdr, place: Place, req: &Request) -> Result<Outcome, String> {
        let running = match &self.state.pane {
            Some(pane) => self
                .runner
                .run(&process_info(pane))
                .map(|json| foreground_names(&json).iter().any(|n| n == "vim"))
                .unwrap_or(false),
            None => false,
        };
        if running {
            let pane = self.state.pane.clone().unwrap_or_default();
            for step in vim_keys(&pane, req) {
                self.runner.run(&step)?;
            }
        } else {
            let pane = self.pane(h, place, &req.root)?;
            self.runner
                .run(&pane_run(&pane, &terminal_line(&Preset::Vim, req)))?;
        }
        self.focus(h, place)?;
        Ok(Outcome::Opened)
    }

    /// The editor's pane: the one from earlier opens while it exists, else a new place.
    fn pane(&mut self, h: &Herdr, place: Place, root: &Path) -> Result<String, String> {
        if let Some(pane) = &self.state.pane
            && self.runner.run(&pane_get(pane)).is_ok()
        {
            return Ok(pane.clone());
        }
        let json = self.runner.run(&create_place(h, place, root))?;
        let (pane, tab) = created_ids(&json)
            .ok_or_else(|| "herdr did not return the new pane's ID.".to_string())?;
        self.state = PlaceState {
            pane: Some(pane.clone()),
            tab: (place == Place::Tab).then_some(tab).flatten(),
        };
        Ok(pane)
    }

    fn focus(&mut self, h: &Herdr, place: Place) -> Result<(), String> {
        let step = match (place, &self.state.tab) {
            (Place::Tab, Some(tab)) => {
                vec!["herdr".into(), "tab".into(), "focus".into(), tab.clone()]
            }
            (Place::Tab, None) => return Ok(()),
            (dir, _) => vec![
                "herdr".into(),
                "pane".into(),
                "focus".into(),
                "--pane".into(),
                h.pane.clone(),
                "--direction".into(),
                if dir == Place::Right { "right" } else { "down" }.into(),
            ],
        };
        self.runner.run(&step).map(|_| ())
    }
}

fn argv(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

/// The nvim socket for a repo: `$TMPDIR/minimap-$USER-<16 hex digits of the FNV-1a hash of the root>.sock`.
pub fn nvim_socket(root: &Path) -> PathBuf {
    let hash = minimap_core::events::fnv1a(&root.to_string_lossy());
    let user = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
    std::env::temp_dir().join(format!("minimap-{user}-{hash:016x}.sock"))
}

/// Quotes a word for a POSIX shell.
pub fn shell_quote(s: &str) -> String {
    if !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "/._-+:=,@%".contains(c))
    {
        return s.to_string();
    }
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// A Vim command line that opens a path at a line and puts the line at the top.
/// `:drop` goes to a window that has the file, else edits it. The path is a Vim string, so `'` doubles.
pub fn vim_command(req: &Request) -> String {
    let path = req.path_str().replace('\'', "''");
    format!(
        ":execute 'drop ' .. fnameescape('{path}') | call cursor({}, 1) | normal! zt",
        req.line
    )
}

/// The command line that starts a terminal editor at a line, as the shell in a herdr pane runs it.
fn terminal_line(preset: &Preset, req: &Request) -> String {
    terminal_command(preset, req)
        .iter()
        .map(|a| shell_quote(a))
        .collect::<Vec<_>>()
        .join(" ")
}

/// `nvim --listen <socket> +<line> -c 'normal! zt' <path>`.
pub fn nvim_start(socket: &Path, req: &Request) -> String {
    let mut cmd = vec![
        "nvim".to_string(),
        "--listen".into(),
        socket.to_string_lossy().into_owned(),
    ];
    cmd.extend(terminal_command(&Preset::Nvim, req).into_iter().skip(1));
    cmd.iter()
        .map(|a| shell_quote(a))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Asks the nvim at a socket to answer.
pub fn nvim_probe(socket: &Path) -> Vec<String> {
    let s = socket.to_string_lossy();
    argv(&["nvim", "--server", &s, "--remote-expr", "1"])
}

/// Sends the open to the nvim at a socket. `<` is `<lt>` in `--remote-send` key notation.
pub fn nvim_send(socket: &Path, req: &Request) -> Vec<String> {
    let keys = format!(r"<C-\><C-N>{}<CR>", vim_command(req).replace('<', "<lt>"));
    let s = socket.to_string_lossy();
    argv(&["nvim", "--server", &s, "--remote-send", &keys])
}

/// A terminal editor's command: `<editor> +<line> -c 'normal! zt' <path>`, or the custom command.
pub fn terminal_command(preset: &Preset, req: &Request) -> Vec<String> {
    let path = req.path_str();
    let line = format!("+{}", req.line);
    match preset {
        Preset::Nvim => argv(&["nvim", &line, "-c", "normal! zt", &path]),
        Preset::Vim => argv(&["vim", &line, "-c", "normal! zt", &path]),
        other => detached_command(other, req),
    }
}

/// A command that opens the file itself and returns: VS Code, Emacs, or the custom command.
pub fn detached_command(preset: &Preset, req: &Request) -> Vec<String> {
    let path = req.path_str();
    match preset {
        Preset::Vscode => argv(&[
            "code",
            "--goto",
            &format!("{path}:{}:{}", req.line, req.col),
        ]),
        Preset::Emacs => argv(&[
            "emacsclient",
            "-n",
            &format!("+{}:{}", req.line, req.col),
            &path,
        ]),
        Preset::Custom { command, .. } => command.iter().map(|a| fill(a, req)).collect(),
        Preset::Nvim | Preset::Vim => terminal_command(preset, req),
    }
}

/// Replaces `{path}`, `{line}`, `{col}`, and `{root}` in one argument.
pub fn fill(arg: &str, req: &Request) -> String {
    arg.replace("{path}", &req.path_str())
        .replace("{line}", &req.line.to_string())
        .replace("{col}", &req.col.to_string())
        .replace("{root}", &req.root.to_string_lossy())
}

/// Creates the editor's place: a focused tab labeled `minimap editor`, or a split of minimap's pane.
pub fn create_place(h: &Herdr, place: Place, root: &Path) -> Vec<String> {
    let root = root.to_string_lossy();
    match place {
        Place::Tab => argv(&[
            "herdr",
            "tab",
            "create",
            "--workspace",
            &h.workspace,
            "--cwd",
            &root,
            "--label",
            "minimap editor",
            "--focus",
        ]),
        Place::Right | Place::Down => argv(&[
            "herdr",
            "pane",
            "split",
            &h.pane,
            "--direction",
            if place == Place::Right {
                "right"
            } else {
                "down"
            },
            "--cwd",
            &root,
        ]),
    }
}

/// The pane and tab IDs in herdr's answer to `tab create` or `pane split`.
pub fn created_ids(json: &str) -> Option<(String, Option<String>)> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let r = &v["result"];
    let pane = r["root_pane"]["pane_id"]
        .as_str()
        .or_else(|| r["pane"]["pane_id"].as_str())?;
    let tab = r["tab"]["tab_id"].as_str().map(str::to_string);
    Some((pane.to_string(), tab))
}

pub fn pane_get(pane: &str) -> Vec<String> {
    argv(&["herdr", "pane", "get", pane])
}

pub fn pane_run(pane: &str, line: &str) -> Vec<String> {
    argv(&["herdr", "pane", "run", pane, line])
}

pub fn process_info(pane: &str) -> Vec<String> {
    argv(&["herdr", "pane", "process-info", "--pane", pane])
}

/// The names of a pane's foreground processes in herdr's `process-info` answer.
pub fn foreground_names(json: &str) -> Vec<String> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
        return vec![];
    };
    v["result"]["process_info"]["foreground_processes"]
        .as_array()
        .map(|ps| {
            ps.iter()
                .filter_map(|p| p["name"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// The keys that make a running vim open a file: Escape for normal mode, the `:drop` line, then Enter.
pub fn vim_keys(pane: &str, req: &Request) -> Vec<Vec<String>> {
    vec![
        argv(&["herdr", "pane", "send-keys", pane, "Escape"]),
        argv(&["herdr", "pane", "send-text", pane, &vim_command(req)]),
        argv(&["herdr", "pane", "send-keys", pane, "Enter"]),
    ]
}
