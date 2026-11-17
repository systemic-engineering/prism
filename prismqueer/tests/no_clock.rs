//! No clock in the library (docs/spec/checks.md, to-fix 1–4 and 7).
//!
//! A gate verdict is cacheable only if it is a deterministic function of its
//! key, so nothing the gate runs may read the wall clock or a timer. This scans
//! `prismqueer/src` for reads of `SystemTime::now` and `Instant::now`. Timing
//! lives in `prismqueer/benches` (`just bench`), which the gate doesn't run.
//!
//! A test, not a `just` recipe: it runs inside `checks.test` and
//! `checks.coverage`, keyed on `testFileset` (which declares `src/**/*.rs`), so
//! the push gate and CI both enforce it. A recipe would only guard the
//! unkeyed local loop.

use std::path::{Path, PathBuf};

/// The clock reads the scan looks for.
const CLOCK_READS: [&str; 2] = ["SystemTime::now", "Instant::now"];

/// Files under `src/` allowed to read a clock. Empty: every timing claim is a
/// bench under `prismqueer/benches`, outside `src/`.
const ALLOWED: [&str; 0] = [];

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

#[test]
fn the_library_reads_no_clock() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    let mut reads = Vec::new();
    for file in &files {
        let rel = file
            .strip_prefix(&src)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        if ALLOWED.contains(&rel.as_str()) {
            continue;
        }
        let text = std::fs::read_to_string(file).expect("source is readable");
        for (n, line) in text.lines().enumerate() {
            if CLOCK_READS.iter().any(|c| line.contains(c)) {
                reads.push(format!("src/{rel}:{}: {}", n + 1, line.trim()));
            }
        }
    }
    assert!(
        !files.is_empty(),
        "the scan found no sources under {}",
        src.display()
    );
    assert!(
        reads.is_empty(),
        "the library reads a clock; take a seed from the caller, or move the timing to \
         prismqueer/benches:\n{}",
        reads.join("\n")
    );
}
