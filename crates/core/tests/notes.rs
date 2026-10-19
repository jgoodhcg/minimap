//! The notes model (unit `052`): ids, ordinals, the tour order, and the cap. The trail ignores notes.

use std::time::Duration;

use minimap_core::events::{Event, Kind};
use minimap_core::notes::{self, Notes};
use minimap_core::trail::Trail;

fn note(path: &str, lines: Option<(u32, u32)>) -> Event {
    Event::Note {
        path: path.into(),
        lines,
        text: format!("about {path}"),
        source: None,
        session: Some("s1".into()),
    }
}

#[test]
fn notes_take_ids_in_arrival_order_and_keep_them_after_a_clear() {
    let mut n = Notes::default();
    assert_eq!(n.record(&note("a", Some((1, 2)))), Some(1));
    assert_eq!(n.record(&note("b", Some((9, 4)))), Some(2));
    assert_eq!(n.get(2).unwrap().lines, Some((4, 9)), "lines in order");
    assert_eq!(n.record(&Event::trail(Kind::Read, vec!["a".into()])), None);
    assert_eq!(n.len(), 2);
    let a = n.get(1).unwrap();
    assert_eq!((a.path.as_str(), a.lines), ("a", Some((1, 2))));
    assert_eq!(a.text, "about a");
    assert_eq!(a.session.as_deref(), Some("s1"));
    assert_eq!(n.record(&Event::ClearNotes), None);
    assert!(n.is_empty());
    assert_eq!(n.get(1), None);
    // An id never repeats, and the ordinal restarts.
    assert_eq!(n.record(&note("c", None)), Some(3));
    assert_eq!(n.ordinal(3), Some(1));
    assert_eq!(n.ordinal(1), None);
}

#[test]
fn of_path_lists_a_files_notes_oldest_first() {
    let mut n = Notes::default();
    for p in ["a", "b", "a", "c", "a"] {
        n.record(&note(p, None));
    }
    let ids: Vec<u64> = n.of_path("a").map(|x| x.id).collect();
    assert_eq!(ids, [1, 3, 5]);
    assert_eq!(n.of_path("z").count(), 0);
}

#[test]
fn next_and_previous_step_through_the_tour() {
    let mut n = Notes::default();
    let id = |x: Option<&notes::Note>| x.map(|x| x.id);
    assert_eq!(id(n.next(None)), None);
    assert_eq!(id(n.previous(None)), None);
    for p in ["a", "b", "c"] {
        n.record(&note(p, None));
    }
    assert_eq!(id(n.next(None)), Some(1));
    assert_eq!(id(n.next(Some(1))), Some(2));
    assert_eq!(id(n.next(Some(3))), None, "after the last note");
    assert_eq!(id(n.previous(None)), Some(3));
    assert_eq!(id(n.previous(Some(2))), Some(1));
    assert_eq!(id(n.previous(Some(1))), None, "before the first note");
    // From an id that a clear removed: the first, or the last, note.
    n.clear();
    for p in ["d", "e"] {
        n.record(&note(p, None));
    }
    assert_eq!(id(n.next(Some(2))), Some(4));
    assert_eq!(id(n.previous(Some(2))), Some(5));
}

#[test]
fn the_oldest_notes_drop_past_the_cap() {
    let mut n = Notes::default();
    for k in 0..notes::MAX + 5 {
        n.record(&note(&format!("f{k}"), None));
    }
    assert_eq!(n.len(), notes::MAX);
    assert_eq!(n.all()[0].id, 6);
    assert_eq!(n.ordinal(6), Some(1));
    assert_eq!(n.get(5), None);
}

#[test]
fn the_trail_ignores_notes() {
    let mut t = Trail::default();
    t.record(&note("a", Some((1, 2))), Duration::from_secs(1));
    t.record(&Event::ClearNotes, Duration::from_secs(2));
    assert!(t.is_empty());
    t.record(
        &Event::trail(Kind::Read, vec!["a".into()]),
        Duration::from_secs(3),
    );
    t.record(&Event::ClearNotes, Duration::from_secs(4));
    assert_eq!(t.len(), 1, "a notes clear keeps the trail");
}
