//! The certified path builds without a float (docs/spec/features.md).
//!
//! `exact` compiles room's exact half, field, gl, poly, agreement, metrix,
//! measured, hypothesis, kleinos and discoball with no `cc`, no
//! LAPACK/BLAS/Fortran and no native build step; `lapack` adds the measured
//! readings on top (`<module>/reading.rs`, each behind `lapack`). Contract
//! rule 7: the certified path is exact.
//!
//! - **The scan** (every build): no `f64` or `f32` in the source of an exact
//!   module outside its comments, and every exact module declared under
//!   `exact` in lib.rs, never under `lapack`.
//! - **The path** (`exact`, and nothing else needed): agreement, measured,
//!   metrix and room, end to end over ℤ and ℚ. `cargo test -p prismqueer
//!   --no-default-features --features exact` runs it, and
//!   `checks.prismqueer-exact` builds the same feature set without the
//!   native environment.

use std::fs;
use std::path::Path;

/// The exact modules: what `exact` compiles, each a file of `src/`. Their
/// float readings, where they have any, live in `src/<module>/reading.rs`.
const EXACT: [&str; 10] = [
    "room",
    "field",
    "gl",
    "poly",
    "agreement",
    "metrix",
    "measured",
    "hypothesis",
    "kleinos",
    "discoball",
];

