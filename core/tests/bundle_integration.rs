//! Integration test: Bundle trait used from outside the crate.

#![cfg(feature = "bundle")]

use prism_core::{Bundle, Closure, Connection, Fiber, Gauge, Transport};
use prism_core::ScalarLoss;
use std::convert::Infallible;
use terni::Imperfect;

struct Spectral {
    optic: &'static str,
    strategy: u32,
}

impl Fiber for Spectral {
    type State = [f64; 16];
}

impl Connection for Spectral {
    type Optic = &'static str;
    fn connection(&self) -> &&'static str {
        &self.optic
    }
}

impl Gauge for Spectral {
    type Group = u32;
    fn gauge(&self) -> &u32 {
        &self.strategy
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
    type Fixed = &'static str;
    fn close(&self) -> &&'static str {
        &"fate"
    }
}

fn traverse_tower<B: Bundle>(b: &B) -> bool
where
    B::State: Default,
    B::Holonomy: std::fmt::Debug,
{
    let state = B::State::default();
    let result = b.transport(&state);
    result.is_ok() || result.is_partial()
}

#[test]
fn bundle_from_outside_crate() {
    let s = Spectral {
        optic: "traversal",
        strategy: 1,
    };
    assert!(traverse_tower(&s));
}

#[test]
fn transport_with_nonzero_state_returns_partial() {
    let s = Spectral {
        optic: "fold",
        strategy: 2,
    };
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
    let s = Spectral {
        optic: "iso",
        strategy: 0,
    };
    let mut state = [0.0f64; 16];
    state[0] = 1.0;
    state[7] = 2.0;
    let result = s.transport(&state);
    assert!(result.is_ok());
}
