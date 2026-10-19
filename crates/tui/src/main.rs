use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use ratatui::crossterm::event::{self, Event};

use minimap_core::events::{self, Event as AgentEvent, Kind, ListenError, Listener};
use minimap_core::history::now;
use minimap_core::index::repo_root;
use minimap_core::watch::Watcher;
use minimap_core::{Config, Content, Details, History, Index, IndexOptions};
use minimap_tui::app::{GitHistory, LIVE_POLL};
use minimap_tui::editor::{self, Editor, Herdr};
use minimap_tui::keymap::Keymap;
use minimap_tui::{App, term};

/// A stable 2D map of a codebase, beside a coding agent.
#[derive(Parser)]
#[command(
    name = "minimap",
    version,
    args_conflicts_with_subcommands = true,
    after_help = "Agents: run \"minimap note --help\" to point a person at code on the map."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    #[command(flatten)]
    repo: RepoArgs,
    /// Do not draw Nerd Font file-type icons. Use it when the terminal font has no Nerd Font glyphs.
    #[arg(long)]
    no_icons: bool,
    /// Load an index dump or mockups/fake.json instead of a repo.
    #[arg(long, value_name = "FILE", conflicts_with_all = ["path", "excludes", "config"])]
    from_json: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Command {
    /// Print the file index of a repo as JSON.
    Index(RepoArgs),
    /// Print the keymap as config TOML: the defaults with the config's [keys] changes.
    Keys(KeysArgs),
    /// Send one event to the minimap that shows the repo. It prints nothing and always exits 0.
    Send(SendArgs),
    /// Read one agent hook payload on stdin and send its events. It prints nothing and always exits 0.
    Hook(HookArgs),
    /// Send a note about a file or its lines to the minimap window that shows the repo.
    #[command(long_about = NOTE_HELP)]
    Note(NoteArgs),
}

/// `minimap note --help`: what an agent needs to guide a person through code (unit `052`).
const NOTE_HELP: &str = "\
Send a note about a file, or lines of it, to the minimap window that shows the repo (minimap-gui).

The window marks the file on the map with the note's number, lists the note in its agent history
and in its selection panel, and shows the note on its lines in its reader.
The person presses n and N in the window to step through the notes in the order that they arrived.

Use notes to guide a person through code, for example a pull request review:
  1. Clear the notes of an earlier tour:
       minimap note --clear
  2. Send one note per stop, in the order to read them:
       minimap note src/retry.rs --lines 40-58 \"Retries start here. The delay doubles each time.\"
     Read a long text from stdin with -:
       minimap note src/retry.rs --lines 40-58 - <<'EOF'
       ...
       EOF
  3. Tell the person to press n in the window.

A note without --lines is about the whole file. A note without TEXT marks the lines without words.

Exit status: 0 when a window received the note. 1 when no minimap shows the repo, or when PATH is
outside the repo. 2 for a usage error.";

#[derive(Args)]
struct NoteArgs {
    /// The file that the note is about. A relative path resolves against the current directory.
    #[arg(value_name = "PATH", required_unless_present = "clear")]
    path: Option<PathBuf>,
    /// The note's text. - reads it from stdin. Without it, the note marks the lines without words.
    #[arg(value_name = "TEXT")]
    text: Option<String>,
    /// The lines that the note is about, 1-based: A-B, or one line.
    #[arg(long, value_name = "A-B", value_parser = lines_arg)]
    lines: Option<(u32, u32)>,
    /// A path inside the repo. Default: the repo of PATH, else of the current directory.
    #[arg(long, value_name = "PATH")]
    repo: Option<PathBuf>,
    /// Remove every note from the window.
    #[arg(long, conflicts_with_all = ["path", "text", "lines"])]
    clear: bool,
}

#[derive(Args)]
struct HookArgs {
    /// The agent: claude-code.
    #[arg(value_parser = ["claude-code"])]
    agent: String,
    /// Also write each raw payload to DIR/<tool>-<tool_use_id>.json.
    #[arg(long, value_name = "DIR")]
    log: Option<PathBuf>,
}

