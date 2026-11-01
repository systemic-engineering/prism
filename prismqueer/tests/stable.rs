//! prism-012 Ask 1, decided by Alex (2026-11-01): `Stable` is a marker
//! trait, a proof-carrying refinement of `Prism`. A stable Prism's light only
//! adds and never twists. `Unstable` is the absence of the proof, not a type.
//!
//! Two behavioural laws, on every Prism marked `Stable` here:
//!
//! 1. **Lights only add.** On a growing input, nothing visible retracts: each
//!    output contains the one before, so the lums-like count (the size of the
//!    lit set) never falls.
//! 2. **Darkness is received, never made.** A stable Prism's own result adds
//!    no loss: over a `Partial` input it returns the same value as over the
//!    lit input, carrying exactly the loss it received.
//!
//! A control, not marked `Stable`, shows the first law has teeth: a
//! complement retracts on a growing input.

#![cfg(feature = "bundle")]

use prismqueer::{apply, Beam, Compose, IdentityPrism, Optic, Prism, ScalarLoss, Stable};
use std::collections::BTreeSet;
use terni::Loss;

type Set = BTreeSet<u32>;
type SetIn = Optic<(), Set>;

/// The first retraction a Prism shows over a chain of inputs, if any: the
/// step `i` at which output `i` doesn't contain output `i - 1`.
fn first_retraction<P>(prism: &P, chain: &[Set]) -> Option<usize>
where
    P: Prism<Input = SetIn>,
    P::Refracted: Beam<Out = Set>,
{
    let mut lit: Option<Set> = None;
    for (i, input) in chain.iter().enumerate() {
        let out = apply(prism, Optic::ok((), input.clone()))
            .result()
            .ok()
            .expect("a lit input stays lit")
            .clone();
        if let Some(before) = &lit {
            if !before.is_subset(&out) || out.len() < before.len() {
                return Some(i);
            }
        }
        lit = Some(out);
    }
    None
}

/// Law 1, which only a `Stable` Prism is asked to keep.
fn lights_only_add<P>(prism: &P, chain: &[Set])
where
    P: Stable<Input = SetIn>,
    P::Refracted: Beam<Out = Set>,
{
    assert_eq!(
        first_retraction(prism, chain),
        None,
        "a stable Prism retracted light on a growing input"
    );
}

/// Law 2: over a `Partial` input, the same value and exactly the received
/// loss. A stable Prism adds no darkness of its own.
fn darkness_is_received<P>(prism: &P, input: &Set, received: f64)
where
    P: Stable<Input = SetIn>,
    P::Refracted: Beam<Out = Set, Loss = ScalarLoss>,
{
    let lit = apply(prism, Optic::ok((), input.clone()));
    assert!(
        !lit.is_partial() && lit.is_ok(),
        "a stable Prism made darkness"
    );
    let dimmed = apply(
        prism,
        Optic::partial((), input.clone(), ScalarLoss::new(received)),
    );
    assert_eq!(dimmed.result().ok(), lit.result().ok(), "the light changed");
    match dimmed.result() {
        terni::Imperfect::Partial(_, loss) => assert_eq!(loss, ScalarLoss::new(received)),
        _ => panic!("received darkness was dropped or changed"),
    }
}

/// Growing chains of sets, from a small deterministic generator: each set
/// is the one before plus up to three new elements.
fn growing_chains() -> Vec<Vec<Set>> {
    let mut state: u64 = 0x5eed;
    let mut next = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 33) as u32
    };
    (0..64)
        .map(|_| {
            let mut s = Set::new();
            (0..8)
                .map(|_| {
                    for _ in 0..(next() % 4) {
                        s.insert(next() % 16);
                    }
                    s.clone()
                })
                .collect()
        })
        .collect()
}

/// Not `Stable`: the complement within 0..16. Negation, so it retracts.
struct Complement;

impl Prism for Complement {
    type Input = SetIn;
    type Focused = Optic<Set, Set>;
    type Projected = Optic<Set, Set>;
    type Refracted = Optic<Set, Set>;

    fn focus(&self, beam: Self::Input) -> Self::Focused {
        let s = beam.result().ok().expect("lit").clone();
        beam.next(s)
    }
    fn project(&self, beam: Self::Focused) -> Self::Projected {
        let s: Set = (0..16)
            .filter(|n| !beam.result().ok().expect("lit").contains(n))
            .collect();
        beam.next(s)
    }
    fn settle(&self, beam: Self::Projected) -> Self::Refracted {
        let s = beam.result().ok().expect("lit").clone();
        beam.next(s)
    }
}

#[test]
fn the_check_catches_a_complement_retracting() {
    let chains = growing_chains();
    assert!(
        chains
            .iter()
            .any(|chain| first_retraction(&Complement, chain).is_some()),
        "the law has no teeth: a complement never retracted"
    );
}

#[test]
fn identity_lights_only_add() {
    let id = IdentityPrism::<Set>::new();
    for chain in growing_chains() {
        lights_only_add(&id, &chain);
    }
}

