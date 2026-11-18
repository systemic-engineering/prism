//! V0 integrity (prism-020 P4): seam note amendment 1, A1.3 and A1.7 P4
//! (mirror `4f6bdc17`), over the one pinned reflection format, the F_G
//! file (fixpoint §16.10 item 1).
//!
//! The correct case is mirror's own `mirror.facts` (data, copied byte for
//! byte): read on its own, and as the reflection below a seam. No mirror
//! file carries a reflection yet, so the seam case is built here.

use std::fs;
use std::path::PathBuf;

use echo::address::{self, Address};
use echo::integrity::{check_facts, check_reflection, Breach, HeaderFault, Integrity};
use echo::{Position, Refusal, Unreadable};

const GRAMMAR: &str = "9092fca18d8bc728f7f3467329c4a01d4073de2194fb7bce939ca739938857ee";

fn facts() -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../docs/spec/echo-vectors/mirror/mirror.facts");
    fs::read_to_string(p).expect("mirror.facts")
}

fn a32(s: &str) -> Address {
    core::array::from_fn(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).expect("hex"))
}

fn at(line: usize, column: usize) -> Position {
    Position { line, column }
}

/// Replace line `n` (from 1) of `text`.
fn with_line(text: &str, n: usize, new: &str) -> String {
    let mut lines: Vec<&str> = text.lines().collect();
    lines[n - 1] = new;
    lines.join("\n") + "\n"
}

fn without_line(text: &str, n: usize) -> String {
    let mut lines: Vec<&str> = text.lines().collect();
    lines.remove(n - 1);
    lines.join("\n") + "\n"
}

fn line(text: &str, n: usize) -> &str {
    text.lines().nth(n - 1).expect("line")
}

const ABOVE: &str = "-- a shard, above the seam\nshard <- 'x'\n";

/// `a ‖ "---\n" ‖ r`: the seam on line 3, so `r`'s line n is the file's n + 3.
fn reflected(r: &str) -> String {
    format!("{ABOVE}---\n{r}")
}

#[test]
fn mirror_facts_passes() {
    assert_eq!(
        check_facts(facts().as_bytes()),
        Ok(Integrity {
            grammar: a32(GRAMMAR),
            facts: 539,
            above: None
        })
    );
}

/// The same facts as a reflection: V0 passes and gives back `H(a)`, the
/// BLAKE3 of the bytes above the seam only (seam note §2.1).
#[test]
fn mirror_facts_as_a_reflection_passes() {
    let s = reflected(&facts());
    let got = check_reflection(s.as_bytes()).expect("passes");
    assert_eq!((got.grammar, got.facts), (a32(GRAMMAR), 539));
    assert_eq!(got.above, Some(address::hash(ABOVE.as_bytes())));
    assert_ne!(got.above, Some(address::hash(s.as_bytes())));
    // `r` is not in H(a): another reflection, the same H(a)
    let empty = format!(
        "{ABOVE}---\n# format-grammar {}\n# root {}\n# grammar {}\n",
        "0".repeat(40),
        "0".repeat(64),
        hex(&address::fragment(&item(), "grammar", &[]))
    );
    assert_eq!(
        check_reflection(empty.as_bytes()).map(|i| (i.above, i.facts)),
        Ok((got.above, 0))
    );
}

fn item() -> echo::Tau {
    echo::Tau::root("@parse/item").expect("tau")
}

fn hex(a: &[u8]) -> String {
    a.iter().map(|b| format!("{b:02x}")).collect()
}

/// The claimed F_G address is one hex digit off: refused at the claim.
#[test]
fn a_wrong_claimed_address_is_refused_at_the_claim() {
    let mut bad = GRAMMAR.to_string();
    bad.replace_range(63.., "f");
    let text = with_line(&facts(), 3, &format!("# grammar {bad}"));
    let want = |l| Breach::Address {
        at: at(l, 11),
        claimed: a32(&bad),
        computed: a32(GRAMMAR),
    };
    assert_eq!(check_facts(text.as_bytes()), Err(want(3)));
    assert_eq!(check_reflection(reflected(&text).as_bytes()), Err(want(6)));
}

/// A fact goes missing (the order still holds): the claim no longer
/// recomputes from what `r` lists.
#[test]
fn a_dropped_fact_is_refused_at_the_claim() {
    let text = without_line(&facts(), 300);
    match check_facts(text.as_bytes()) {
        Err(Breach::Address {
            at: p,
            claimed,
            computed,
        }) => {
            assert_eq!((p, claimed), (at(3, 11), a32(GRAMMAR)));
            assert_ne!(computed, claimed);
        }
        other => panic!("{other:?}"),
    }
}

