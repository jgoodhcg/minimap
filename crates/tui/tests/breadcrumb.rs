//! The top bar's breadcrumb is `minimap_core`'s, which the GUI shows too (unit `038`).

mod common;

use common::*;
use minimap_tui::filetree::Entry;

#[test]
fn the_top_bar_breadcrumb_is_the_core_breadcrumb() {
    let (mut t, mut app) = quayside(200, 40);
    let f = file(&app, "components/berth/src/quayside/berth/scoring.clj");
    app.select(Entry::File(f), true);
    draw(&mut t, &mut app);
    let want = format!(" {}", app.tree.file_crumb(f).text());
    assert!(row(&t, 0).starts_with(&want), "{:?}", row(&t, 0));
    let n = app.tree.files[f].node;
    app.select(Entry::Dir(n), true);
    draw(&mut t, &mut app);
    let want = format!(" {}", app.tree.dir_crumb(n).text());
    assert_eq!(
        want,
        " quayside › components › berth › src › quayside › berth"
    );
    assert!(row(&t, 0).starts_with(&want), "{:?}", row(&t, 0));
}
