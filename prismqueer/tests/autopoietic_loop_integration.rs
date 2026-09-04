//! Integration test: full autopoietic 6-arrow loop per Alex 2026-09-04 PM Move 8+9.
//!
//! Exercises the entire prismqueer floor composition-chain that Reed shipped as
//! Discharges #5-#18 today:
//!
//! ```text
//! Reality → Recursion.tick → Observation → assert(Model) → Assertion →
//!   hypothesize → Hypothesis → compose(Chaos) → Question →
//!   [Choice from @subject] → Choice.into_reality → next Reality
//! ```
//!
//! Three sequential ticks demonstrating iterated autopoietic loop closure.
//!
//! # Composition-verified LANDED modules
//!
//! - `prismqueer::shard::Shard<T>` (Discharge #5 `22723bb`)
//! - `prismqueer::observer::Observer<N>` (Discharge #6 `55b8a76`)
//! - `prismqueer::chaos::ScalarChaos` (Discharge #7 `1350d60`)
//! - `prismqueer::crystal_shard::Crystal<T>` (Discharge #8 `30885b6`)
//! - `prismqueer::observation::Observation<T>` (Discharge #9 `30885b6`)
//! - `prismqueer::model::Model<T>` (Discharge #10 `30885b6`)
//! - `prismqueer::assertion::Assertion<T>` (Discharge #11 `30885b6`)
//! - `prismqueer::reality::Reality<T>` (Discharge #12 `30885b6`)
//! - `prismqueer::hypothesis::Hypothesis` (Discharge #13 `30885b6`)
//! - `prismqueer::question::Question` (Discharge #14 `30885b6`)
//! - `prismqueer::choice::Choice` (Discharge #15 `30885b6`)
//! - `prismqueer::recursion::Recursion<T>` (Discharge #16 `30885b6`)
//! - Fluent pipeline methods (Discharge #17 `efe7989`)
//! - Loop-closure methods (Discharge #18 `773c0ec`)

use prismqueer::assertion::Assertion;
use prismqueer::chaos::ScalarChaos;
use prismqueer::choice::Choice;
use prismqueer::crystal_shard::Crystal;
use prismqueer::hypothesis::KTQuestionShape;
use prismqueer::model::Model;
use prismqueer::observation::Observation;
use prismqueer::observer::{Observer, PeerObserver, TuringObserver, VoidObserver};
use prismqueer::question::Question;
use prismqueer::reality::Reality;
use prismqueer::recursion::Recursion;
use prismqueer::shard::Shard;
use terni::Loss;

/// Full 6-arrow autopoietic loop — ONE tick fluent chain.
#[test]
fn autopoietic_loop_one_tick_fluent_composition() {
    let observer: PeerObserver = Observer::new();
    let reality: Reality<&[u8]> = Reality::Settled(Crystal::new(b"seed"));
    let model: Model<&[u8]> = Model::empty();

    // Arrows 1-4: substrate side (fluent method-chain per Move 8+9)
    let question: Question = observer
        .observe(reality)              // arrow 1: Reality → Observation
        .assert(model)                  // arrow 2: + Model → Assertion
        .hypothesize()                  // arrow 3: → Hypothesis
        .compose(ScalarChaos::zero());  // arrow 4: + Chaos → Question

    assert_eq!(question.hypothesis.shape, KTQuestionShape::Reflexive);
}