#[derive(Args)]
struct SendArgs {
    /// A path inside the repo. Default: the current directory.
    #[arg(long, value_name = "PATH")]
    repo: Option<PathBuf>,
    /// An event as one JSON line, for example '{"v":1,"type":"read","path":"src/a.clj"}'.
    #[arg(conflicts_with_all = ["read", "grep", "edit", "clear"])]
    json: Option<String>,
    /// The agent read this file.
    #[arg(long, value_name = "PATH", conflicts_with_all = ["grep", "edit", "clear"])]
    read: Option<PathBuf>,
    /// The agent found these files by search.
    #[arg(long, value_name = "PATH", num_args = 1.., conflicts_with_all = ["edit", "clear"])]
    grep: Vec<PathBuf>,
    /// The agent edited this file.
    #[arg(long, value_name = "PATH", conflicts_with = "clear")]
    edit: Option<PathBuf>,
    /// The edited lines, 1-based: A-B, or one line.
    #[arg(long, value_name = "A-B", requires = "edit")]
    lines: Option<String>,
    /// Empty the trail.
    #[arg(long)]
    clear: bool,
}

#[derive(Args)]
struct KeysArgs {
    /// The config file. Default: $XDG_CONFIG_HOME/minimap/config.toml.
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,
}

#[derive(Args)]
struct RepoArgs {
    /// A path inside the repo.
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Leave out files that match this glob (repo-relative). Repeatable.
    #[arg(long = "exclude", value_name = "GLOB")]
    excludes: Vec<String>,
    /// The config file. Default: $XDG_CONFIG_HOME/minimap/config.toml.
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Index(args)) => index(&args),
        Some(Command::Hook(args)) => {
            minimap_tui::claude_code::run(args.log.as_deref());
            Ok(())
        }
        Some(Command::Send(args)) => {
            // An adapter must never fail or slow the agent: every problem is silent.
            let _ = send(&args);
            Ok(())
        }
        Some(Command::Note(args)) => {
            if let Err(e) = note(&args) {
                eprintln!("minimap note: {e:#}");
                std::process::exit(1);
            }
            Ok(())
        }
        Some(Command::Keys(args)) => {
            let config = Config::load(args.config.as_deref())?;
            print!("{}", keymap(&config)?.to_toml());
            Ok(())
        }
        None => {
            let mut app = match &cli.from_json {
                Some(file) => {
                    let text = std::fs::read_to_string(file)
                        .with_context(|| format!("cannot read {}", file.display()))?;
                    let mut app = App::new(Index::from_json(&text)?);
                    app.keymap = Arc::new(keymap(&Config::load(None)?)?);
                    app.details = Details::from_json(&text)?;
                    let history = History::from_json(&text, now())?;
                    if !history.is_empty() {
                        app.git = GitHistory::Ready(history);
                    }
                    app
                }
                None => {
                    let (index, root, config, options) = load_index(&cli.repo)?;
                    // A bad [keys] table stops minimap here, before the terminal changes.
                    let keys = keymap(&config)?;
                    let mut app = App::new(index);
                    app.keymap = Arc::new(keys);
                    app.details = Details::repo(root.clone());
                    app.content = Content::Repo(root.clone());
                    app.theme_path = config.viewer_theme();
                    let var = |k: &str| std::env::var(k).ok();
                    app.editor = Editor {
                        settings: editor::Settings::resolve(
                            &config.editor,
                            var("VISUAL").as_deref(),
                            var("EDITOR").as_deref(),
                        ),
                        herdr: Herdr::from_env(),
                        runner: Box::new(editor::System),
                        state: Default::default(),
                    };
                    // The walk runs in the background, so the map shows first.
                    app.start_history(root.clone(), now());
                    // The live index (unit 034): without a watcher the map stays as it started.
                    match Watcher::spawn(root.clone(), options, app.index.clone()) {
                        Ok(w) => app.watcher = Some(w),
                        Err(e) => app.flash(format!("The map will not follow file changes: {e:#}")),
                    }
                    // The event socket (unit 010): one minimap per repo shows the trail.
                    match Listener::spawn(&root) {
                        Ok(l) => app.listener = Some(l),
                        Err(ListenError::AlreadyListening) => {
                            app.flash("Another minimap shows the trail for this repo.".to_string())
                        }
                        Err(e) => app.flash(format!("No trail: {e}")),
                    }
                    app
                }
            };
            app.icons = !cli.no_icons;
            app.copy = Box::new(|text| {
                arboard::Clipboard::new()
                    .and_then(|mut c| c.set_text(text))
                    .map_err(|e| e.to_string())
            });
            let mut terminal = term::init()?;
            let result = run(&mut terminal, &mut app);
            term::restore();
            result
        }
    }
}

