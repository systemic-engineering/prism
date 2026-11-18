//! The address law `H` (prism-020 P2), against data from both sides.
//!
//! - mirror's §5.0 known-answer vectors: all 215 `.vector` files of
//!   `tests/checker/vectors` (Taut's, written from the spec text, digests
//!   checked with b3sum 1.8.7), copied byte for byte into
//!   docs/spec/echo-vectors/mirror (its README names the commit).
//! - prism's own vectors, docs/spec/echo-vectors/address.vectors, written by
//!   docs/spec/echo-vectors/address.py from the same §5.0 text with Python's
//!   `blake3` package and b3sum, never from echo.
//!
//! Spec: mirror `docs/specs/mirror-fixpoint-parser.md` §4.1 (fact store
//! bytes, the tags), §5.0, §5.0.1 items 1, 2, 10, 11, §16.10 item 1.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use echo::address::{self, Address, Repeated, Tau};
use echo::{read, Atom, Value};

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../docs/spec/echo-vectors")
}

fn unhex(s: &str) -> Vec<u8> {
    assert!(s.len() % 2 == 0, "odd hex {s}");
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

fn a32(s: &str) -> Address {
    unhex(s).try_into().expect("32 bytes")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// One record: `key  value` lines, keys repeatable (`note`, `fact`, `entry`).
struct Rec(Vec<(String, String)>);

impl Rec {
    fn get(&self, k: &str) -> Option<&str> {
        self.0.iter().find(|(x, _)| x == k).map(|(_, v)| v.as_str())
    }
    fn must(&self, k: &str) -> &str {
        self.get(k)
            .unwrap_or_else(|| panic!("{}: no {k}", self.name()))
    }
    fn all<'a>(&'a self, k: &'a str) -> impl Iterator<Item = &'a str> {
        self.0
            .iter()
            .filter(move |(x, _)| x == k)
            .map(|(_, v)| v.as_str())
    }
    fn name(&self) -> &str {
        self.get("vector").unwrap_or("?")
    }
    fn digest(&self) -> Address {
        a32(self.must("blake3"))
    }
}

/// Blank-line separated records; `#` lines are comments.
fn records(text: &str) -> Vec<Rec> {
    let mut out = Vec::new();
    let mut cur = Vec::new();
    for line in text.lines() {
        if line.starts_with('#') {
            continue;
        }
        if line.trim().is_empty() {
            if !cur.is_empty() {
                out.push(Rec(std::mem::take(&mut cur)));
            }
            continue;
        }
        let (k, v) = line.split_once(' ').unwrap_or((line, ""));
        cur.push((k.to_string(), v.trim_start().to_string()));
    }
    if !cur.is_empty() {
        out.push(Rec(cur));
    }
    out
}

/// mirror's 215 `.vector` files, by file stem.
fn mirror() -> Vec<Rec> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir().join("mirror"))
        .expect("vectors")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "vector"))
        .collect();
    files.sort();
    files
        .iter()
        .map(|p| {
            let mut r = records(&fs::read_to_string(p).expect("vector"));
            assert_eq!(r.len(), 1, "{}", p.display());
            r.remove(0)
        })
        .collect()
}

fn ours() -> Vec<Rec> {
    records(&fs::read_to_string(dir().join("address.vectors")).expect("address.vectors"))
}

/// `<altitude> root` or `<altitude> grammar <64 hex>`.
fn tau(s: &str) -> Tau {
    let w: Vec<&str> = s.split(' ').collect();
    match w.as_slice() {
        [name, "root"] => Tau::root(name),
        [name, "grammar", g] => Tau::under(name, a32(g)),
        _ => panic!("tau {s}"),
    }
    .unwrap_or_else(|| panic!("tau {s} refused"))
}

/// An atom in canonical text or bare pact-text, read by echo's reader.
fn atom(text: &str) -> Atom {
    let f = read(format!("fact {text}\n").as_bytes()).unwrap_or_else(|r| panic!("{text}: {r}"));
    assert_eq!(f.facts.len(), 1);
    f.facts[0].atom.clone()
}

fn label(r: &Rec) -> String {
    r.must("label").trim_matches('"').to_string()
}

/// Every vector's reference name: the stem up to its first `-` (`01`, `77a`).
fn by_ref(recs: &[Rec]) -> HashMap<String, Address> {
    recs.iter()
        .map(|r| {
            let n = r.name().split('-').next().expect("name").to_string();
            (n, r.digest())
        })
        .collect()
}