/// Full 6-arrow loop — THREE sequential ticks with Choice-as-subject-input closing loop.
#[test]
fn autopoietic_loop_three_ticks_iterated_closure() {
    // VoidObserver would compose here at N=5 altitude if we wired
    // Observer.observe() into subject_driven_loop_tick; currently the three
    // Recursion.from_reality direct calls exercise the loop.
    let _observer: VoidObserver = Observer::new();

    // Tick 1: seed Reality
    let reality_0: Reality<String> = Reality::Settled(Crystal::new("initial".to_string()));
    let question_1: Question = Recursion::from_reality(reality_0)
        .subject_driven_loop_tick(Model::empty(), ScalarChaos::zero());
    assert_eq!(question_1.hypothesis.shape, KTQuestionShape::Reflexive);

    // Arrow 5: @subject inputs Choice (external subject-substrate boundary crossing)
    let choice_1 = Choice::new("observe");

    // Arrow 6: Choice → next Reality (via Choice.into_reality)
    let reality_1: Reality<String> = choice_1.into_reality();

    // Tick 2: fold subject-input into next Recursion
    let question_2: Question = Recursion::from_reality(reality_1)
        .subject_driven_loop_tick(Model::empty(), ScalarChaos::zero());
    assert_eq!(question_2.hypothesis.shape, KTQuestionShape::Reflexive);

    // Arrow 5 again: subject inputs another Choice
    let choice_2 = Choice::new("reflect");
    let reality_2: Reality<String> = choice_2.into_reality();

    // Tick 3: continues the loop
    let question_3: Question = Recursion::from_reality(reality_2)
        .subject_driven_loop_tick(Model::empty(), ScalarChaos::zero());
    assert_eq!(question_3.hypothesis.shape, KTQuestionShape::Reflexive);

    // The autopoietic loop demonstrably iterates: 3 Questions emitted; 2 Choices
    // folded back; all composing at type-level per Alex Move 9 subject-driven-loop
    // discipline.
}

/// Reality::Fractured (disconnected Shards) → Recursion.tick extracts first Shard
/// as observation-crystal per Move 8 minimum-viable elegant closure.
#[test]
fn autopoietic_loop_fractured_reality_extracts_first_shard() {
    let observer: PeerObserver = Observer::new();
    let shards: Vec<Shard<&[u8]>> = vec![
        Shard::new(b"first"),
        Shard::new(b"second"),
        Shard::new(b"third"),
    ];
    let reality: Reality<&[u8]> = Reality::Fractured(shards);

    let observation: Observation<&[u8]> = observer.observe(reality);
    assert_eq!(observation.crystal.payload, b"first");
}

/// All three Observer<N> dimensionality altitudes compose the same fluent-pipeline
/// shape: N=1 Turing tape + N=3 K_3 peer + N=5 K_5 @void basis per Alex Move 1.
#[test]
fn autopoietic_loop_all_three_canonical_observer_dimensions() {
    // N=1 Turing tape (degenerate; K_{1,n-1} pole)
    let _observer_1: TuringObserver = Observer::new();
    let question_1 = Recursion::from_reality(Reality::Settled(Crystal::new(b"t1" as &[u8])))
        .subject_driven_loop_tick(Model::<&[u8]>::empty(), ScalarChaos::zero());
    assert_eq!(question_1.hypothesis.shape, KTQuestionShape::Reflexive);

    // N=3 K_3 peer stability (past+now+future; @reality/object sufficient)
    let observer_3: PeerObserver = Observer::new();
    let _obs: Observation<&[u8]> = observer_3.observe(Reality::Settled(Crystal::new(b"t3" as &[u8])));

    // N=5 K_5 @void gauge basis (@reality/subject + hodobodo required)
    let observer_5: VoidObserver = Observer::new();
    let _obs: Observation<&[u8]> = observer_5.observe(Reality::Settled(Crystal::new(b"t5" as &[u8])));

    assert_eq!(Observer::<1>::dimensionality(), 1);
    assert_eq!(Observer::<3>::dimensionality(), 3);
    assert_eq!(Observer::<5>::dimensionality(), 5);
}

/// Assertion composed with Model carries Hawking model-dependent-reality EXPLICITLY.
#[test]
fn autopoietic_loop_hawking_model_dependent_reality_explicit_at_type_level() {
    // Hawking 2010 model-dependent-realism: no reality is ever observed
    // model-independently. Assertion carries BOTH observation AND model through
    // which the observation was made — per Alex Move 4 verbatim ("Hawking believed
    // we live an a model dependent reality. Let's make that explicit in prismqueer.").
    let observer: PeerObserver = Observer::new();
    let reality: Reality<&[u8]> = Reality::Settled(Crystal::new(b"reality"));
    let observation: Observation<&[u8]> = observer.observe(reality);

    let shard_model_a = Shard::new(b"model-frame-a" as &[u8]);
    let shard_model_b = Shard::new(b"model-frame-b" as &[u8]);
    let model: Model<&[u8]> = Model::new(vec![shard_model_a, shard_model_b]);

    let assertion: Assertion<&[u8]> = observation.assert(model);
    // Assertion carries BOTH observation AND the 2-shard model composition tree
    assert_eq!(assertion.model.shards.len(), 2);
}
