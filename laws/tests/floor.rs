//! The floor holds no float entry (docs/spec/laws.md, "Judges").
//!
//! "No float entry" is not "no float anywhere": a judge takes a measured
//! value as an argument, beside the law, and reads it once as the exact
//! dyadic rational it is. The rule, over every file of `src/`, outside
//! comments:
//!
//! - `f64` appears only as the type of a parameter of a named judge
//!   ([`JUDGES`]), so never as an `Entry`, a field, a return type, a cast,
//!   a generic argument or a local;
//! - each such parameter appears in its judge's body exactly once, as the
//!   argument of `BigRational::from_float`, so nothing computes in it;
//! - `f32` and a `Float` bound appear nowhere.
//!
//! The judges are a fixed list: a new one is a visible change here, never
//! a file exemption.

use std::fs;
use std::path::{Path, PathBuf};

/// A judge: the file it lives in (under `src/`), its name, and its float
/// parameters, in order.
struct Judge {
    file: &'static str,
    name: &'static str,
    floats: &'static [&'static str],
}

/// Every function of the floor that may take a float.
const JUDGES: [Judge; 3] = [
    Judge {
        file: "poly/judge.rs",
        name: "locate",
        floats: &["measured", "radius"],
    },
    Judge {
        file: "poly/judge.rs",
        name: "identify",
        floats: &["measured", "radius"],
    },
    Judge {
        file: "algedonic.rs",
        name: "of_floats",
        floats: &["lo", "hi"],
    },
];

/// The code of a source, comments dropped: whole comment lines, and the
/// rest of a line from `//` on.
fn code(source: &str) -> String {
    source
        .lines()
        .map(|line| match line.find("//") {
            Some(i) => &line[..i],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Byte offsets of `word` in `code` as a whole word.
fn words(code: &str, word: &str) -> Vec<usize> {
    let bytes = code.as_bytes();
    code.match_indices(word)
        .map(|(i, _)| i)
        .filter(|&i| {
            let before = i.checked_sub(1).map(|j| bytes[j]);
            let after = bytes.get(i + word.len()).copied();
            !before.is_some_and(is_word) && !after.is_some_and(is_word)
        })
        .collect()
}

/// Byte offsets of each float type token: `f64` or `f32` not preceded by a
/// letter or digit and not followed by a digit (so `to_f64` counts).
fn float_tokens(code: &str, ty: &str) -> Vec<usize> {
    let bytes = code.as_bytes();
    code.match_indices(ty)
        .map(|(i, _)| i)
        .filter(|&i| {
            let before = i.checked_sub(1).map(|j| bytes[j]);
            let after = bytes.get(i + ty.len()).copied();
            !before.is_some_and(|b| b.is_ascii_alphanumeric())
                && !after.is_some_and(|b| b.is_ascii_digit())
        })
        .collect()
}

/// A judge's signature (`fn name(...)`) and body (`{...}`), as byte ranges.
fn judge_at(code: &str, name: &str) -> Option<(std::ops::Range<usize>, std::ops::Range<usize>)> {
    let start = code.find(&format!("fn {name}("))?;
    let open = start + code[start..].find('{')?;
    let mut depth = 0usize;
    for (i, c) in code[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((start..open, open..open + i + 1));
                }
            }
            _ => {}
        }
    }
    None
}

/// What breaks the rule in one file's code, given the judges in it.
fn violations(file: &str, code: &str, judges: &[&Judge]) -> Vec<String> {
    let mut found = Vec::new();
    let line = |at: usize| code[..at].lines().count().max(1);
    for ty in ["f32"] {
        for at in float_tokens(code, ty) {
            found.push(format!("{file}:{}: `{ty}`", line(at)));
        }
    }
    for at in words(code, "Float") {
        found.push(format!("{file}:{}: a `Float` bound", line(at)));
    }
    // Each judge's float parameters: their `f64` tokens are allowed, and
    // each is read once, through from_float.
    let mut allowed = Vec::new();
    for judge in judges {
        let Some((signature, body)) = judge_at(code, judge.name) else {
            found.push(format!("{file}: no judge `{}`", judge.name));
            continue;
        };
        let sig = &code[signature.clone()];
        for at in float_tokens(sig, "f64") {
            let param = sig[..at].trim_end().strip_suffix(':').map(|s| {
                let s = s.trim_end();
                let start = s.rfind(|c: char| !(c.is_alphanumeric() || c == '_'));
                &s[start.map_or(0, |i| i + 1)..]
            });
            match param {
                Some(p) if judge.floats.contains(&p) => allowed.push(signature.start + at),
                _ => found.push(format!(
                    "{file}:{}: `f64` in `{}`'s signature, not as a judged parameter",
                    line(signature.start + at),
                    judge.name
                )),
            }
        }
        let declared = float_tokens(sig, "f64").len();
        if declared != judge.floats.len() {
            found.push(format!(
                "{file}: `{}` takes {declared} float(s), the list says {}",
                judge.name,
                judge.floats.len()
            ));
        }
        let text = &code[body.clone()];
        for p in judge.floats {
            let uses = words(text, p);
            let read = format!("BigRational::from_float({p})");
            let once = uses.len() == 1 && text.matches(&read).count() == 1;
            if !once {
                found.push(format!(
                    "{file}: `{}` reads `{p}` {} time(s), not once through `{read}`",
                    judge.name,
                    uses.len()
                ));
            }
        }
    }
    for at in float_tokens(code, "f64") {
        if !allowed.contains(&at) {
            found.push(format!(
                "{file}:{}: `f64` outside a judge's parameters",
                line(at)
            ));
        }
    }
    found
}

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.expect("a directory entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|x| x == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn the_floor_holds_a_float_only_as_a_judges_argument() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&src, &mut files);
    assert!(files.len() > 10, "the scan reads the floor's sources");
    let mut found = Vec::new();
    let mut judged = 0;
    for path in &files {
        let file = path
            .strip_prefix(&src)
            .expect("under src")
            .to_string_lossy()
            .replace('\\', "/");
        let source = fs::read_to_string(path).unwrap_or_else(|e| panic!("{file}: {e}"));
        let judges: Vec<&Judge> = JUDGES.iter().filter(|j| j.file == file).collect();
        judged += judges.len();
        found.extend(violations(&file, &code(&source), &judges));
    }
    assert_eq!(judged, JUDGES.len(), "every judge's file is in src/");
    assert!(
        found.is_empty(),
        "the floor holds a float outside the judges' rule (docs/spec/laws.md):\n{}",
        found.join("\n")
    );
}