fn refs(field: &str, known: &HashMap<String, Address>) -> Vec<Address> {
    field
        .split(' ')
        .map(|t| t.trim_end_matches(','))
        .filter_map(|t| known.get(t).copied())
        .collect()
}

/// `H` over every vector's own preimage gives its `blake3` line: all 215.
#[test]
fn every_mirror_digest_reproduces() {
    let all = mirror();
    assert_eq!(all.len(), 215);
    for r in &all {
        let pre = unhex(r.must("preimage"));
        assert_eq!(pre.len().to_string(), r.must("length"), "{}", r.name());
        assert_eq!(hex(&address::hash(&pre)), r.must("blake3"), "{}", r.name());
    }
    // a `not-equal` vector's digest is never the digest it names
    let known = by_ref(&all);
    let names: HashMap<&str, Address> = all.iter().map(|r| (r.name(), r.digest())).collect();
    let mut negatives = 0;
    for r in &all {
        if let Some(other) = r.must("expect").strip_prefix("not-equal ") {
            assert_ne!(r.digest(), names[other], "{}", r.name());
            negatives += 1;
        }
    }
    assert_eq!(negatives, 10);
    assert!(known.len() > 200);
}

/// Every vector that states an atom (`atom` line) gets its address from
/// echo's own encoder: τ and the atom's text in, preimage and digest out.
/// A `(negative)` vector's digest is a known mistake, which echo must not
/// make; every other one is echo's answer exactly.
#[test]
fn every_mirror_atom_reproduces_from_its_text() {
    let mut n = 0;
    for r in mirror() {
        let Some(text) = r.get("atom") else { continue };
        let t = tau(r.must("tau"));
        let a = atom(text);
        if let Some(h) = r.get("tau_hex") {
            assert_eq!(hex(&t.bytes()), h, "{}", r.name());
        }
        let got = address::atom(&t, &a);
        if r.must("kind").contains("(negative)") {
            assert_ne!(got, r.digest(), "{}", r.name());
        } else {
            assert_eq!(hex(&address::atom_preimage(&t, &a)), r.must("preimage"), "{}", r.name());
            assert_eq!(got, r.digest(), "{}", r.name());
        }
        n += 1;
    }
    assert_eq!(n, 63);
}

/// Every `fact <pact-text> <hex>` line: that atom's address at the vector's τ.
#[test]
fn every_mirror_fact_line_reproduces() {
    let mut n = 0;
    for r in mirror() {
        for line in r.all("fact") {
            let (text, h) = line.rsplit_once(' ').expect("fact line");
            assert_eq!(hex(&address::atom(&tau(r.must("tau")), &atom(text))), h, "{}: {line}", r.name());
            n += 1;
        }
    }
    assert_eq!(n, 32);
}

/// Fragments, lenses and sets stated by their parts: children by vector
/// number, `members` as vector numbers or `&`-joined pact atoms, `fact`
/// lines, `entry` atoms, and 09/10's `n(nat i)` children.
#[test]
fn every_mirror_composite_reproduces_from_its_parts() {
    let all = mirror();
    let known = by_ref(&all);
    let mut n = 0;
    for r in &all {
        let kind = r.must("kind");
        let Some(l) = r.get("label").map(|_| label(r)) else { continue };
        let t = tau(r.must("tau"));
        let got = if let Some(c) = r.get("children") {
            let kids: Vec<Address> = if let Some(rest) = c.strip_prefix("child i = addr of n(nat i) at ") {
                let (at, range) = rest.split_once(", i = 0..").expect("range");
                let last: u64 = range.parse().expect("last");
                let ct = tau(at);
                (0..=last)
                    .map(|i| address::atom(&ct, &Atom { pred: "n".into(), args: vec![Value::Nat(i)] }))
                    .collect()
            } else if c == "none" {
                vec![]
            } else {
                let k = refs(c, &known);
                if k.is_empty() {
                    continue; // the certificate's named children: not stated as parts
                }
                k
            };
            address::fragment(&t, &l, &kids)
        } else if let Some(ts) = r.get("targets") {
            address::lens(&t, &l, &refs(ts, &known))
        } else if r.get("entry").is_some() {
            let kids: Vec<Address> = r.all("entry").map(|e| address::atom(&t, &atom(e))).collect();
            address::fragment(&t, &l, &kids)
        } else if let Some(m) = r.get("members") {
            let by_number = refs(m, &known);
            let members: Vec<Address> = if !by_number.is_empty() {
                by_number
            } else if m.is_empty() {
                vec![]
            } else {
                m.split(" & ").map(|a| address::atom(&tau(r.must("tau")), &atom(a))).collect()
            };
            address::set(&t, &l, &members).expect("a set")
        } else if kind == "certificate: facts" {
            // `fact <text> <hex>` lines, or a `facts` line of `&`-joined
            // atoms, or neither (35b: the empty set)
            let mut members: Vec<Address> = r
                .all("fact")
                .map(|f| a32(f.rsplit_once(' ').expect("fact").1))
                .collect();
            if let Some(fs) = r.get("facts").filter(|f| !f.starts_with("none")) {
                members.extend(fs.split(" & ").map(|a| address::atom(&t, &atom(a))));
            }
            address::set(&t, &l, &members).expect("a set")
        } else {
            continue;
        };
        if kind.contains("(negative)") {
            assert_ne!(got, r.digest(), "{}", r.name());
        } else {
            assert_eq!(hex(&got), r.must("blake3"), "{}", r.name());
        }
        n += 1;
    }
    assert_eq!(n, COMPOSITES);
}

