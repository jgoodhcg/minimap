//! Line shapes and file details (unit `007`).

use minimap_core::IndexFile;
use minimap_core::details::{Details, FileDetails};
use minimap_core::shape::{LineShape, ShapeRow};

fn row(indent: u16, len: u16) -> ShapeRow {
    ShapeRow { indent, len }
}

#[test]
fn a_short_file_keeps_one_row_per_line() {
    let text = "(ns a)\n\n(defn f\n  [x]\n\t\tx)   \n\n;; end\n  \n(def g 1)\n(def h 2)";
    let s = LineShape::from_text(text);
    assert_eq!(s.lines, 10);
    assert_eq!(
        s.rows,
        vec![
            row(0, 6),
            row(0, 0),
            row(0, 7),
            row(2, 5),
            // Tabs count 2 columns, and trailing spaces do not count.
            row(4, 6),
            row(0, 0),
            row(0, 6),
            row(0, 0),
            row(0, 9),
            row(0, 9),
        ]
    );
}

/// 1,000 lines: line k has indent k % 8 and length indent + 10 + k % 200, capped at 126.
fn thousand_lines() -> String {
    let lines: Vec<String> = (0..1000)
        .map(|k| {
            let indent = k % 8;
            format!("{}{}", " ".repeat(indent), "x".repeat(10 + k % 200))
        })
        .collect();
    lines.join("\n")
}

/// The lower median (nearest rank) of some values.
fn lower_median(mut v: Vec<u16>) -> u16 {
    v.sort_unstable();
    v[(v.len() - 1) / 2]
}

#[test]
fn a_long_file_samples_64_rows_of_median_indent_and_length() {
    // Each row covers 15 or 16 lines, more than 4, so it keeps the medians (unit 028).
    let s = LineShape::from_text(&thousand_lines());
    assert_eq!(s.lines, 1000);
    assert_eq!(s.rows.len(), 64);
    for (r, got) in s.rows.iter().enumerate() {
        let (a, b) = (r * 1000 / 64, (r + 1) * 1000 / 64);
        let indent = lower_median((a..b).map(|k| (k % 8) as u16).collect());
        let len = lower_median(
            (a..b)
                .map(|k| (k % 8 + 10 + k % 200).min(126) as u16)
                .collect(),
        );
        assert_eq!(*got, row(indent, len), "row {r}");
    }
}

#[test]
fn without_the_median_a_long_file_keeps_min_indent_and_max_length() {
    let s = LineShape::from_text_with(&thousand_lines(), usize::MAX);
    for (r, got) in s.rows.iter().enumerate() {
        let (a, b) = (r * 1000 / 64, (r + 1) * 1000 / 64);
        let indent = (a..b).map(|k| k % 8).min().unwrap() as u16;
        let len = (a..b)
            .map(|k| (k % 8 + 10 + k % 200).min(126))
            .max()
            .unwrap() as u16;
        assert_eq!(*got, row(indent, len), "row {r}");
    }
}

#[test]
fn alternating_short_and_long_lines_give_medians_not_extremes() {
    // 1,000 lines alternate `x` at indent 0 and 100 x's at indent 8: min and max would be 0 and 108.
    let lines: Vec<String> = (0..1000)
        .map(|k| {
            if k % 2 == 0 {
                "x".to_string()
            } else {
                format!("{}{}", " ".repeat(8), "x".repeat(100))
            }
        })
        .collect();
    let s = LineShape::from_text(&lines.join("\n"));
    for (r, got) in s.rows.iter().enumerate() {
        // A row of 15 or 16 lines takes the short line's values when short lines are more than half, else the long line's.
        let (a, b) = (r * 1000 / 64, (r + 1) * 1000 / 64);
        let short = (a..b).filter(|k| k % 2 == 0).count();
        let want = if short > (b - a - 1) / 2 {
            row(0, 1)
        } else {
            row(8, 108)
        };
        assert_eq!(*got, want, "row {r}");
    }
    let s = LineShape::from_text_with(&lines.join("\n"), usize::MAX);
    assert!(s.rows.iter().all(|r| *r == row(0, 108)));
}