/// `minimap send`: builds the event and sends it to the repo's socket. Relative paths resolve against the current
/// directory.
fn send(args: &SendArgs) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let abs = |p: &PathBuf| cwd.join(p).to_string_lossy().into_owned();
    let event = if let Some(line) = &args.json {
        events::parse(line).context("not an event")?
    } else if let Some(p) = &args.read {
        AgentEvent::trail(Kind::Read, vec![abs(p)])
    } else if !args.grep.is_empty() {
        AgentEvent::trail(Kind::Grep, args.grep.iter().map(abs).collect())
    } else if let Some(p) = &args.edit {
        AgentEvent::Trail {
            kind: Kind::Edit,
            paths: vec![abs(p)],
            lines: args.lines.as_deref().and_then(parse_lines),
            source: None,
            session: None,
        }
    } else if args.clear {
        AgentEvent::Clear
    } else {
        bail!("no event");
    };
    let root = repo_root(args.repo.as_deref().unwrap_or(&cwd))?;
    events::send(&root, &event)?;
    Ok(())
}

/// `minimap note`: sends a note, or a notes clear, and reports a window that did not receive it (unit `052`).
fn note(args: &NoteArgs) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let path = args.path.as_ref().map(|p| cwd.join(p));
    // The repo of the path: its nearest existing directory, as a new file's directory may not exist yet.
    let base = match (&args.repo, &path) {
        (Some(r), _) => cwd.join(r),
        (None, Some(p)) => p
            .ancestors()
            .skip(1)
            .find(|d| d.is_dir())
            .map_or(cwd.clone(), std::path::Path::to_path_buf),
        (None, None) => cwd.clone(),
    };
    let root = repo_root(&base)?;
    let event = match path {
        None => AgentEvent::ClearNotes,
        Some(p) => {
            let abs = p.to_string_lossy().into_owned();
            let canonical = std::fs::canonicalize(&root).unwrap_or_else(|_| root.clone());
            if events::normalize(&root, &canonical, &abs).is_none() {
                bail!("{} is outside the repo {}", p.display(), root.display());
            }
            let text = match args.text.as_deref() {
                Some("-") => {
                    let mut t = String::new();
                    std::io::Read::read_to_string(&mut std::io::stdin(), &mut t)
                        .context("cannot read the note from stdin")?;
                    t.trim_end().to_string()
                }
                Some(t) => t.to_string(),
                None => String::new(),
            };
            AgentEvent::Note {
                path: abs,
                lines: args.lines,
                text: events::note_text(&text),
                source: None,
                session: None,
            }
        }
    };
    events::send(&root, &event).map_err(|_| {
        anyhow::anyhow!(
            "no minimap shows {}. Start minimap-gui there.",
            root.display()
        )
    })
}

/// `--lines` for `minimap note`: `A-B` or `A`, from line 1.
fn lines_arg(s: &str) -> Result<(u32, u32), String> {
    parse_lines(s)
        .filter(|&(a, _)| a >= 1)
        .ok_or_else(|| format!("{s:?} is not A-B or one line, from line 1"))
}