/// mirror's composites that state all their parts: 13 fragments, 2 lenses,
/// the ε frame (25), 18 sets by `members` (16–18 and the result models) and
/// 16 `certificate: facts` sets. The 10 whole certificates name children
/// (`header layers …`) that only certificate replay derives; their digests
/// are still checked over their preimages above.
const COMPOSITES: usize = 50;

/// prism's own vectors (address.py, from the §5.0 text): every preimage and
/// every address.
#[test]
fn our_vectors_reproduce() {
    let recs = ours();
    let mut names: HashMap<String, Address> = HashMap::new();
    let mut n = 0;
    for r in &recs {
        let t = tau(r.must("tau"));
        let got = match r.must("kind") {
            "atom" => {
                let a = atom(r.must("atom"));
                assert_eq!(hex(&t.bytes()), r.must("tau_hex"), "{}", r.name());
                assert_eq!(hex(&a.content()), r.must("content"), "{}", r.name());
                assert_eq!(hex(&address::atom_preimage(&t, &a)), r.must("preimage"), "{}", r.name());
                address::atom(&t, &a)
            }
            kind @ ("fragment" | "lens" | "set") => {
                let kids: Vec<Address> = match r.must("children") {
                    "none" => vec![],
                    c => c.split(' ').map(|k| names[k]).collect(),
                };
                match kind {
                    "fragment" => {
                        assert_eq!(hex(&address::fragment_preimage(&t, &label(r), &kids)), r.must("preimage"));
                        address::fragment(&t, &label(r), &kids)
                    }
                    "lens" => {
                        assert_eq!(hex(&address::lens_preimage(&t, &label(r), &kids)), r.must("preimage"));
                        address::lens(&t, &label(r), &kids)
                    }
                    _ => address::set(&t, &label(r), &kids).expect("a set"),
                }
            }
            "set (F_G)" => {
                let src = fs::read(dir().join("mirror/mirror.facts")).expect("mirror.facts");
                let f = read(&src).expect("reads");
                let kids: Vec<Address> = f.facts.iter().map(|x| address::atom(&t, &x.atom)).collect();
                assert_eq!(kids.len().to_string(), r.must("k"));
                address::set(&t, &label(r), &kids).expect("a set")
            }
            k => panic!("{}: kind {k}", r.name()),
        };
        assert_eq!(hex(&got), r.must("blake3"), "{}", r.name());
        names.insert(r.name().to_string(), got);
        n += 1;
    }
    assert_eq!(n, 19);
    // and the F_G it closes on is mirror's own header
    assert_eq!(
        hex(&names["p30-mirror-facts-grammar"]),
        "9092fca18d8bc728f7f3467329c4a01d4073de2194fb7bce939ca739938857ee"
    );
}

const HEXNAME: &str = "abababababababababababababababababababababababababababababababab";

