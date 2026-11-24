//! The HOLD law (prism-024 §1; Mara, mirror-029 I.1 and mirror
//! `mara/bridge-grade` 3333dd80 §1; the five assertions are mirror-029 Ask 2
//! item 1). A decision sealed over premise projections stands while every
//! projection's address is the one sealed; otherwise it is HOLD, naming the
//! moved premises and the owner, and only the owner's seal over the current
//! tuple clears it. No feature: the law is in the default build.

use prismqueer::hold::{
    field, reseal, standing, Conflict, Current, Decision, DecisionDraft, Facts, Output, Premise,
    Projection, Refused, Standing, StatusAlone, Twice,
};
use prismqueer::oid::Oid;

fn oid(s: &str) -> Oid {
    Oid::new(s)
}

/// A derivation: `name` is `contradicted` once `refuter` is a fact, else
/// `unrefuted`. Its output carries the facts head beside the status, as
/// bullet-up-021's verdict file does.
fn derive(name: &str, refuter: &str, facts: &Facts) -> Output {
    let status = if facts.contains(&oid(refuter)) {
        "contradicted"
    } else {
        "unrefuted"
    };
    Output::new(oid(name), status).headed(facts.address())
}

fn facts<'a>(names: impl IntoIterator<Item = &'a str>) -> Facts {
    let mut facts = Facts::new();
    for name in names {
        facts.add(oid(name));
    }
    facts
}

/// The tiny graph of Ask 2: p1, p2, p3 refuted by err1, err2, err3.
fn outputs(facts: &Facts) -> Current {
    Current::new([
        derive("p1", "err1", facts),
        derive("p2", "err2", facts),
        derive("p3", "err3", facts),
    ])
    .expect("one output per premise")
}

fn seal<P: Projection>(rule: &str, reads: &[&str], current: &Current, owner: &str) -> Decision<P> {
    let premises = reads
        .iter()
        .map(|p| Premise::<P>::of(current.get(&oid(p)).expect("the premise is derived")));
    DecisionDraft::new(oid(rule), premises)
        .expect("each premise once")
        .seal(oid(owner))
}

/// D1 reads p1; D2 reads p1 and p2; D3 reads p3.
fn graph(current: &Current) -> [Decision; 3] {
    [
        seal("D1", &["p1"], current, "o1"),
        seal("D2", &["p1", "p2"], current, "o2"),
        seal("D3", &["p3"], current, "o3"),
    ]
}

// ---------------------------------------------------------------------------
// 1. An irrelevant fact leaves every seal
// ---------------------------------------------------------------------------

/// Early cutoff (Lemma H): twenty facts that refute nothing. Every output's
/// full address moves (its head, beside the status, moves with every fact),
/// and no projection does, so every decision stands.
#[test]
fn an_irrelevant_fact_leaves_every_seal() {
    let mut log = facts(["ok0"]);
    let before = outputs(&log);
    let decisions = graph(&before);
    for i in 1..=20 {
        log.add(oid(&format!("ok{i}")));
        let now = outputs(&log);
        for p in ["p1", "p2", "p3"] {
            let (was, is) = (before.get(&oid(p)).unwrap(), now.get(&oid(p)).unwrap());
            assert_ne!(was.address(), is.address(), "the head beside moved");
            assert_eq!(
                Premise::<StatusAlone>::of(was),
                Premise::<StatusAlone>::of(is),
                "the projection did not"
            );
        }
        for d in &decisions {
            assert_eq!(standing(d, &now), Standing::Stands, "fact ok{i}");
        }
    }
}

// ---------------------------------------------------------------------------
// 2. A relevant fact HOLDs exactly the dependents
// ---------------------------------------------------------------------------

/// `err1` moves p1 only: D1 and D2 HOLD, each naming its owner and p1; D3,
/// which never read p1, stands.
#[test]
fn a_relevant_fact_holds_exactly_the_dependents() {
    let mut log = facts(["ok0"]);
    let [d1, d2, d3] = graph(&outputs(&log));
    log.add(oid("err1"));
    let now = outputs(&log);
    assert_eq!(
        standing(&d1, &now),
        Standing::Hold {
            moved: vec![oid("p1")],
            owner: oid("o1")
        }
    );
    assert_eq!(
        standing(&d2, &now),
        Standing::Hold {
            moved: vec![oid("p1")],
            owner: oid("o2")
        }
    );
    assert_eq!(standing(&d3, &now), Standing::Stands);
}

