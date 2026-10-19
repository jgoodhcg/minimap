//! Generated-file marks (unit `041`): the built-in lock-file names, `linguist-generated` in `.gitattributes`, and the config's rules.

use minimap_core::generated::{LOCK_FILES, Marks, Rules, is_lock_file};

fn rules(patterns: &[&str]) -> Rules {
    Rules::gitignore(&patterns.iter().map(|p| p.to_string()).collect::<Vec<_>>()).unwrap()
}

fn marks(config: &[&str], attributes: &[(&str, &str)]) -> Marks {
    let mut m = Marks::new(&config.iter().map(|p| p.to_string()).collect::<Vec<_>>()).unwrap();
    for (path, text) in attributes {
        m.add_attributes(path, text);
    }
    m
}

#[test]
fn built_in_names_match_exactly() {
    assert_eq!(LOCK_FILES.len(), 16);
    for name in LOCK_FILES {
        assert!(is_lock_file(name), "{name}");
        assert!(
            is_lock_file(&format!("web/app/{name}")),
            "{name} in a directory"
        );
    }
    for path in [
        "my-package-lock.json",
        "yarn.lock.bak",
        "package-lock.json/readme.md",
        "Cargo.toml",
        "go.mod",
        "locks/uv.lock.txt",
    ] {
        assert!(!is_lock_file(path), "{path}");
    }
    let m = Marks::new(&[]).unwrap();
    assert!(m.is_generated("web/yarn.lock"));
    assert!(!m.is_generated("web/config.json"));
}

#[test]
fn gitattributes_read_only_linguist_generated() {
    let text = "\
# generated code
*.pb.go linguist-generated
api/*.ts linguist-generated=true
schema.json linguist-generated=yes
yarn.lock -linguist-generated
Cargo.lock linguist-generated=false
go.sum !linguist-generated
*.md text eol=lf
[attr]gen linguist-generated
!*.keep linguist-generated
build/ linguist-generated
\"odd name.txt\" linguist-generated
";
    let m = marks(&[], &[(".gitattributes", text)]);
    let yes = [
        "a.pb.go",
        "deep/dir/b.pb.go",
        "api/client.ts",
        "schema.json",
        "x/schema.json",
        "odd name.txt",
        // `!` returns go.sum to unspecified, and the built-in list marks it.
        "go.sum",
    ];
    for p in yes {
        assert!(m.is_generated(p), "{p}");
    }
    let no = [
        // `-` and `=false` unmark built-in names.
        "yarn.lock",
        "Cargo.lock",
        "web/Cargo.lock",
        // `api/*.ts` is anchored at the file's directory, and `*` does not cross `/`.
        "web/api/client.ts",
        "api/v1/client.ts",
        // Other attributes, macros, negative patterns, and directory patterns mark nothing.
        "README.md",
        "x.keep",
        "build/out.js",
        "gen",
    ];
    for p in no {
        assert!(!m.is_generated(p), "{p}");
    }
}

#[test]
fn a_later_line_and_a_deeper_file_win() {
    let root = "*.json linguist-generated\nkeep.json -linguist-generated\n";
    let web = "*.json -linguist-generated\nweb.gen.json linguist-generated\n";
    let m = marks(
        &[],
        &[("web/.gitattributes", web), (".gitattributes", root)],
    );
    assert!(m.is_generated("a.json"));
    assert!(
        !m.is_generated("keep.json"),
        "a later line overrides an earlier one"
    );
    assert!(
        !m.is_generated("web/a.json"),
        "a deeper file overrides the root's"
    );
    assert!(m.is_generated("web/web.gen.json"));
    assert!(m.is_generated("web/sub/web.gen.json"));
    // A pattern with a `/` is relative to its file's directory.
    let m = marks(
        &[],
        &[(
            "web/.gitattributes",
            "/top.json linguist-generated\nsub/*.ts linguist-generated\n",
        )],
    );
    assert!(m.is_generated("web/top.json"));
    assert!(!m.is_generated("web/x/top.json"));
    assert!(!m.is_generated("top.json"));
    assert!(m.is_generated("web/sub/a.ts"));
    assert!(!m.is_generated("sub/a.ts"));
}

#[test]
fn config_rules_are_gitignore_rules() {
    let r = rules(&[
        "*.min.js",
        "/dist.js",
        "schema/*.json",
        "gen/",
        "vendor",
        "docs/**/api.md",
    ]);
    let yes = [
        "a.min.js",
        "web/static/b.min.js",
        "dist.js",
        "schema/a.json",
        "gen/a.ts",
        "web/gen/deep/b.ts",
        "vendor",
        "lib/vendor/x.js",
        "docs/api.md",
        "docs/v1/v2/api.md",
    ];
    for p in yes {
        assert_eq!(r.decide(p), Some(Some(true)), "{p}");
    }
    let no = [
        "min.js.map",
        "web/dist.js",
        "web/schema/a.json",
        "schema/deep/a.json",
        // `gen/` matches directories only, not a file named gen.
        "gen",
        "docs/api.mdx",
    ];
    for p in no {
        assert_eq!(r.decide(p), None, "{p}");
    }
}

#[test]
fn a_bang_unmarks_and_the_last_rule_wins() {
    let m = marks(&["*.json", "!package-lock.json", "!keep.json"], &[]);
    assert!(
        !m.is_generated("package-lock.json"),
        "a rule unmarks a built-in name"
    );
    assert!(!m.is_generated("web/package-lock.json"));
    assert!(m.is_generated("yarn.lock"), "other built-in names stay");
    assert!(m.is_generated("a.json"));
    assert!(!m.is_generated("deep/keep.json"));
    let m = marks(&["vendor/", "!vendor/keep.js"], &[]);
    assert!(m.is_generated("vendor/a.js"));
    assert!(
        !m.is_generated("vendor/keep.js"),
        "a file under a marked directory can be unmarked"
    );
    let m = marks(&["!yarn.lock", "yarn.lock"], &[]);
    assert!(m.is_generated("yarn.lock"));
}

#[test]
fn the_config_overrides_gitattributes_which_overrides_the_built_in_list() {
    let attrs = [(
        ".gitattributes",
        "yarn.lock -linguist-generated\n*.snap linguist-generated\n",
    )];
    let m = marks(&[], &attrs);
    assert!(!m.is_generated("yarn.lock"));
    assert!(m.is_generated("a.snap"));
    let m = marks(&["yarn.lock", "!*.snap"], &attrs);
    assert!(m.is_generated("yarn.lock"));
    assert!(!m.is_generated("a.snap"));
}

#[test]
fn a_bad_rule_is_an_error_that_names_it() {
    let err = Marks::new(&["ok.js".into(), "bad[.js".into()]).unwrap_err();
    assert!(format!("{err:#}").contains("bad[.js"), "{err:#}");
    assert!(Marks::new(&["!".into()]).is_err());
    assert!(Marks::new(&["/".into()]).is_err());
    // A bad line in `.gitattributes` is skipped, and the others still count.
    let m = marks(
        &[],
        &[(
            ".gitattributes",
            "bad[.js linguist-generated\n*.gen linguist-generated\n",
        )],
    );
    assert!(m.is_generated("a.gen"));
    assert!(!m.is_generated("bad[.js"));
}