/// §5.0.1 item 11 and §4.1: every argument is `tag ‖ enc(bytes)` with tags
/// sym 0x00, nat 0x01, addr 0x02, and bytes 0x03 (§16.10 item 1). So a
/// typed `addr` and a plain name with the same 64 hex digits are two atoms
/// with two addresses; so are `nat 7` and `sym "7"`, and `bytes "2f"` and
/// `sym "2f"`.
#[test]
fn typed_and_plain_have_different_addresses() {
    let t = Tau::root("@parse/item").expect("tau");
    let typed = atom(&format!("node(addr {HEXNAME}, seq)"));
    let plain = atom(&format!("node({HEXNAME}, seq)"));
    let quoted = atom(&format!("node(sym \"{HEXNAME}\", seq)"));
    assert_eq!(plain, quoted, "a bare name is sym \"<name>\" (§16.10 item 1)");
    assert_eq!(address::atom(&t, &plain), address::atom(&t, &quoted));
    assert_ne!(address::atom(&t, &typed), address::atom(&t, &plain));
    for (a, b) in [("q(nat 7)", "q(sym \"7\")"), ("q(bytes \"2f\")", "q(sym \"2f\")"), ("q(bytes \"2f\")", "q(nat 47)")] {
        assert_ne!(address::atom(&t, &atom(a)), address::atom(&t, &atom(b)), "{a} vs {b}");
    }
    // the bytes differ exactly at the tag
    let (pa, pp) = (address::atom_preimage(&t, &typed), address::atom_preimage(&t, &plain));
    let at = pa.iter().zip(&pp).position(|(x, y)| x != y).expect("they differ");
    assert_eq!((pa[at], pp[at]), (0x02, 0x00));
}

/// A one-byte change anywhere in an encoding moves the address: every
/// byte of 02's preimage, flipped in turn, and one-byte changes of the
/// content, τ and ℓ.
#[test]
fn one_byte_moves_the_address() {
    let v = mirror().into_iter().find(|r| r.name() == "02-atom-sym-nat-addr").expect("02");
    let pre = unhex(v.must("preimage"));
    let base = address::hash(&pre);
    assert_eq!(base, v.digest());
    for i in 0..pre.len() {
        let mut p = pre.clone();
        p[i] ^= 0x01;
        assert_ne!(address::hash(&p), base, "byte {i}");
    }
    let t = Tau::root("@datalog/fact").expect("tau");
    assert_ne!(address::atom(&t, &atom("p(sym \"a\")")), address::atom(&t, &atom("p(sym \"b\")")));
    assert_ne!(address::atom(&t, &atom("p(nat 6)")), address::atom(&t, &atom("p(nat 7)")));
    let u = Tau::root("@datalog/facu").expect("tau");
    assert_ne!(address::atom(&t, &atom("p")), address::atom(&u, &atom("p")));
    assert_ne!(address::fragment(&t, "shard", &[]), address::fragment(&t, "share", &[]));
}

/// τ (§5.0): the name, NUL, then the flag; a NUL in the name is refused.
#[test]
fn tau_bytes_and_nul() {
    let g: Address = core::array::from_fn(|i| i as u8);
    assert_eq!(hex(&Tau::root("@mirror").expect("tau").bytes()), "406d6972726f720000");
    let under = Tau::under("@x", g).expect("tau");
    assert_eq!(under.bytes()[..4], [b'@', b'x', 0x00, 0x01]);
    assert_eq!(under.bytes()[4..], g);
    assert_eq!((under.name(), under.grammar()), ("@x", Some(&g)));
    assert_eq!(Tau::root("@mirror").expect("tau").grammar(), None);
    assert_eq!(Tau::root("@a\0b"), None);
    assert_eq!(Tau::under("@a\0b", g), None);
}

/// Fragments and lenses keep their order; a set doesn't care about the
/// order it's handed, and refuses a repeated member (§5.0.1 item 2).
#[test]
fn order_kind_and_sets() {
    let t = Tau::root("@mirror").expect("tau");
    let (x, y) = ([1u8; 32], [2u8; 32]);
    assert_ne!(address::fragment(&t, "f", &[x, y]), address::fragment(&t, "f", &[y, x]));
    assert_ne!(address::fragment(&t, "f", &[x, y]), address::lens(&t, "f", &[x, y]));
    assert_eq!(address::set(&t, "s", &[y, x]), Ok(address::fragment(&t, "s", &[x, y])));
    assert_eq!(address::set(&t, "s", &[x, y, x]), Err(Repeated(x)));
    assert_eq!(address::set(&t, "s", &[]), Ok(address::fragment(&t, "s", &[])));
}