/// `A-B` or `A` as a line range.
fn parse_lines(s: &str) -> Option<(u32, u32)> {
    let (a, b) = s.split_once(['-', '–']).unwrap_or((s, s));
    let (a, b): (u32, u32) = (a.trim().parse().ok()?, b.trim().parse().ok()?);
    Some((a.min(b), a.max(b)))
}

/// The keymap from the config's `[keys]` tables. Each problem is one line of the error.
fn keymap(config: &Config) -> Result<Keymap> {
    match Keymap::from_config(&config.keys) {
        Ok(k) => Ok(k),
        Err(errors) => bail!(
            "the config's key bindings have problems:\n  {}",
            errors.join("\n  ")
        ),
    }
}

/// Indexes the repo with the config excludes and the `--exclude` flags combined.
/// Returns the index, the repo root, the config, and the index options.
fn load_index(args: &RepoArgs) -> Result<(Index, PathBuf, Config, IndexOptions)> {
    let config = Config::load(args.config.as_deref())?;
    let root = repo_root(&args.path)?;
    let mut excludes = config.excludes_for(&root);
    excludes.extend(args.excludes.iter().cloned());
    let options = IndexOptions {
        excludes,
        generated: config.generated_for(&root),
    };
    let index = Index::from_repo(&root, &options)?;
    Ok((index, root, config, options))
}

fn index(args: &RepoArgs) -> Result<()> {
    let t0 = Instant::now();
    let (index, _, _, _) = load_index(args)?;
    let ms = t0.elapsed().as_millis();
    let mut out = std::io::stdout().lock();
    writeln!(out, "{}", index.to_json())?;
    eprintln!(
        "{} files, {} excluded, {} ms",
        index.files.len(),
        index.excluded,
        ms
    );
    Ok(())
}

fn run(terminal: &mut term::Tui, app: &mut App) -> Result<()> {
    let start = Instant::now();
    while !app.quit {
        app.now = start.elapsed();
        terminal.draw(|frame| minimap_tui::draw(frame, app))?;
        if !wait_for_input(app, start)? {
            continue;
        }
        // Handle every pending event before the next draw, so a burst of mouse moves draws once.
        loop {
            app.now = start.elapsed();
            match event::read()? {
                Event::Key(key) => app.on_key(key),
                Event::Mouse(m) => app.on_mouse(m),
                // A resize redraws on the next loop.
                _ => {}
            }
            if app.quit || app.suspend.is_some() || !event::poll(Duration::ZERO)? {
                break;
            }
        }
        // A terminal editor without herdr takes the terminal until it exits (unit 031).
        if let Some(cmd) = app.suspend.take() {
            let name = app.editor.name().to_string();
            let result = term::run_suspended(terminal, &cmd);
            // The editor ran for a while: the flash counts from now.
            app.now = start.elapsed();
            match result {
                Ok(()) => app.flash(format!("Back from {name}")),
                Err(e) => app.flash(e),
            }
        }
    }
    Ok(())
}

/// Waits for input, or only until the screen changes by itself: a flash message ends, or a live input such as the
/// watcher or the event socket sends something. Live inputs are checked every [`LIVE_POLL`]. Returns true when input is ready.
fn wait_for_input(app: &mut App, start: Instant) -> Result<bool> {
    let redraw_at = app.next_redraw().map(|w| Instant::now() + w);
    loop {
        let until_redraw = redraw_at.map(|t| t.saturating_duration_since(Instant::now()));
        let wait = match (until_redraw, app.live()) {
            (Some(r), true) => r.min(LIVE_POLL),
            (Some(r), false) => r,
            (None, true) => LIVE_POLL,
            // Nothing changes the screen by itself: block until input.
            (None, false) => return Ok(true),
        };
        if event::poll(wait)? {
            return Ok(true);
        }
        app.now = start.elapsed();
        if app.poll_live() || redraw_at.is_some_and(|t| Instant::now() >= t) {
            return Ok(false);
        }
    }
}
