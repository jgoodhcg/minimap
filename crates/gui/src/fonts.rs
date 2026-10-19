//! The GUI's fonts: a Nerd Font for text and icons, and its bold face for directory names.
//!
//! The `[gui] font` config file comes first. Else the first `*NerdFontMono-Regular.ttf` by name in `~/Library/Fonts`,
//! then in `/Library/Fonts`. Without one, egui's built-in fonts draw the text, and icons are squares.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use eframe::egui::{self, FontData, FontDefinitions, FontFamily, FontId};

/// The family name of the bold face.
pub const BOLD: &str = "bold";
const FILE_SUFFIX: &str = "NerdFontMono-Regular.ttf";

/// A regular face and, when it has a sibling file, its bold face.
pub struct Faces {
    pub regular: PathBuf,
    pub bold: Option<PathBuf>,
}

/// The font files to use. An explicit path that does not exist is an error.
pub fn find(explicit: Option<PathBuf>) -> Result<Option<Faces>> {
    let regular = match explicit {
        Some(p) => {
            if !p.is_file() {
                anyhow::bail!("the [gui] font {} does not exist", p.display());
            }
            Some(p)
        }
        None => search(),
    };
    Ok(regular.map(|regular| Faces {
        bold: bold_of(&regular),
        regular,
    }))
}

fn search() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let dirs = home
        .map(|h| h.join("Library/Fonts"))
        .into_iter()
        .chain([PathBuf::from("/Library/Fonts")]);
    for dir in dirs {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        let mut names: Vec<String> = entries
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .filter(|n| n.ends_with(FILE_SUFFIX))
            .collect();
        names.sort();
        if let Some(n) = names.first() {
            return Some(dir.join(n));
        }
    }
    None
}

/// `X-Regular.ttf` → `X-Bold.ttf`, when that file exists.
fn bold_of(regular: &Path) -> Option<PathBuf> {
    let name = regular.file_name()?.to_str()?;
    let bold = regular.with_file_name(name.replace("-Regular.", "-Bold."));
    (bold != regular && bold.is_file()).then_some(bold)
}

/// Installs the faces in front of egui's fonts, and the bold family. Returns true when icons can draw.
pub fn install(ctx: &egui::Context, faces: Option<&Faces>) -> Result<bool> {
    let mut defs = FontDefinitions::default();
    let mut bold = defs.families[&FontFamily::Monospace].clone();
    let icons = match faces {
        Some(f) => {
            let regular = fs::read(&f.regular)
                .with_context(|| format!("cannot read the font {}", f.regular.display()))?;
            defs.font_data
                .insert("nerd".into(), Arc::new(FontData::from_owned(regular)));
            for family in [FontFamily::Monospace, FontFamily::Proportional] {
                defs.families
                    .get_mut(&family)
                    .expect("egui has this family")
                    .insert(0, "nerd".into());
            }
            bold.insert(0, "nerd".into());
            if let Some(b) = &f.bold
                && let Ok(bytes) = fs::read(b)
            {
                defs.font_data
                    .insert("nerd-bold".into(), Arc::new(FontData::from_owned(bytes)));
                bold.insert(0, "nerd-bold".into());
            }
            true
        }
        None => false,
    };
    defs.families.insert(FontFamily::Name(BOLD.into()), bold);
    ctx.set_fonts(defs);
    Ok(icons)
}

/// The font for a text item.
pub fn font(size: f32, bold: bool) -> FontId {
    if bold {
        FontId::new(size, FontFamily::Name(BOLD.into()))
    } else {
        FontId::new(size, FontFamily::Monospace)
    }
}