/// Two premises moved: both are named, in the decision's declared order.
#[test]
fn a_hold_names_every_moved_premise() {
    let mut log = facts(["ok0"]);
    let [_, d2, d3] = graph(&outputs(&log));
    log.add(oid("err2"));
    log.add(oid("err1"));
    let now = outputs(&log);
    assert_eq!(
        standing(&d2, &now),
        Standing::Hold {
            moved: vec![oid("p1"), oid("p2")],
            owner: oid("o2")
        }
    );
    assert_eq!(standing(&d3, &now), Standing::Stands);
}

/// A declared premise with no current output counts as moved: its absence is
/// not the projection that was sealed.
#[test]
fn an_unread_premise_counts_as_moved() {
    let log = facts(["ok0"]);
    let [_, d2, _] = graph(&outputs(&log));
    let only_p2 = Current::new([derive("p2", "err2", &log)]).unwrap();
    assert_eq!(
        standing(&d2, &only_p2),
        Standing::Hold {
            moved: vec![oid("p1")],
            owner: oid("o2")
        }
    );
}

// ---------------------------------------------------------------------------
// 3. A stage move without a status move doesn't HOLD
// ---------------------------------------------------------------------------

/// The premise output's full address moves (λ and ρ moved), its status
/// doesn't, and under the status-alone projection the decision stands.
///
/// FLAGGED FOR ALEX: status alone is Mara's lean (mirror-029 I.1, "yours to
/// decide"), not a theorem. Under a projection that includes the stage, the
/// same move HOLDs (the second half of this test), which is Mara's kept
/// dissonance: someone may want to re-seal when ρ moved. That is a declared
/// ruler, never the default.
#[test]
fn a_stage_move_without_a_status_move_does_not_hold() {
    let early = Output::new(oid("h1"), "contradicted").at(1, 3);
    let late = Output::new(oid("h1"), "contradicted").at(2, 5);
    assert_ne!(early.address(), late.address(), "the full output moved");
    assert_eq!(late.stage(), (2, 5));
    assert_eq!(late.status(), "contradicted");

    let sealed = Current::new([early.clone()]).unwrap();
    let now = Current::new([late]).unwrap();
    let d: Decision = seal("ship", &["h1"], &sealed, "owner");
    assert_eq!(standing(&d, &now), Standing::Stands);

    let staged: Decision<WithStage> = seal("ship", &["h1"], &sealed, "owner");
    assert_eq!(
        standing(&staged, &now),
        Standing::Hold {
            moved: vec![oid("h1")],
            owner: oid("owner")
        }
    );
}

// ---------------------------------------------------------------------------
// 4. A blob carrying facts_head storms (documenting)
// ---------------------------------------------------------------------------

/// The status and the stage: the projection Mara's dissonance would declare.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WithStage;

impl Projection for WithStage {
    const NAME: &'static str = "status+stage";
    fn read(output: &Output, preimage: &mut Vec<u8>) {
        StatusAlone::read(output, preimage);
        let (lambda, rho) = output.stage();
        preimage.extend_from_slice(&lambda.to_le_bytes());
        preimage.extend_from_slice(&rho.to_le_bytes());
    }
}

/// The wrong projection: the facts head inside (bullet-up-021 §2 as written).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WithHead;

impl Projection for WithHead {
    const NAME: &'static str = "status+facts_head";
    fn read(output: &Output, preimage: &mut Vec<u8>) {
        StatusAlone::read(output, preimage);
        let head = output.head().map(|h| h.as_str().as_bytes()).unwrap_or(b"");
        field(preimage, head);
    }
}

/// The costly projection: the cites inside.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WithCites;

impl Projection for WithCites {
    const NAME: &'static str = "status+cites";
    fn read(output: &Output, preimage: &mut Vec<u8>) {
        StatusAlone::read(output, preimage);
        preimage.extend_from_slice(&(output.cites().len() as u64).to_le_bytes());
        for cite in output.cites() {
            field(preimage, cite.as_str().as_bytes());
        }
    }
}

/// Mara's B1 log (`bridge_probe.py`, `b1()`): twenty `span_ok_i`, with
/// `span_err_h1` inserted at 13 and a second refutation `span_err_h1b` at 17:
/// 22 facts, 21 appends after the first. The derivation: `contradicted` once
/// any `span_err` is a fact, citing every one present.
fn b1_log() -> Vec<String> {
    let mut log: Vec<String> = (0..20).map(|i| format!("span_ok_{i}")).collect();
    log.insert(13, "span_err_h1".into());
    log.insert(17, "span_err_h1b".into());
    log
}

fn b1_derive(facts: &Facts) -> Output {
    let cites: Vec<Oid> = facts
        .iter()
        .filter(|f| f.as_str().starts_with("span_err"))
        .cloned()
        .collect();
    let status = if cites.is_empty() {
        "unrefuted"
    } else {
        "contradicted"
    };
    Output::new(oid("h1"), status)
        .citing(cites)
        .headed(facts.address())
}

