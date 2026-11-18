//! The entry, `prism echo <source> <certificate> <record> [<score>]`
//! (docs/spec/echo.md §10.1; prism-022 S6; certificate §4.3).
//!
//! It reads its arguments and nothing else. By construction: the library
//! takes bytes and has no `std`, so only the binary can open a file, and
//! the binary's source opens exactly the paths it was given, once each,
//! with no environment, clock, network or process. The tests below run it
//! with an empty environment from an empty directory, with a nonexistent
//! extra path, and read its source for the calls it may not make.

use std::path::PathBuf;
use std::process::{Command, Output};

use echo::replay::{self, Inputs, BUDGET};

fn fixture(name: &str, file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../docs/spec/echo-vectors/mirror/fixtures")
        .join(name)
        .join(file)
}

fn prism(args: &[&std::ffi::OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_prism"))
        .args(args)
        .output()
        .expect("runs")
}

fn v01() -> [PathBuf; 3] {
    [
        fixture("pos-v01", "source.pact"),
        fixture("pos-v01", "certificate"),
        fixture("pos-v01", "record"),
    ]
}

fn echo_args(paths: &[PathBuf]) -> Vec<&std::ffi::OsStr> {
    let mut v: Vec<&std::ffi::OsStr> = vec!["echo".as_ref()];
    v.extend(paths.iter().map(|p| p.as_os_str()));
    v
}

fn stdout(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).expect("utf-8")
}

/// The binary prints what the library answers for the same bytes, with the
/// pinned budget, and exits with its code.
#[test]
fn the_binary_prints_the_library_answer() {
    let paths = v01();
    let o = prism(&echo_args(&paths));
    let bytes: Vec<Vec<u8>> = paths
        .iter()
        .map(|p| std::fs::read(p).expect("fixture"))
        .collect();
    let want = replay::replay(
        &Inputs {
            source: Some(&bytes[0]),
            certificate: Some(&bytes[1]),
            record: Some(&bytes[2]),
            score: None,
        },
        BUDGET,
    );
    assert_eq!(stdout(&o), want.print());
    assert!(stdout(&o).starts_with("unknown unreplayed source\nheard source e997e229"));
    assert_eq!(o.status.code(), Some(2));
    assert!(o.stderr.is_empty());
}

/// The same bytes with no environment at all, from an empty directory: the
/// binary depends on nothing but its arguments.
#[test]
fn nothing_but_the_arguments() {
    let paths = v01();
    let here = prism(&echo_args(&paths));
    let empty = std::env::temp_dir().join(format!("echo-bin-{}", std::process::id()));
    std::fs::create_dir_all(&empty).expect("temp dir");
    let mut there = Command::new(env!("CARGO_BIN_EXE_prism"));
    there
        .args(echo_args(&paths))
        .env_clear()
        .current_dir(&empty);
    // Under coverage the binary is instrumented: its profiling runtime (not
    // echo's code) writes where LLVM_PROFILE_FILE says, or into the cwd.
    // Keep that one variable so the empty directory stays empty.
    if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
        there.env("LLVM_PROFILE_FILE", profile);
    }
    let there = there.output().expect("runs");
    let left: Vec<_> = std::fs::read_dir(&empty).expect("temp dir").collect();
    std::fs::remove_dir_all(&empty).expect("temp dir");
    assert!(left.is_empty(), "the binary wrote into its cwd: {left:?}");
    assert_eq!(here.stdout, there.stdout);
    assert_eq!(here.status.code(), there.status.code());
}

/// A nonexistent score path is the input that couldn't be read, named and
/// `none` in `heard`; nothing else is opened in its place.
#[test]
fn a_nonexistent_path_is_unknown_io() {
    let mut paths = v01().to_vec();
    paths.push(fixture("pos-v01", "no-such-score"));
    let o = prism(&echo_args(&paths));
    let out = stdout(&o);
    let l: Vec<&str> = out.lines().collect();
    assert_eq!(l[0], "unknown io score");
    assert!(l[1].ends_with(" score none"), "{}", l[1]);
    assert!(l[2].starts_with("implements spec "));
    assert_eq!(o.status.code(), Some(2));
}

/// Five paths, the fifth nonexistent: wrong arity, so usage on stderr,
/// nothing on stdout, exit 2, and no path is read (contract delta, last
/// row). The same for two paths and for a first word other than `echo`.
#[test]
fn wrong_arguments_are_usage_and_read_nothing() {
    let mut five = v01().to_vec();
    five.push(fixture("pos-v01", "record"));
    five.push(fixture("pos-v01", "no-such-extra"));
    let three = v01();
    let two = three[..2].to_vec();
    let cases: Vec<Vec<&std::ffi::OsStr>> = vec![
        echo_args(&five),
        echo_args(&two),
        vec![],
        vec![
            "listen".as_ref(),
            three[0].as_os_str(),
            three[1].as_os_str(),
            three[2].as_os_str(),
        ],
    ];
    for args in cases {
        let o = prism(&args);
        assert!(o.stdout.is_empty(), "{args:?}");
        assert_eq!(
            String::from_utf8(o.stderr.clone()).expect("utf-8"),
            "usage: prism echo <source> <certificate> <record> [<score>]\n"
        );
        assert_eq!(o.status.code(), Some(2), "{args:?}");
    }
}

/// The binary's source makes none of the calls §4.3 forbids, and reads
/// files in one place only.
#[test]
fn the_binary_source_reads_only_its_arguments() {
    let src = include_str!("../src/bin/prism.rs");
    let code: String = src
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden in [
        "env::var",
        "env::vars",
        "current_dir",
        "std::net",
        "Command",
        "Instant",
        "SystemTime",
        "read_dir",
        "File::open",
        "OpenOptions",
        "include_",
    ] {
        assert!(!code.contains(forbidden), "{forbidden}");
    }
    assert_eq!(code.matches("fs::read(").count(), 1);
    assert!(code.contains("BUDGET"));
}