#[test]
fn a_row_of_five_lines_with_a_blank_takes_the_medians_of_the_other_four() {
    // 320 lines make 64 rows of 5. Row 0: a blank line, then indents 0, 2, 4, 6 and lengths 10, 20, 30, 40.
    let mut lines = vec![
        String::new(),
        "x".repeat(10),
        format!("  {}", "x".repeat(18)),
        format!("    {}", "x".repeat(26)),
        format!("      {}", "x".repeat(34)),
    ];
    lines.extend((5..320).map(|_| "y".to_string()));
    let s = LineShape::from_text(&lines.join("\n"));
    assert_eq!(s.rows.len(), 64);
    assert_eq!(s.rows[0], row(2, 20));
    assert_eq!(s.rows[1], row(0, 1));
}

#[test]
fn empty_text_has_no_rows() {
    assert_eq!(LineShape::from_text("").rows, vec![]);
}

#[test]
fn decodes_the_fake_repo_m_field() {
    // `AE` = indent 0, length 8; `CB` = indent 4, length 2; `AA` = a blank row.
    let s = LineShape::decode("AECBAA", 3).unwrap();
    assert_eq!(s.rows, vec![row(0, 8), row(4, 2), row(0, 0)]);
    assert_eq!(s.lines, 3);
    assert!(LineShape::decode("AEC", 3).is_none());
    assert!(LineShape::decode("A!", 1).is_none());
}

#[test]
fn an_unregistered_language_has_a_shape_but_no_outline() {
    let d = FileDetails::compute("web/app.ts", b"export function f() {\n  return 1;\n}\n");
    assert!(d.outline.is_empty());
    assert_eq!(d.namespace, None);
    assert_eq!(d.shape.unwrap().lines, 3);
    assert_eq!(
        FileDetails::compute("a.bin", b"\0\x01\x02"),
        FileDetails::default()
    );
}

#[test]
fn repo_details_load_on_demand() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src/a")).unwrap();
    std::fs::write(dir.path().join("src/a/b.clj"), "(ns a.b)\n(defn f [] 1)\n").unwrap();
    let mut details = Details::repo(dir.path().to_path_buf());
    let file = IndexFile::new("src/a/b.clj");
    assert!(details.get(&file.path).is_none(), "nothing before load");
    details.load(&file);
    let d = details.get(&file.path).unwrap();
    assert_eq!(d.namespace.as_deref(), Some("a.b"));
    assert_eq!(d.outline.len(), 1);
    // A missing file has empty details instead of an error.
    let gone = IndexFile::new("src/gone.clj");
    details.load(&gone);
    assert_eq!(details.get("src/gone.clj"), Some(&FileDetails::default()));
}

#[test]
fn json_details_come_by_path() {
    let json = r#"{"root":"r","files":[
        {"p":"a.clj","n":2,"m":"AEAC","o":[[1,"defn","f"],[2,"defn-","g"]],"ns":"a"},
        {"p":"b.md","n":1}
    ]}"#;
    let d = Details::from_json(json).unwrap();
    let a = d.get("a.clj").unwrap();
    assert_eq!(a.namespace.as_deref(), Some("a"));
    assert_eq!(a.shape.as_ref().unwrap().rows, vec![row(0, 8), row(0, 4)]);
    assert_eq!(a.outline[1].name, "g");
    assert!(a.outline[1].private && !a.outline[0].private);
    assert!(d.get("b.md").is_none());
}

#[test]
fn details_keep_every_measured_line_beside_the_sampled_shape() {
    // Unit 039: one row of 4 bytes per line, uncapped, and the 64-row shape samples the same rows.
    let text = thousand_lines();
    let d = FileDetails::compute("a.clj", text.as_bytes());
    assert_eq!(d.measured.len(), 1000);
    assert_eq!(std::mem::size_of::<ShapeRow>(), 4);
    assert_eq!(d.measured[0], row(0, 10));
    assert_eq!(d.measured[199], row(7, 7 + 10 + 199));
    assert_eq!(d.shape, Some(LineShape::from_text(&text)));
    // Binary content has neither.
    let bin = FileDetails::compute("a.png", &[0u8, 1, 2, 0, 9]);
    assert!(bin.measured.is_empty() && bin.shape.is_none());
}
