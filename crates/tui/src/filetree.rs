//! The file tree in the left column (units 017 and 018).
//!
//! The row model is `minimap_core::filetree` (unit `042`), which the GUI shares.

pub use minimap_core::filetree::{Entry, FileTree, Row, Step};

/// Wheel step over the tree, in rows.
pub const WHEEL_ROWS: usize = 3;
