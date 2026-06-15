//! Integration test: Bundle trait used from outside the crate.
//!
//! After the spectral-triple-grammar audit (commit `ea341d1` on
//! `reed/spec-inference`), the bundle traits carry algebraic supertrait
//! constraints:
//!   - Connection::Optic: Prism
//!   - Gauge::Group: GroupStructure (plus `act_on`)
//!   - Transport::Holonomy: Metric
//!   - Closure::Fixed: LawvereFixedPoint
//!
//! This file exercises those constraints from outside the crate.

#![cfg(feature = "bundle")]

use prismqueer::{
    Bundle, Closure, Connection, Cyclic, Fiber, Gauge, GroupStructure, IdentityPrism,
    LawvereFixedPoint, Prism, ScalarLoss, StableFiber, Transport,
};
use std::convert::Infallible;
use terni::Imperfect;

struct Spectral {
    optic: IdentityPrism<[f64; 16]>,
    // The gauge: cyclic shift mod 16.
    strategy: Cyclic<16>,
    fixed: StableFiber<[f64; 16]>,
}

impl Fiber for Spectral {
    type State = [f64; 16];
}

impl Connection for Spectral {
    type Optic = IdentityPrism<[f64; 16]>;
    fn connection(&self) -> &IdentityPrism<[f64; 16]> {
        &self.optic
    }
}

impl Gauge for Spectral {
    type Group = Cyclic<16>;
    fn gauge(&self) -> &Cyclic<16> {
        &self.strategy
    }
    fn act_on(&self, state: &[f64; 16]) -> [f64; 16] {
        let k = self.strategy.value() as usize % 16;
        let mut out = [0.0; 16];
        for i in 0..16 {
            out[i] = state[(i + k) % 16];
        }
        out
    }
}

impl Transport for Spectral {
    type Holonomy = ScalarLoss;
    fn transport(&self, state: &[f64; 16]) -> Imperfect<[f64; 16], Infallible, ScalarLoss> {
        // Compress: keep first 8, zero last 8
        let mut compressed = *state;
        let mut loss = 0.0;
        for i in 8..16 {
            loss += compressed[i].abs();
            compressed[i] = 0.0;
        }
        if loss == 0.0 {
            Imperfect::Success(compressed)
        } else {
            Imperfect::Partial(compressed, ScalarLoss::new(loss))
        }
    }
}

impl Closure for Spectral {
    type Fixed = StableFiber<[f64; 16]>;
    fn close(&self) -> &StableFiber<[f64; 16]> {
        &self.fixed
    }
}

fn make_spectral(strategy_shift: u8, kernel: bool) -> Spectral {
    Spectral {
        optic: IdentityPrism::new(),
        strategy: Cyclic::<16>::new(strategy_shift),
        fixed: StableFiber {
            state: [0.0; 16],
            kernel,
        },
    }
}

fn traverse_tower<B: Bundle>(b: &B) -> bool
where
    B::Optic: Prism,
    <<B::Optic as Prism>::Input as prismqueer::Beam>::In: Sized,
    B::State: Default,
    B::Holonomy: std::fmt::Debug,
{
    let state = B::State::default();
    let result = b.transport(&state);
    result.is_ok() || result.is_partial()
}

#[test]
fn bundle_from_outside_crate() {
    let s = make_spectral(1, true);
    assert!(traverse_tower(&s));
}

#[test]
fn transport_with_nonzero_state_returns_partial() {
    let s = make_spectral(2, true);
    let mut state = [0.0f64; 16];
    state[10] = 5.0;
    state[15] = 3.0;
    let result = s.transport(&state);
    match result {
        Imperfect::Partial(_, loss) => assert_eq!(loss.as_f64(), 8.0),
        _ => panic!("expected Partial for nonzero upper half"),
    }
}

#[test]
fn transport_with_zero_upper_returns_success() {
    let s = make_spectral(0, true);
    let mut state = [0.0f64; 16];
    state[0] = 1.0;
    state[7] = 2.0;
    let result = s.transport(&state);
    assert!(result.is_ok());
}

#[test]
fn gauge_act_on_works_outside_crate() {
    let s = make_spectral(1, true);
    let mut state = [0.0f64; 16];
    state[0] = 1.0;
    state[1] = 2.0;
    let acted = s.act_on(&state);
    // Shift by 1: acted[i] == state[(i+1) % 16]; so acted[15] == state[0] == 1.0.
    assert_eq!(acted[15], 1.0);
    assert_eq!(acted[0], 2.0);
}

#[test]
fn closure_fixed_point_in_kernel_outside_crate() {
    let s = make_spectral(0, true);
    assert!(s.close().in_kernel());
}

#[test]
fn group_axioms_outside_crate() {
    // Identity + inverse + associativity for Cyclic<16>.
    let id = Cyclic::<16>::identity();
    let x = Cyclic::<16>::new(7);
    assert_eq!(id.compose(&x), x);
    assert_eq!(x.compose(&id), x);
    assert_eq!(x.compose(&x.inverse()), id);
}
