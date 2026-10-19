//! The minimap core: the file index, the map layout, fuzzy path matching, file details (shape and outline),
//! the file tree's rows, file text and uncommitted diffs for the viewer, git history, layout stability measurement, the live index,
//! the event socket and the agent's notes, the file-type icons and colors and the text helpers that every renderer shares, grammar outlines, syntax colors, and generated-file marks.
//!
//! This crate has no terminal or renderer dependencies,
//! so a pixel renderer can attach to it later.

pub mod config;
pub mod content;
pub mod details;
pub mod diff;
pub mod events;
pub mod filetree;
pub mod filetype;
pub mod fuzzy;
pub mod generated;
pub mod highlight;
pub mod history;
pub mod index;
pub mod layout;
pub mod notes;
pub mod outline;
pub mod palette;
pub mod shape;
pub mod stability;
pub mod syntax;
pub mod text;
pub mod trail;
pub mod watch;

pub use config::Config;
pub use content::Content;
pub use details::{Details, FileDetails};
pub use history::History;
pub use index::{Index, IndexFile, IndexOptions};
pub use layout::{Layout, LayoutConfig, Tree};
