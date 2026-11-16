//! Every integration test file is a module of the one suite binary.
//!
//! prismqueer's integration tests link into one binary (`tests/main.rs`),
//! not one binary per file: each test binary costs a full link of the crate
//! and its dependencies, and macOS assesses every freshly linked binary on
//! its first launch (~2.5s each, measured 2026-11-16). With autotests off,
//! cargo no longer finds a new `tests/*.rs` by itself, so a file missing from
//! `tests/main.rs` would silently never run. This test is that guard.

use std::fs;
use std::path::Path;

#[test]
fn every_test_file_is_a_module_of_the_suite() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let main = fs::read_to_string(dir.join("main.rs")).unwrap_or_default();
    let declared: Vec<&str> = main
        .lines()
        .filter_map(|l| l.trim().strip_prefix("mod "))
        .filter_map(|l| l.strip_suffix(';'))
        .collect();
    let mut missing: Vec<String> = fs::read_dir(&dir)
        .expect("tests/ is readable")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .filter(|stem| stem != "main" && !declared.contains(&stem.as_str()))
        .collect();
    missing.sort();
    assert!(
        missing.is_empty(),
        "tests/*.rs not declared in prismqueer/tests/main.rs (they would never run): {missing:?}; \
         add `mod <name>;` for each"
    );
}

#[test]
fn the_suite_is_the_only_test_binary() {
    let manifest =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap();
    assert!(
        manifest.lines().any(|l| l.trim() == "autotests = false"),
        "prismqueer/Cargo.toml must set `autotests = false`, or every tests/*.rs \
         also builds (and runs) as its own binary"
    );
}
