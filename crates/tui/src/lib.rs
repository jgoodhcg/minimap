//! The minimap terminal renderer.

pub mod app;
pub mod claude_code;
pub mod editor;
pub mod filetree;
pub mod icons;
pub mod keymap;
pub mod search;
pub mod shape;
pub mod syntax;
pub mod term;
pub mod text;
pub mod theme;
pub mod trail;
pub mod ui;
pub mod viewer;

use ratatui::Frame;

pub use app::App;

/// Records the frame size in the app, takes the git history and syntax colors that have arrived,
/// loads the selected file's details, then draws the screen.
pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    app.resize(area);
    app.poll_history();
    app.poll_syntax();
    app.load_details();
    frame.render_widget(ui::Screen(app), area);
}
