//! Source guards (docs/spec/mcp.md §1, §7): never a float, never serde.

use std::path::{Path, PathBuf};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("src/ is readable")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|x| x == "rs") {
            out.push(path);
        }
    }
}

/// Every line of `mcp/src` holding one of `needles` as a word.
fn hits(needles: &[&str]) -> (usize, Vec<String>) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    let mut found = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).expect("source is readable");
        for (n, line) in text.lines().enumerate() {
            let words = line.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'));
            if words.into_iter().any(|w| needles.contains(&w)) {
                found.push(format!("{}:{}: {}", file.display(), n + 1, line.trim()));
            }
        }
    }
    (files.len(), found)
}

/// Contract rule 7: no float anywhere in the crate, comments included.
#[test]
fn no_float_in_the_crate() {
    let (files, found) = hits(&["f64", "f32"]);
    assert!(files >= 2, "the scan found the sources");
    assert!(
        found.is_empty(),
        "a float in mcp/src:\n{}",
        found.join("\n")
    );
}

/// Alex: "No serde." Not in a `use`, not in a path.
#[test]
fn no_serde_in_the_crate() {
    let (_, found) = hits(&["serde", "serde_json", "miniserde", "nanoserde"]);
    assert!(found.is_empty(), "serde in mcp/src:\n{}", found.join("\n"));
}

/// The scan sees a word it is looking for (the guard can fail).
#[test]
fn the_scan_finds_what_it_looks_for() {
    let (_, found) = hits(&["MAX_DEPTH"]);
    assert!(!found.is_empty());
}