/// Replay the log through `standing`: seal after the first fact, and on every
/// HOLD count it and let the owner re-seal over the current tuple.
fn b1_holds<P: Projection>() -> usize {
    let log = b1_log();
    let mut facts = facts([log[0].as_str()]);
    let mut decision: Decision<P> = seal(
        "ship",
        &["h1"],
        &Current::new([b1_derive(&facts)]).unwrap(),
        "owner",
    );
    let mut holds = 0;
    for fact in &log[1..] {
        facts.add(oid(fact));
        let out = b1_derive(&facts);
        let now = Current::new([out]).unwrap();
        if let Standing::Hold { moved, owner } = standing(&decision, &now) {
            assert_eq!(moved, vec![oid("h1")]);
            holds += 1;
            let new = seal("ship", &["h1"], &now, owner.as_str());
            decision = reseal(&decision, new, &now).expect("the owner over the current tuple");
        }
    }
    holds
}

/// Documenting, never a design (Mara, mirror-029 I.1 and 3333dd80 §1.2 item
/// 3, B1: "21 HOLDs over 21 facts, against 1 for the verdict alone, and 2
/// with `cites` inside"). A facts head inside the projection moves on every
/// fact, so the decision HOLDs on every fact: the storm early cutoff
/// removes. Reproduced exactly through `standing`.
#[test]
fn a_blob_carrying_facts_head_storms() {
    assert_eq!(b1_log().len() - 1, 21, "21 facts after the first");
    assert_eq!(
        b1_holds::<WithHead>(),
        21,
        "facts_head inside: a HOLD per fact"
    );
    assert_eq!(b1_holds::<StatusAlone>(), 1, "status alone: the one flip");
    assert_eq!(
        b1_holds::<WithCites>(),
        2,
        "cites inside: the flip and the second refutation"
    );
}

/// Two projections of one output are two addresses: a decision sealed under
/// one never compares equal under another.
#[test]
fn a_projection_names_itself_in_its_address() {
    let out = Output::new(oid("h1"), "unrefuted");
    assert_ne!(
        Premise::<StatusAlone>::of(&out).address(),
        Premise::<WithCites>::of(&out).address()
    );
    assert_eq!(Premise::<StatusAlone>::of(&out).premise(), &oid("h1"));
}

// ---------------------------------------------------------------------------
// 5. Only a seal naming the new tuple clears
// ---------------------------------------------------------------------------

/// D1 HOLDs after err1. A re-seal by a non-owner, a re-seal over the old
/// tuple, and one over a tuple that isn't the current one are refused; the
/// owner's seal over the current tuple stands.
#[test]
fn only_the_owners_seal_over_the_new_tuple_clears() {
    let mut log = facts(["ok0"]);
    let old = outputs(&log);
    let [d1, _, _] = graph(&old);
    log.add(oid("err1"));
    let now = outputs(&log);
    assert!(matches!(standing(&d1, &now), Standing::Hold { .. }));

    let by_another = seal("D1", &["p1"], &now, "o2");
    assert_eq!(
        reseal(&d1, by_another, &now),
        Err(Refused::NotOwner {
            owner: oid("o1"),
            by: oid("o2")
        })
    );

    let over_old = seal("D1", &["p1"], &old, "o1");
    assert_eq!(reseal(&d1, over_old, &now), Err(Refused::OldTuple));

    let elsewhere = Current::new([Output::new(oid("p1"), "supported")]).unwrap();
    let over_other = seal("D1", &["p1"], &elsewhere, "o1");
    assert_eq!(
        reseal(&d1, over_other, &now),
        Err(Refused::NotCurrent {
            premises: vec![oid("p1")]
        })
    );

    let cleared = reseal(&d1, seal("D1", &["p1"], &now, "o1"), &now).expect("the owner, current");
    assert_eq!(standing(&cleared, &now), Standing::Stands);
    assert_eq!(cleared.owner(), &oid("o1"));
}

/// A seal over another rule, or other premise names, is another decision and
/// clears nothing.
#[test]
fn a_seal_over_another_decision_clears_nothing() {
    let mut log = facts(["ok0"]);
    let [d1, _, _] = graph(&outputs(&log));
    log.add(oid("err1"));
    let now = outputs(&log);
    let other_rule = seal("D9", &["p1"], &now, "o1");
    assert_eq!(reseal(&d1, other_rule, &now), Err(Refused::OtherDecision));
    let other_reads = seal("D1", &["p1", "p2"], &now, "o1");
    assert_eq!(reseal(&d1, other_reads, &now), Err(Refused::OtherDecision));
}