#[test]
fn a_reference_to_a_stable_prism_lights_only_add() {
    let id = IdentityPrism::<Set>::new();
    for chain in growing_chains() {
        lights_only_add(&&id, &chain);
    }
}

#[test]
fn composed_stable_prisms_light_only_add() {
    let twice = Compose(IdentityPrism::<Set>::new(), IdentityPrism::<Set>::new());
    let thrice = Compose(
        Compose(IdentityPrism::<Set>::new(), IdentityPrism::<Set>::new()),
        IdentityPrism::<Set>::new(),
    );
    for chain in growing_chains() {
        lights_only_add(&twice, &chain);
        lights_only_add(&thrice, &chain);
    }
}

#[test]
fn stable_prisms_only_receive_darkness() {
    let input: Set = [1, 4, 9].into_iter().collect();
    let id = IdentityPrism::<Set>::new();
    darkness_is_received(&id, &input, 2.5);
    darkness_is_received(&&id, &input, 2.5);
    darkness_is_received(
        &Compose(IdentityPrism::<Set>::new(), IdentityPrism::<Set>::new()),
        &input,
        2.5,
    );
    darkness_is_received(
        &Compose(
            IdentityPrism::<Set>::new(),
            Compose(IdentityPrism::<Set>::new(), IdentityPrism::<Set>::new()),
        ),
        &input,
        0.5,
    );
}

// --- Compose as a pipeline, on Prisms that aren't stable --------------------

/// Not `Stable`: parses a number, and fails on anything else. Its own
/// result can be dark.
struct Parse;

impl Prism for Parse {
    type Input = Optic<(), String, String>;
    type Focused = Optic<String, String, String>;
    type Projected = Optic<String, String, String>;
    type Refracted = Optic<String, u32, String>;

    fn focus(&self, beam: Self::Input) -> Self::Focused {
        beam.smap(|s| terni::Imperfect::Success(s.trim().to_string()))
    }
    fn project(&self, beam: Self::Focused) -> Self::Projected {
        beam.smap(|s| terni::Imperfect::Success(s.clone()))
    }
    fn settle(&self, beam: Self::Projected) -> Self::Refracted {
        beam.smap(|s| match s.parse::<u32>() {
            Ok(n) => terni::Imperfect::Success(n),
            Err(_) => terni::Imperfect::Failure(format!("not a number: {s}"), ScalarLoss::new(1.0)),
        })
    }
}

/// Not `Stable` either: doubles, and dims anything odd.
struct Double;

impl Prism for Double {
    type Input = Optic<(), u32, String>;
    type Focused = Optic<u32, u32, String>;
    type Projected = Optic<u32, u32, String>;
    type Refracted = Optic<u32, u32, String>;

    fn focus(&self, beam: Self::Input) -> Self::Focused {
        beam.smap(|&n| {
            if n % 2 == 1 {
                terni::Imperfect::Partial(n, ScalarLoss::new(0.25))
            } else {
                terni::Imperfect::Success(n)
            }
        })
    }
    fn project(&self, beam: Self::Focused) -> Self::Projected {
        beam.smap(|&n| terni::Imperfect::Success(n * 2))
    }
    fn settle(&self, beam: Self::Projected) -> Self::Refracted {
        beam.smap(|&n| terni::Imperfect::Success(n))
    }
}

#[test]
fn compose_runs_the_first_prism_then_the_second() {
    let p = Compose(Parse, Double);
    let out = apply(&p, Optic::ok((), " 21 ".to_string()));
    assert_eq!(out.result().ok(), Some(&42));
    assert_eq!(out.result().loss(), ScalarLoss::new(0.25));
}

#[test]
fn compose_keeps_the_source_of_its_focus() {
    let p = Compose(Parse, Double);
    let focused = p.focus(Optic::ok((), " 8 ".to_string()));
    assert_eq!(focused.input(), &" 8 ".to_string());
    assert_eq!(focused.result().ok(), Some(&8));
}

#[test]
fn compose_carries_the_first_prisms_failure_through() {
    let p = Compose(Parse, Double);
    let out = apply(&p, Optic::ok((), "light".to_string()));
    assert!(out.is_err());
    match out.into_focus() {
        terni::Imperfect::Failure(e, loss) => {
            assert_eq!(e, "not a number: light");
            assert_eq!(loss, ScalarLoss::new(1.0));
        }
        other => panic!("expected the parse failure, got {other:?}"),
    }
}

#[test]
fn compose_carries_received_darkness_through() {
    let p = Compose(Parse, Double);
    let out = apply(&p, Optic::err((), "no input".to_string()));
    match out.into_focus() {
        terni::Imperfect::Failure(e, _) => assert_eq!(e, "no input"),
        other => panic!("expected the received failure, got {other:?}"),
    }
}

#[test]
fn compose_carries_received_loss_through() {
    let p = Compose(Parse, Double);
    let out = apply(
        &p,
        Optic::partial((), "3".to_string(), ScalarLoss::new(1.0)),
    );
    assert_eq!(out.result().ok(), Some(&6));
    assert_eq!(
        out.result().loss(),
        ScalarLoss::new(1.0).combine(ScalarLoss::new(0.25))
    );
}
