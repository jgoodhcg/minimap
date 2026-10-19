//! The file tree's row model (unit `042`) and the shared `l` and `h` decisions (unit `043`).

use minimap_core::Tree;
use minimap_core::filetree::{Entry, FileTree, Step};

fn tree() -> Tree {
    Tree::from_paths(
        "r",
        [
            ("a/one.txt", false),
            ("a/b/two.txt", false),
            ("a/b/three.txt", false),
            ("c/four.txt", false),
            ("top.txt", false),
        ],
    )
}

fn node(t: &Tree, path: &str) -> usize {
    t.nodes.iter().position(|n| n.path == path).unwrap()
}

fn file(t: &Tree, path: &str) -> usize {
    t.files.iter().position(|f| f.path == path).unwrap()
}

#[test]
fn l_opens_a_closed_directory_then_goes_to_its_first_child() {
    let t = tree();
    let mut ft = FileTree::new(&t);
    let a = node(&t, "a");
    let rows = ft.rows(&t, true);
    assert_eq!(
        ft.step_in(&t, &rows, Some(Entry::Dir(a))),
        Some(Step::Open(a))
    );
    ft.set_open(a, true);
    let rows = ft.rows(&t, true);
    // The first child row of `a` is its child directory `b`, before its files.
    let b = node(&t, "a/b");
    assert_eq!(
        ft.step_in(&t, &rows, Some(Entry::Dir(a))),
        Some(Step::Select(Entry::Dir(b)))
    );
    // A file, or no selection, does nothing.
    let f = file(&t, "a/one.txt");
    assert_eq!(ft.step_in(&t, &rows, Some(Entry::File(f))), None);
    assert_eq!(ft.step_in(&t, &rows, None), None);
}

#[test]
fn l_on_an_open_empty_row_list_does_nothing_past_the_directory() {
    // An open directory whose next row is not its child (here: no next row) gives nothing.
    let t = Tree::from_paths("r", [("z/only.txt", false)]);
    let mut ft = FileTree::new(&t);
    let z = node(&t, "z");
    ft.set_open(z, true);
    let rows = ft.rows(&t, true);
    assert_eq!(
        ft.step_in(&t, &rows, Some(Entry::Dir(z))),
        Some(Step::Select(Entry::File(0)))
    );
    let rows = vec![rows[0]];
    assert_eq!(ft.step_in(&t, &rows, Some(Entry::Dir(z))), None);
}

#[test]
fn h_closes_an_open_directory_else_goes_to_the_parent() {
    let t = tree();
    let mut ft = FileTree::new(&t);
    let (a, b) = (node(&t, "a"), node(&t, "a/b"));
    ft.set_open(a, true);
    assert_eq!(ft.step_out(&t, Some(Entry::Dir(a))), Some(Step::Close(a)));
    // A closed directory and a file go up to their parent directory.
    assert_eq!(
        ft.step_out(&t, Some(Entry::Dir(b))),
        Some(Step::Select(Entry::Dir(a)))
    );
    let two = file(&t, "a/b/two.txt");
    assert_eq!(
        ft.step_out(&t, Some(Entry::File(two))),
        Some(Step::Select(Entry::Dir(b)))
    );
    // A root child has no parent row.
    let top = file(&t, "top.txt");
    assert_eq!(ft.step_out(&t, Some(Entry::File(top))), None);
    ft.set_open(a, false);
    assert_eq!(ft.step_out(&t, Some(Entry::Dir(a))), None);
    assert_eq!(ft.step_out(&t, None), None);
}
