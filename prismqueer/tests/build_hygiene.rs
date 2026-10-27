//! The build writes only into OUT_DIR, never into the source tree.
//!
//! Under nix the source is read-only, so a Fortran compiler that drops its
//! `.mod` files next to the crate fails the build (mirror hit this,
//! 2026-10-27). On a writable checkout the same bug is silent: the files
//! appear in `prismqueer/` and `.gitignore` hides them. This checks for them
//! directly, so it doesn't depend on file permissions.

#![cfg(feature = "lapack")]

#[test]
fn the_fortran_build_leaves_no_module_files_in_the_source_tree() {
    let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let stray: Vec<String> = std::fs::read_dir(crate_dir)
        .expect("crate dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "mod"))
        .map(|p| p.display().to_string())
        .collect();
    assert!(stray.is_empty(), "the build wrote into the source tree: {stray:?}");
}
