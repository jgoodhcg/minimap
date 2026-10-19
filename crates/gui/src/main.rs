use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use eframe::egui;

use minimap_core::details::Loader;
use minimap_core::events::{ListenError, Listener};
use minimap_core::history::{self, History};
use minimap_core::index::repo_root;
use minimap_core::watch::Watcher;
use minimap_core::{Config, Details, Index, IndexOptions};
use minimap_gui::capture::{self, Capture};
use minimap_gui::panel::GitHistory;
use minimap_gui::scene::Map;
use minimap_gui::{GuiApp, fonts};

/// A zoomable map of a codebase in a desktop window.
#[derive(Parser)]
#[command(name = "minimap-gui", version)]
struct Cli {
    /// A path inside the repo.
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Leave out files that match this glob (repo-relative). Repeatable.
    #[arg(long = "exclude", value_name = "GLOB")]
    excludes: Vec<String>,
    /// The config file. Default: $XDG_CONFIG_HOME/minimap/config.toml.
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,
    /// Load an index dump or mockups/fake.json instead of a repo.
    #[arg(long, value_name = "FILE", conflicts_with_all = ["path", "excludes", "config"])]
    from_json: Option<PathBuf>,
    /// Do not draw Nerd Font file-type icons: an icon is a square in the type color.
    #[arg(long)]
    no_icons: bool,
    /// Run the visual check script: save PNGs and frame times to DIR, then exit.
    #[arg(long, value_name = "DIR")]
    capture: Option<PathBuf>,
    /// Select this file or directory at startup and zoom to it. The capture steps center on it.
    #[arg(long, value_name = "PATH")]
    select: Option<String>,
}

/// Where the git history comes from.
enum Git {
    None,
    Ready(History),
    /// Walk the repo at this root.
    Walk(PathBuf),
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let t0 = Instant::now();
    // The git history: the walk starts with the window (unit `044`); an index JSON may carry its own.
    // The live index (unit 048) watches the repo with the options it was indexed with.
    let mut live = None;
    let (index, config, details, git) = match &cli.from_json {
        Some(file) => {
            let text = std::fs::read_to_string(file)
                .with_context(|| format!("cannot read {}", file.display()))?;
            let history = History::from_json(&text, history::now())?;
            let git = if history.is_empty() {
                Git::None
            } else {
                Git::Ready(history)
            };
            (
                Index::from_json(&text)?,
                Config::load(None)?,
                Details::from_json(&text)?,
                git,
            )
        }
        None => {
            let config = Config::load(cli.config.as_deref())?;
            let root = repo_root(&cli.path)?;
            let mut excludes = config.excludes_for(&root);
            excludes.extend(cli.excludes.iter().cloned());
            let options = IndexOptions {
                excludes,
                generated: config.generated_for(&root),
            };
            let index = Index::from_repo(&root, &options)?;
            live = Some((root.clone(), options));
            (index, config, Details::repo(root.clone()), Git::Walk(root))
        }
    };
    let faces = fonts::find(config.gui_font())?;
    let theme = config.viewer_theme();
    let title = format!("minimap — {}", index.root);
    let map = Map::new(index);
    eprintln!(
        "{} files, map {} x {} cells, {} ms",
        map.index.files.len(),
        map.layout.w,
        map.layout.h,
        t0.elapsed().as_millis()
    );

    let mut app = GuiApp::new(map);
    // The note reader (unit 054) reads the working tree. With --from-json it has no text.
    if let Some((root, _)) = &live {
        app.content = minimap_core::Content::Repo(root.clone());
    }
    let mut viewport = egui::ViewportBuilder::default()
        .with_title(&title)
        .with_inner_size([1400.0, 900.0]);
    let selected = match &cli.select {
        Some(p) => Some(
            app.map
                .entry(p)
                .ok_or_else(|| anyhow!("--select: no file or directory {p} in the index"))?,
        ),
        None => None,
    };
    if let Some(e) = selected {
        app.select_at_start(e);
    }
    if let Some(dir) = &cli.capture {
        let target = match selected {
            Some(e) => app.map.rect_of(e).center(),
            None => (app.map.world() / 2.0).to_pos2(),
        };
        app.capture = Some(Capture::new(dir.clone(), target, capture::steps())?);
        // The capture window opens at a fixed size, without asking for the focus. It floats above other windows,
        // because eframe draws nothing while its window is covered.
        viewport = viewport
            .with_inner_size(capture::WINDOW)
            .with_resizable(false)
            .with_active(false)
            .with_window_level(egui::WindowLevel::AlwaysOnTop);
        // A window that never draws must not hang the check.
        std::thread::spawn(|| {
            std::thread::sleep(capture::TIMEOUT);
            eprintln!(
                "the capture stopped after {} s: its window did not draw. Is it on another desktop or minimized?",
                capture::TIMEOUT.as_secs()
            );
            std::process::exit(2);
        });
    }
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    let no_icons = cli.no_icons;
    let capture_dir = cli.capture.clone();
    let result = eframe::run_native(
        "minimap-gui",
        options,
        Box::new(move |cc| {
            let icons = fonts::install(&cc.egui_ctx, faces.as_ref())?;
            cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
            app.opts.icons = icons && !no_icons;
            // Each result repaints the window, so shapes show as they arrive.
            let ctx = cc.egui_ctx.clone();
            app.loader = Some(Loader::spawn(details, move || ctx.request_repaint()));
            // The note reader's syntax colors (unit 055), with the config's [viewer] theme when it names one.
            let ctx = cc.egui_ctx.clone();
            app.colorer = Some(minimap_gui::reader::Colorer::spawn(theme, move || {
                ctx.request_repaint()
            }));
            // The walk runs in the background, so the map shows first; its end repaints the window.
            // The live index (unit 048): each update repaints the window.
            if let Some((root, options)) = live {
                let ctx = cc.egui_ctx.clone();
                let index = app.map.index.clone();
                match Watcher::spawn_with(root.clone(), options, index, move || {
                    ctx.request_repaint()
                }) {
                    Ok(w) => app.watcher = Some(w),
                    Err(e) => {
                        app.notice = Some(format!("The map will not follow file changes: {e:#}"))
                    }
                }
                app.root = Some(root);
            }
            // The agent's trail (unit 047): the listener's events repaint the window. One minimap per repo listens.
            if let Git::Walk(root) = &git {
                let ctx = cc.egui_ctx.clone();
                match Listener::spawn_with(root, move || ctx.request_repaint()) {
                    Ok(l) => app.listener = Some(l),
                    Err(ListenError::AlreadyListening) => {
                        app.notice =
                            Some("Another minimap shows the trail for this repo.".to_string());
                    }
                    Err(e) => app.notice = Some(format!("The trail is off: {e}")),
                }
            }
            app.git = match git {
                Git::None => GitHistory::None,
                Git::Ready(h) => GitHistory::Ready(h),
                Git::Walk(root) => {
                    let paths = app.map.index.files.iter().map(|f| f.path.clone()).collect();
                    let ctx = cc.egui_ctx.clone();
                    GitHistory::Loading(history::Loader::spawn_with(
                        root,
                        paths,
                        history::now(),
                        move || ctx.request_repaint(),
                    ))
                }
            };
            Ok(Box::new(app))
        }),
    );
    result.map_err(|e| anyhow!("{e}"))?;
    if let Some(dir) = capture_dir {
        let frames = dir.join("frames.txt");
        match std::fs::read_to_string(&frames) {
            Ok(text) => eprint!("{text}"),
            Err(_) => eprintln!("the capture did not finish: no {}", frames.display()),
        }
    }
    Ok(())
}