/// A standing decision re-sealed by its owner over the current tuple, which
/// is the sealed one, is the same decision again.
#[test]
fn a_standing_decision_reaffirmed_is_the_same_decision() {
    let now = outputs(&facts(["ok0"]));
    let [d1, _, _] = graph(&now);
    let again = reseal(&d1, seal("D1", &["p1"], &now, "o1"), &now).expect("current");
    assert_eq!(again, d1);
}

// ---------------------------------------------------------------------------
// The decision's address, and its parts
// ---------------------------------------------------------------------------

/// The signer sits beside the address (`docs/spec/ruler.md`): one judgment
/// sealed by two people is one address and two signers.
#[test]
fn the_owner_sits_beside_the_address() {
    let now = outputs(&facts(["ok0"]));
    let by_o1: Decision = seal("D1", &["p1"], &now, "o1");
    let by_o2: Decision = seal("D1", &["p1"], &now, "o2");
    assert_eq!(by_o1.address(), by_o2.address());
    assert_ne!(by_o1.signer(), by_o2.signer());
    assert_eq!(by_o1.signer().by(), &oid("o1"));
    assert_eq!(by_o1.signer().over(), &by_o1.address());
    assert_eq!(by_o1.rule(), &oid("D1"));
    assert_eq!(by_o1.premises().len(), 1);
    assert_eq!(by_o1.premises()[0].premise(), &oid("p1"));
}

/// The address covers the rule and the tuple, in order.
#[test]
fn the_address_covers_the_rule_and_the_tuple() {
    let now = outputs(&facts(["ok0"]));
    let a: Decision = seal("D2", &["p1", "p2"], &now, "o");
    let b: Decision = seal("D2", &["p2", "p1"], &now, "o");
    let c: Decision = seal("D9", &["p1", "p2"], &now, "o");
    assert_ne!(a.address(), b.address());
    assert_ne!(a.address(), c.address());
}

/// A premise named twice in one tuple is refused.
#[test]
fn a_premise_read_twice_is_refused() {
    let out = Output::new(oid("p1"), "unrefuted");
    let twice = DecisionDraft::<StatusAlone>::new(oid("D"), [Premise::of(&out), Premise::of(&out)]);
    assert_eq!(twice.err(), Some(Twice { premise: oid("p1") }));
}

/// Two different outputs for one premise are two addresses for one key
/// (shape (a)): refused. The same output twice is one.
#[test]
fn two_outputs_for_one_premise_conflict() {
    let a = Output::new(oid("p1"), "unrefuted");
    let b = Output::new(oid("p1"), "contradicted");
    assert_eq!(
        Current::new([a.clone(), b]).err(),
        Some(Conflict { premise: oid("p1") })
    );
    let once = Current::new([a.clone(), a.clone()]).unwrap();
    assert_eq!(once.get(&oid("p1")), Some(&a));
    assert_eq!(once.get(&oid("p2")), None);
}

/// An output's parts, and what its full address covers.
#[test]
fn an_output_carries_its_parts() {
    let head = facts(["x"]).address();
    let out = Output::new(oid("p"), "contradicted")
        .at(4, 7)
        .citing([oid("b"), oid("a")])
        .headed(head.clone());
    assert_eq!(out.premise(), &oid("p"));
    assert_eq!(out.cites(), &[oid("a"), oid("b")][..], "a set, ordered");
    assert_eq!(out.head(), Some(&head));
    let bare = Output::new(oid("p"), "contradicted").at(4, 7);
    assert_eq!(bare.head(), None);
    assert!(bare.cites().is_empty());
    assert_ne!(out.address(), bare.address());
    assert_ne!(
        bare.address(),
        bare.clone().citing([oid("a")]).address(),
        "cites in the full address"
    );
}

/// A fact set: added once, its address order-free and moving with every new
/// fact.
#[test]
fn a_fact_set_grows_and_its_address_moves() {
    let mut f = Facts::new();
    let empty = f.address();
    assert!(f.add(oid("b")));
    assert!(!f.add(oid("b")), "a fact is added once");
    assert!(f.add(oid("a")));
    assert_ne!(f.address(), empty);
    assert_eq!(f.len(), 2);
    assert!(!f.is_empty());
    assert!(Facts::new().is_empty());
    assert_eq!(f.address(), facts(["a", "b"]).address(), "order-free");
    assert_eq!(
        f.iter().cloned().collect::<Vec<_>>(),
        vec![oid("a"), oid("b")]
    );
}