/// No clock in the floor either (docs/spec/checks.md, to-fix 1–4 and 7, as
/// prismqueer's `no_clock` holds for the lab): a verdict is cacheable only
/// if it is a deterministic function of its key.
#[test]
fn the_floor_reads_no_clock() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&src, &mut files);
    assert!(files.len() > 10, "the scan reads the floor's sources");
    let reads: Vec<String> = files
        .iter()
        .flat_map(|path| {
            let text = fs::read_to_string(path).expect("source is readable");
            let file = path.display().to_string();
            text.lines()
                .enumerate()
                .filter(|(_, l)| {
                    ["SystemTime::now", "Instant::now"]
                        .iter()
                        .any(|c| l.contains(c))
                })
                .map(|(n, l)| format!("{file}:{}: {}", n + 1, l.trim()))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(
        reads.is_empty(),
        "the floor reads a clock:\n{}",
        reads.join("\n")
    );
}

// --- The scan, on code written to break it ---------------------------------

const JUDGE: Judge = Judge {
    file: "x.rs",
    name: "judge",
    floats: &["m"],
};

fn scan(code_text: &str) -> Vec<String> {
    violations("x.rs", &code(code_text), &[&JUDGE])
}

const LAWFUL: &str =
    "pub fn judge(m: f64) -> Option<Q> {\n    BigRational::from_float(m).map(Q)\n}\n";

#[test]
fn a_lawful_judge_passes() {
    assert_eq!(scan(LAWFUL), Vec::<String>::new());
    // A comment may say f64, f32 and Float.
    assert_eq!(
        scan(&format!("// f64 f32 Float\n{LAWFUL}/// x: f64\n")),
        Vec::<String>::new()
    );
}

#[test]
fn a_float_entry_is_refused() {
    let found = scan(&format!("{LAWFUL}impl Entry for f64 {{}}\n"));
    assert_eq!(found, ["x.rs:4: `f64` outside a judge's parameters"]);
}

#[test]
fn a_stored_returned_or_cast_float_is_refused() {
    for line in [
        "pub struct S { x: f64 }",
        "pub fn f() -> f64 { 0.0 }",
        "fn g(n: &BigInt) { n.to_f64(); }",
        "fn h(x: f64) {}",
        "fn k() -> Vec<f64> { vec![] }",
    ] {
        let found = scan(&format!("{LAWFUL}{line}\n"));
        assert_eq!(found.len(), 1, "{line}: {found:?}");
        assert!(found[0].contains("outside a judge's parameters"), "{line}");
    }
}

#[test]
fn a_judge_that_computes_in_its_float_is_refused() {
    let twice =
        "pub fn judge(m: f64) -> Q {\n    let _ = m * 2.0;\n    BigRational::from_float(m)\n}\n";
    assert_eq!(
        scan(twice),
        ["x.rs: `judge` reads `m` 2 time(s), not once through `BigRational::from_float(m)`"]
    );
    let unread = "pub fn judge(m: f64) -> Q {\n    Q::zero(m)\n}\n";
    assert_eq!(scan(unread).len(), 1);
}

#[test]
fn a_judge_that_returns_a_float_or_takes_an_unlisted_one_is_refused() {
    let returns = "pub fn judge(m: f64) -> f64 {\n    BigRational::from_float(m)\n}\n";
    let found = scan(returns);
    assert!(
        found
            .iter()
            .any(|f| f.contains("not as a judged parameter")),
        "{found:?}"
    );
    let extra = "pub fn judge(m: f64, r: f64) -> Q {\n    BigRational::from_float(m)\n}\n";
    let found = scan(extra);
    assert!(
        found.iter().any(|f| f.contains("takes 2 float(s)")),
        "{found:?}"
    );
}

#[test]
fn f32_and_a_float_bound_are_refused_anywhere() {
    assert_eq!(
        scan(&format!("{LAWFUL}fn f(x: f32) {{}}\n")),
        ["x.rs:4: `f32`"]
    );
    assert_eq!(
        scan(&format!(
            "{LAWFUL}impl<T: num_traits::Float> Entry for T {{}}\n"
        )),
        ["x.rs:4: a `Float` bound"]
    );
}

#[test]
fn a_missing_judge_is_refused() {
    assert_eq!(scan("pub fn other() {}\n"), ["x.rs: no judge `judge`"]);
}
