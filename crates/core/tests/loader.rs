//! The background details loader (unit `036`), on a temporary repo with quayside paths.

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use minimap_core::Index;
use minimap_core::details::{Details, FileDetails, Loader};
use minimap_core::outline::{generic, language_for};

fn fixture(name: &str) -> String {
    fs::read_to_string(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

/// A text for a path, by its type, so each language's outline has items.
fn text_for(path: &str, k: usize) -> String {
    let name = path.rsplit('/').next().unwrap();
    let ext = name.rsplit_once('.').map_or("", |(_, e)| e);
    let body = |n: usize, f: &dyn Fn(usize) -> String| (0..n).map(f).collect::<Vec<_>>().join("\n");
    match ext {
        "clj" | "cljs" | "cljc" => format!(
            "(ns quayside.n{k})\n\n{}\n",
            body(3 + k % 5, &|i| format!("(defn f{i}\n  [x]\n  (inc x))\n"))
        ),
        "md" => format!(
            "# Title {k}\n\n{}\n",
            body(2 + k % 3, &|i| format!("## Part {i}\n\nText.\n"))
        ),
        "py" => body(2 + k % 3, &|i| format!("def f{i}(x):\n    return x\n")),
        "ts" | "js" => body(2 + k % 3, &|i| {
            format!("function f{i}(x) {{\n  return x;\n}}\n")
        }),
        "sql" => format!("create table t{k} (id int);\n"),
        "png" | "jpg" | "ico" => "\u{0}PNG binary".to_string(),
        _ => body(1 + k % 4, &|i| format!("line {i} of {name}")),
    }
}

/// Writes up to `n` quayside paths, spread over the whole index so every kind of file shows, with generated texts. Returns the paths.
fn temp_repo(dir: &Path, n: usize) -> Vec<String> {
    let index = Index::from_json(&fixture("quayside-files.json")).unwrap();
    let step = (index.files.len() / n).max(1);
    let mut paths = Vec::new();
    for (k, f) in index.files.iter().enumerate().step_by(step).take(n) {
        let p = dir.join(&f.path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, text_for(&f.path, k)).unwrap();
        paths.push(f.path.clone());
    }
    paths
}

/// Polls until `n` results arrived, or panics after `limit`.
fn wait_for(loader: &mut Loader, n: usize, limit: Duration) -> Vec<(String, FileDetails)> {
    let t0 = Instant::now();
    let mut out = Vec::new();
    while out.len() < n {
        out.extend(loader.poll());
        assert!(
            t0.elapsed() < limit,
            "{} of {n} results after {limit:?}",
            out.len()
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    out
}

#[test]
fn results_equal_compute_plus_the_generic_outline() {
    let dir = tempfile::tempdir().unwrap();
    let paths = temp_repo(dir.path(), 300);
    let mut loader = Loader::spawn(Details::repo(dir.path().to_path_buf()), || {});
    loader.request(paths.clone());
    let results = wait_for(&mut loader, paths.len(), Duration::from_secs(20));
    assert_eq!(results.len(), paths.len());
    let (mut generic_items, mut clojure_items) = (0, 0);
    for (path, got) in &results {
        let bytes = fs::read(dir.path().join(path)).unwrap();
        let mut want = FileDetails::compute(path, &bytes);
        if want.shape.is_some() && language_for(path).is_none() {
            want.outline = generic(path, &String::from_utf8_lossy(&bytes));
            generic_items += want.outline.len();
        } else {
            clojure_items += want.outline.len();
        }
        assert_eq!(got, &want, "{path}");
        assert_eq!(loader.get(path), Some(got));
    }
    assert!(generic_items > 0 && clojure_items > 0);
    // Binary files have no shape and no outline.
    let binary: Vec<_> = results
        .iter()
        .filter(|(p, _)| text_for(p, 0).starts_with('\u{0}'))
        .collect();
    assert!(!binary.is_empty());
    assert!(binary.iter().all(|(_, d)| d == &FileDetails::default()));
}

#[test]
fn a_second_request_reorders_the_pending_paths() {
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    let mut idle = Loader::with_workers(Details::memory(Default::default()), || {}, 0);
    idle.request(s(&["a", "b", "c", "d"]));
    assert_eq!(idle.pending(), s(&["a", "b", "c", "d"]));
    idle.request(s(&["d", "c", "x", "c"]));
    assert_eq!(idle.pending(), s(&["d", "c", "x"]));

    // Paths that are done are skipped.
    let dir = tempfile::tempdir().unwrap();
    let paths = temp_repo(dir.path(), 4);
    let mut loader = Loader::with_workers(Details::repo(dir.path().to_path_buf()), || {}, 1);
    loader.request(paths[..2].to_vec());
    wait_for(&mut loader, 2, Duration::from_secs(10));
    loader.request(paths.clone());
    let rest = wait_for(&mut loader, 2, Duration::from_secs(10));
    let mut got: Vec<&str> = rest.iter().map(|(p, _)| p.as_str()).collect();
    got.sort();
    let mut want: Vec<&str> = paths[2..].iter().map(String::as_str).collect();
    want.sort();
    assert_eq!(got, want);
    std::thread::sleep(Duration::from_millis(50));
    assert!(loader.poll().is_empty(), "no path is computed twice");
    // A forgotten path is computed again.
    loader.forget(&paths[0]);
    assert_eq!(loader.get(&paths[0]), None);
    loader.request(vec![paths[0].clone()]);
    let again = wait_for(&mut loader, 1, Duration::from_secs(10));
    assert_eq!(again[0].0, paths[0]);
}

#[test]
fn wake_runs_once_per_result() {
    let dir = tempfile::tempdir().unwrap();
    let paths = temp_repo(dir.path(), 120);
    let wakes = Arc::new(AtomicUsize::new(0));
    let w = wakes.clone();
    let mut loader = Loader::spawn(Details::repo(dir.path().to_path_buf()), move || {
        w.fetch_add(1, Ordering::SeqCst);
    });
    loader.request(paths.clone());
    wait_for(&mut loader, paths.len(), Duration::from_secs(20));
    // A worker wakes the caller right after it sends a result.
    let t0 = Instant::now();
    while wakes.load(Ordering::SeqCst) < paths.len() && t0.elapsed() < Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(1));
    }
    std::thread::sleep(Duration::from_millis(20));
    assert_eq!(wakes.load(Ordering::SeqCst), paths.len());

    // Preloaded details answer at once, with a wake per result.
    let wakes = Arc::new(AtomicUsize::new(0));
    let w = wakes.clone();
    let details = Details::from_json(&fixture("quayside-details.json")).unwrap();
    let mut pre = Loader::spawn(details, move || {
        w.fetch_add(1, Ordering::SeqCst);
    });
    // The fixture has shapes and outlines for one directory, the berth component's sources; its other paths get empty details.
    let some: Vec<String> = Index::from_json(&fixture("quayside-files.json"))
        .unwrap()
        .files
        .iter()
        .filter(|f| f.path.starts_with("components/berth/"))
        .map(|f| f.path.clone())
        .collect();
    pre.request(some.clone());
    let got = pre.poll();
    assert_eq!(got.len(), some.len());
    assert_eq!(wakes.load(Ordering::SeqCst), some.len());
    assert!(
        got.iter()
            .any(|(_, d)| d.shape.is_some() && !d.outline.is_empty())
    );
    assert!(got.iter().any(|(_, d)| d == &FileDetails::default()));
    pre.request(some);
    assert!(pre.poll().is_empty());
}

#[test]
fn dropping_the_loader_joins_its_threads_within_1_s() {
    let dir = tempfile::tempdir().unwrap();
    let paths = temp_repo(dir.path(), 500);
    let mut loader = Loader::spawn(Details::repo(dir.path().to_path_buf()), || {});
    loader.request(paths);
    std::thread::sleep(Duration::from_millis(5));
    let t0 = Instant::now();
    drop(loader);
    assert!(t0.elapsed() < Duration::from_secs(1), "{:?}", t0.elapsed());
}