fn source(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Each `f64` or `f32` in `code` not preceded by a letter or digit (so
/// `to_f64` counts, `xf64` doesn't), as (line, text); comment lines skipped.
fn floats(code: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for (n, line) in code.lines().enumerate() {
        if line.trim_start().starts_with("//") {
            continue;
        }
        let bytes = line.as_bytes();
        for (i, _) in line.match_indices('f') {
            let token = bytes.get(i + 1..i + 3);
            let after = bytes.get(i + 3).copied();
            let before = i.checked_sub(1).map(|j| bytes[j]);
            if matches!(token, Some(b"64") | Some(b"32"))
                && !before.is_some_and(|b| b.is_ascii_alphanumeric())
                && !after.is_some_and(|b| b.is_ascii_digit())
            {
                found.push((n + 1, line.trim().to_owned()));
            }
        }
    }
    found
}

#[test]
fn the_scan_finds_a_float_where_there_is_one() {
    let code =
        "// f64 in a comment\nlet x: f64 = 1.0;\nlet y = n.to_f32();\nlet z = xf64 + f640;\n";
    assert_eq!(
        floats(code),
        [
            (2, "let x: f64 = 1.0;".to_owned()),
            (3, "let y = n.to_f32();".to_owned())
        ]
    );
}

#[test]
fn no_exact_module_holds_a_float() {
    let found: Vec<String> = EXACT
        .iter()
        .flat_map(|m| {
            let file = format!("{m}.rs");
            floats(&source(&file))
                .into_iter()
                .map(move |(n, l)| format!("src/{file}:{n}: {l}"))
        })
        .collect();
    assert!(
        found.is_empty(),
        "a float in an exact module (move it to src/<module>/reading.rs, behind `lapack`):\n{}",
        found.join("\n")
    );
}

#[test]
fn every_exact_module_is_declared_under_exact() {
    let lib = source("lib.rs");
    let lines: Vec<&str> = lib.lines().collect();
    for m in EXACT {
        let decl = format!("pub mod {m};");
        let at = lines
            .iter()
            .position(|l| l.trim() == decl)
            .unwrap_or_else(|| panic!("lib.rs declares no `{decl}`"));
        let gate = at.checked_sub(1).map(|i| lines[i].trim());
        // poly needs no feature at all: exact, and in every build.
        let allowed = if m == "poly" {
            gate != Some("#[cfg(feature = \"lapack\")]")
        } else {
            gate == Some("#[cfg(feature = \"exact\")]")
        };
        assert!(allowed, "`{decl}` is gated by {gate:?}, not `exact`");
    }
}

#[test]
fn every_float_reading_of_an_exact_module_is_behind_lapack() {
    for m in EXACT {
        let code = source(&format!("{m}.rs"));
        let lines: Vec<&str> = code.lines().collect();
        for (i, l) in lines.iter().enumerate() {
            if l.trim() == "mod reading;" {
                assert_eq!(
                    i.checked_sub(1).map(|j| lines[j].trim()),
                    Some("#[cfg(feature = \"lapack\")]"),
                    "src/{m}.rs:{}: `mod reading;` outside `lapack`",
                    i + 1
                );
            }
        }
    }
}

#[cfg(feature = "exact")]
mod path {
    use num_bigint::BigInt;
    use num_rational::BigRational;
    use prismqueer::agreement::{check, Relations, Selection, SelectionDraft, Verdict};
    use prismqueer::algedonic::{Clock, Draft, Question, Region, Ruler, Wald};
    use prismqueer::field::Graph;
    use prismqueer::hypothesis::Frame;
    use prismqueer::measured::{measure, Estimator, Method, Support};
    use prismqueer::metrix::{Form, Inertia};
    use prismqueer::oid::Oid;
    use prismqueer::room::{laplacian, Q, Z};

    fn z(x: i64) -> Z {
        Z::from(x)
    }

    fn q(n: i64, d: i64) -> Q {
        Q::new(n, d).expect("non-zero denominator")
    }

    fn selection(readings: &[(u64, i64)]) -> Selection<Z> {
        SelectionDraft::new(readings.iter().map(|&(v, x)| (v, z(x)))).seal(Oid::new("adjudicator"))
    }

    /// agreement: the consistent triangle 0 → 1 (+3), 1 → 2 (+4), 0 → 2
    /// (+7) over ℤ. (0, 3, 7) is In; (0, 3, 8) is Out, on 1–2 and 0–2 alike
    /// (one of them, whichever the check names, but never In).
    #[test]
    fn agreement_checks_a_selection_exactly() {
        let relations =
            Relations::new(&(), [], [(0, 1, z(3)), (1, 2, z(4)), (0, 2, z(7))]).unwrap();
        assert!(matches!(
            check(&relations, &selection(&[(0, 0), (1, 3), (2, 7)])),
            Ok(Verdict::In(_))
        ));
        assert!(matches!(
            check(&relations, &selection(&[(0, 0), (1, 3), (2, 8)])),
            Ok(Verdict::Out(_))
        ));
    }

    /// measured → metrix: the two-stream sample (1,2), (2,2), (3,5), (4,7),
    /// MLE, full support. S = [[5/4, 9/4], [9/4, 9/2]], K = S⁻¹ =
    /// [[8, −4], [−4, 20/9]], LDLᵀ: d = (8, 2/9) (tests/measured.rs works it
    /// by hand). Sealed by the window's own ruler, it is a Metrix.
    #[test]
    fn measured_estimates_and_metrix_certifies_exactly() {
        let window: Ruler = Draft::new(
            Region::above(BigRational::from_integer(BigInt::from(0))),
            Question::Max,
            Wald::new(
                BigRational::new(1.into(), 20.into()),
                BigRational::new(1.into(), 20.into()),
            )
            .expect("valid rates"),
            Clock::agreed(10, 1, 100),
        )
        .seal(Oid::new("window-keeper"));
        let frame = Frame::new(Oid::new("streams"), 2);
        let counts: Vec<Vec<Z>> = [[1, 2], [2, 2], [3, 5], [4, 7]]
            .iter()
            .map(|r| r.iter().map(|&n| z(n)).collect())
            .collect();
        let method = Method {
            estimator: Estimator::Mle,
            support: Support::full(2),
        };
        let m = measure(&window, &frame, &counts, method).expect("measurable");
        let k = vec![vec![q(8, 1), q(-4, 1)], vec![q(-4, 1), q(20, 9)]];
        assert_eq!(m.estimate(), k);
        let draft = m.draft().expect("positive definite");
        assert_eq!(draft.certificate().d(), [q(8, 1), q(2, 9)]);
        let metrix = m.seal(&window).expect("the window's own ruler");
        assert_eq!(metrix.rows(), k);
        assert_eq!(metrix.frame(), frame.oid());
    }

    /// metrix: a Form of any signature, read by Sylvester's inertia, exactly.
    #[test]
    fn metrix_reads_a_forms_inertia_exactly() {
        let form = Form::new(vec![
            vec![q(1, 1), q(0, 1), q(0, 1)],
            vec![q(0, 1), q(-1, 1), q(0, 1)],
            vec![q(0, 1), q(0, 1), q(0, 1)],
        ])
        .expect("symmetric");
        assert_eq!(
            form.inertia(),
            Inertia {
                positive: 1,
                negative: 1,
                zero: 1
            }
        );
    }

    /// room: the triangle's L₀ = δ*δ has charpoly x(x − 3)² = x³ − 6x² + 9x,
    /// exactly, with no eigen reading.
    #[test]
    fn room_reads_a_spectrum_exactly() {
        let triangle = Graph::new([0, 1, 2], [(0, 1), (1, 2), (0, 2)]);
        assert_eq!(
            laplacian(&triangle).charpoly().coefficients(),
            [z(1), z(-6), z(9), z(0)]
        );
    }
}