/// The facts are a set listed strictly ascending by address (§16.10 item
/// 1, §5.0.1 items 1–2): a repeat, or two lines swapped, is refused at the
/// line that breaks the order.
#[test]
fn order_and_repeats_are_refused_at_their_line() {
    let f = facts();
    let repeated = with_line(&f, 101, line(&f, 100));
    assert_eq!(
        check_facts(repeated.as_bytes()),
        Err(Breach::Order {
            at: at(101, 1),
            repeated: true
        })
    );
    let mut lines: Vec<&str> = f.lines().collect();
    lines.swap(99, 100); // lines 100 and 101
    let swapped = lines.join("\n") + "\n";
    assert_eq!(
        check_facts(swapped.as_bytes()),
        Err(Breach::Order {
            at: at(101, 1),
            repeated: false
        })
    );
    assert_eq!(
        check_reflection(reflected(&swapped).as_bytes()),
        Err(Breach::Order {
            at: at(104, 1),
            repeated: false
        })
    );
}

/// The header (§16.10 item 1): three keys, in order, lowercase hex of
/// their lengths (§5.0.1 item 7).
#[test]
fn headers_are_well_formed() {
    let f = facts();
    let header = |text: &str| check_facts(text.as_bytes());
    // the grammar line is gone: the block ends before `grammar`
    assert_eq!(
        header(&without_line(&f, 3)),
        Err(Breach::Header {
            at: at(3, 1),
            fault: HeaderFault::Missing("grammar")
        })
    );
    // root and grammar swapped
    let mut lines: Vec<&str> = f.lines().collect();
    lines.swap(1, 2);
    assert_eq!(
        header(&(lines.join("\n") + "\n")),
        Err(Breach::Header {
            at: at(2, 3),
            fault: HeaderFault::Expected("root")
        })
    );
    // uppercase, short, and non-hex values
    let upper = format!("# grammar {}", GRAMMAR.to_uppercase());
    let short = format!("# root {}", &line(&f, 2)[7..70]);
    let sha1ish = format!("# format-grammar {}", "g".repeat(40));
    assert_eq!(
        header(&with_line(&f, 3, &upper)),
        Err(Breach::Header {
            at: at(3, 11),
            fault: HeaderFault::Hex
        })
    );
    assert_eq!(
        header(&with_line(&f, 2, &short)),
        Err(Breach::Header {
            at: at(2, 8),
            fault: HeaderFault::Hex
        })
    );
    assert_eq!(
        header(&with_line(&f, 1, &sha1ish)),
        Err(Breach::Header {
            at: at(1, 18),
            fault: HeaderFault::Hex
        })
    );
    // a fourth header
    let extra = f.replacen("\nfact ", "\n# answers none\nfact ", 1);
    assert_eq!(
        header(&extra),
        Err(Breach::Header {
            at: at(4, 3),
            fault: HeaderFault::Unexpected
        })
    );
    // no header at all
    assert_eq!(
        header("fact p\n"),
        Err(Breach::Header {
            at: at(1, 1),
            fault: HeaderFault::Missing("format-grammar")
        })
    );
}

/// No seam, no reflection to check; a reflection that doesn't read is
/// refused where it stops, in the whole file's coordinates.
#[test]
fn unreflected_and_unreadable() {
    assert_eq!(
        check_reflection(facts().as_bytes()),
        Err(Breach::Unreflected)
    );
    let s = reflected(&with_line(&facts(), 5, "rule p(?x) requires q(?x)"));
    assert_eq!(
        check_reflection(s.as_bytes()),
        Err(Breach::Unreadable(Refusal {
            at: at(8, 1),
            what: Unreadable::Form
        }))
    );
    assert!(matches!(
        check_facts(b"fact p(\n"),
        Err(Breach::Unreadable(_))
    ));
}

/// Every breach names itself and its place.
#[test]
fn breaches_print() {
    let shown = [
        Breach::Unreflected.to_string(),
        Breach::Unreadable(Refusal {
            at: at(8, 1),
            what: Unreadable::Form,
        })
        .to_string(),
        Breach::Header {
            at: at(3, 1),
            fault: HeaderFault::Missing("grammar"),
        }
        .to_string(),
        Breach::Header {
            at: at(2, 3),
            fault: HeaderFault::Expected("root"),
        }
        .to_string(),
        Breach::Header {
            at: at(4, 3),
            fault: HeaderFault::Unexpected,
        }
        .to_string(),
        Breach::Header {
            at: at(3, 11),
            fault: HeaderFault::Hex,
        }
        .to_string(),
        Breach::Order {
            at: at(101, 1),
            repeated: true,
        }
        .to_string(),
        Breach::Order {
            at: at(101, 1),
            repeated: false,
        }
        .to_string(),
        Breach::Address {
            at: at(3, 11),
            claimed: [0xab; 32],
            computed: a32(GRAMMAR),
        }
        .to_string(),
    ];
    assert_eq!(
        shown,
        [
            "unreflected: no seam".to_string(),
            "unreadable: 8:1 unknown line form".to_string(),
            "3:1 header: `grammar` is missing".to_string(),
            "2:3 header: `root` belongs here".to_string(),
            "4:3 header: a header after `grammar`".to_string(),
            "3:11 header: not lowercase hex of the key's length".to_string(),
            "101:1 a fact repeated".to_string(),
            "101:1 a fact out of ascending address order".to_string(),
            format!(
                "3:11 address: claimed {}, recomputed {GRAMMAR}",
                "ab".repeat(32)
            ),
        ]
    );
}
