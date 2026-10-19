//! The minimap desktop window (Milestone G1): the `CARDS` map as file cards, with smooth pan and zoom, in egui,
//! the file tree beside it (Milestone G3), the selection panel (Milestone G4), and the agent's activity (Milestones G5 and G6).
//!
//! It links `minimap-core` directly. The TUI stays an option on the same core.

pub mod activity;
pub mod app;
pub mod camera;
pub mod capture;
pub mod color;
pub mod fonts;
pub mod paint;
pub mod panel;
pub mod reader;
pub mod scene;
pub mod text;
pub mod tree;

pub use app::GuiApp;
