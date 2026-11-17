//! Mara's Theorem 1 in prism (mirror `mara/ouroboros` 199aeaa7, §1.2, §6.1,
//! §12 item 8; prism-017 ask 8): a crossing carries a proof, seals, or both,
//! never neither. The admission policy is mirror's; prism only refuses the
//! anonymous shape and says which shape a claim has.

use prismqueer::algedonic::{Clock, Draft, Question, Region, Wald};
use prismqueer::brain::Crystal;
use prismqueer::crossing::{attest, Claim, Edge, Grade, Law, NotAProof, Refusal};
use prismqueer::oid::{Oid, Scope};

use num_bigint::BigInt;
use num_rational::BigRational;

/// A deterministic instrument: the input, twice.
struct Twice;

impl Law for Twice {
    fn oid(&self) -> Oid {
        Oid::new("twice")
    }
    fn apply(&self, input: &[u8]) -> Vec<u8> {
        [input, input].concat()
    }
}

fn crystal(bytes: &[u8]) -> Crystal {
    Crystal::settle(&Scope::public(), bytes.to_vec())
}

/// `twice(ab) = abab`: a replay that holds.
fn edge() -> (Edge, Oid) {
    let output = crystal(b"abab");
    let edge = Edge::replay(&Twice, &crystal(b"ab"), &output).expect("twice(ab) = abab");
    (edge, output.address().clone())
}

fn oid(s: &str) -> Oid {
    Oid::new(s)
}

#[test]
fn both_empty_is_refused() {
    assert_eq!(
        Claim::new(oid("v"), None, []),
        Err(Refusal::Anonymous),
        "no proof and no seal: anonymous, refused",
    );
}

#[test]
fn a_proof_alone_is_a_law() {
    let (edge, value) = edge();
    let claim = Claim::new(value.clone(), Some(edge.clone()), []).expect("a proof crosses");
    assert_eq!(claim.grade(), Grade::Law);
    assert_eq!(claim.proof(), Some(&edge));
    assert!(claim.seals().is_empty());
    assert_eq!(claim.value(), &value);
}

#[test]
fn seals_alone_are_sealed() {
    let v = oid("v");
    let claim = Claim::new(v.clone(), None, [attest(&v, oid("observer"))]).expect("a seal crosses");
    assert_eq!(claim.grade(), Grade::Sealed);
    assert_eq!(claim.proof(), None);
    assert_eq!(claim.seals().len(), 1);
}

#[test]
fn a_proof_and_seals_are_measured() {
    let (edge, value) = edge();
    let instrument = attest(&oid("twice"), oid("calibrator"));
    let claim = Claim::new(value, Some(edge), [instrument]).expect("a measurement crosses");
    assert_eq!(claim.grade(), Grade::Measured);
}

#[test]
fn the_edge_names_its_replay() {
    let (edge, value) = edge();
    assert_eq!(edge.from(), crystal(b"ab").address());
    assert_eq!(edge.via(), &oid("twice"));
    assert_eq!(edge.to(), &value);
}

#[test]
fn a_replay_that_disagrees_is_not_a_proof() {
    assert_eq!(
        Edge::replay(&Twice, &crystal(b"ab"), &crystal(b"aba")),
        Err(NotAProof),
        "twice(ab) is abab, not aba",
    );
}

#[test]
fn a_proof_of_another_value_is_refused() {
    let (edge, _) = edge();
    assert_eq!(
        Claim::new(
            oid("elsewhere"),
            Some(edge),
            [attest(&oid("elsewhere"), oid("o"))]
        ),
        Err(Refusal::Elsewhere),
    );
}

#[test]
fn several_signers_are_all_kept() {
    let v = oid("v");
    let seals = [
        attest(&v, oid("ana")),
        attest(&v, oid("bo")),
        attest(&v, oid("cy")),
    ];
    let claim = Claim::new(v, None, seals.clone()).expect("sealed");
    assert_eq!(claim.seals().len(), 3);
    for seal in &seals {
        assert!(claim.seals().contains(seal), "{seal:?} kept");
    }
}

#[test]
fn one_signer_twice_over_one_address_is_one_seal() {
    let v = oid("v");
    let claim = Claim::new(
        v.clone(),
        None,
        [attest(&v, oid("ana")), attest(&v, oid("ana"))],
    )
    .expect("sealed");
    assert_eq!(claim.seals().len(), 1, "a union is idempotent");
}

#[test]
fn a_trigger_rulers_signer_joins_as_it_is() {
    let q = |n: i64, d: i64| BigRational::new(BigInt::from(n), BigInt::from(d));
    let trigger = Draft::new(
        Region::above(q(1, 1)),
        Question::Max,
        Wald::new(q(1, 20), q(1, 20)).expect("a test"),
        Clock::agreed(10, 1, 100),
    )
    .seal(oid("ana"));
    let v = oid("v");
    let claim = Claim::new(
        v.clone(),
        None,
        [trigger.signer().clone(), attest(&v, oid("ana"))],
    )
    .expect("sealed");
    assert_eq!(
        claim.seals().len(),
        2,
        "one signer over two addresses: two seals"
    );
    assert!(claim.seals().contains(trigger.signer()));
    assert!(claim
        .seals()
        .iter()
        .any(|s| s.over() == &trigger.oid() && s.by() == &oid("ana")));
}
