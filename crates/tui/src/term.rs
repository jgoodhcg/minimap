//! Terminal setup and restore.

use std::io::{self, Stdout, stdout};

use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::cursor::Show;
use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Enters raw mode and the alternate screen, and enables mouse capture.
/// It also installs a panic hook that restores the terminal before the panic message prints.
pub fn init() -> io::Result<Tui> {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        hook(info);
    }));
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    Terminal::new(CrosstermBackend::new(stdout()))
}

/// Disables mouse capture, leaves the alternate screen, shows the cursor, and disables raw mode.
/// It ignores errors, because it also runs inside the panic hook.
pub fn restore() {
    let _ = execute!(stdout(), DisableMouseCapture, LeaveAlternateScreen, Show);
    let _ = disable_raw_mode();
}

/// Gives the terminal back for a command that runs in it, such as a terminal editor, and waits for it.
/// Then it takes the terminal again and clears it, so the next draw repaints everything.
pub fn run_suspended(terminal: &mut Tui, argv: &[String]) -> Result<(), String> {
    restore();
    let status = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .status();
    let _ = enable_raw_mode();
    let _ = execute!(stdout(), EnterAlternateScreen, EnableMouseCapture);
    let _ = terminal.clear();
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("{} exited with {s}", argv[0])),
        Err(e) => Err(format!("{}: {e}", argv[0])),
    }
}
