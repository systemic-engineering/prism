//! Beam — the semifunctor. The pipeline value carrier.
//!
//! `tick` is the primitive: one step forward.
//! `next` is the lossless shorthand.
//! `smap` is the semifunctor map, derived from `tick`.

use imperfect::{Imperfect, Loss, ShannonLoss};
use std::convert::Infallible;

// Beam trait and PureBeam will go here

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_beam_ok() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 42);
        assert!(b.is_ok());
        assert!(!b.is_err());
        assert_eq!(b.result().ok(), Some(&42));
        assert_eq!(b.input(), &());
    }

    #[test]
    fn pure_beam_partial() {
        let b: PureBeam<(), u32> = PureBeam::partial((), 42, ShannonLoss::new(1.5));
        assert!(b.is_ok());
        assert!(b.is_partial());
        assert_eq!(b.result().ok(), Some(&42));
    }

    #[test]
    fn pure_beam_err() {
        let b: PureBeam<(), u32, String> = PureBeam::err((), "oops".into());
        assert!(b.is_err());
        assert!(!b.is_ok());
    }

    #[test]
    fn next_ok_to_ok() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 10);
        let n = b.next("hello");
        assert!(n.is_ok());
        assert!(!n.is_partial());
        assert_eq!(n.result().ok(), Some(&"hello"));
        assert_eq!(n.input(), &10u32);
    }

    #[test]
    fn next_partial_carries_loss() {
        let b: PureBeam<(), u32> = PureBeam::partial((), 10, ShannonLoss::new(2.0));
        let n = b.next(20u32);
        assert!(n.is_partial());
        assert_eq!(n.input(), &10u32);
    }

    #[test]
    #[should_panic(expected = "tick on Err beam")]
    fn next_on_err_panics() {
        let b: PureBeam<(), u32, String> = PureBeam::err((), "err".into());
        let _ = b.next(0u32);
    }

    #[test]
    fn tick_ok_with_ok() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 5);
        let n = b.tick(Imperfect::<&str, String>::Ok("hi"));
        assert!(n.is_ok());
        assert!(!n.is_partial());
    }

    #[test]
    fn tick_ok_with_partial() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 5);
        let n = b.tick(Imperfect::<&str, String>::Partial("hi", ShannonLoss::new(1.0)));
        assert!(n.is_partial());
        assert_eq!(n.result().ok(), Some(&"hi"));
    }

    #[test]
    fn tick_ok_with_err() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 5);
        let n: PureBeam<u32, u32, i32> = b.tick(Imperfect::Err(-1));
        assert!(n.is_err());
    }

    #[test]
    fn tick_partial_with_ok_carries_loss() {
        let b: PureBeam<(), u32> = PureBeam::partial((), 5, ShannonLoss::new(1.0));
        let n = b.tick(Imperfect::<u32, String>::Ok(10));
        assert!(n.is_partial());
    }

    #[test]
    fn tick_partial_with_partial_accumulates() {
        let b: PureBeam<(), u32> = PureBeam::partial((), 5, ShannonLoss::new(1.0));
        let n = b.tick(Imperfect::<u32, String>::Partial(10, ShannonLoss::new(0.5)));
        assert!(n.is_partial());
    }

    #[test]
    fn tick_partial_with_err() {
        let b: PureBeam<(), u32> = PureBeam::partial((), 5, ShannonLoss::new(1.0));
        let n = b.tick(Imperfect::<u32, String>::Err("fail".into()));
        assert!(n.is_err());
    }

    #[test]
    #[should_panic(expected = "tick on Err beam")]
    fn tick_on_err_panics() {
        let b: PureBeam<(), u32, String> = PureBeam::err((), "err".into());
        let _ = b.tick(Imperfect::<u32, String>::Ok(0));
    }

    #[test]
    fn type_chain_three_steps() {
        let b0: PureBeam<(), u32> = PureBeam::ok((), 42u32);
        let b1: PureBeam<u32, String> = b0.next("hello".to_string());
        let b2: PureBeam<String, Vec<char>> = b1.next(vec!['a', 'b']);
        assert_eq!(b2.input(), &"hello".to_string());
        assert_eq!(b2.result().ok(), Some(&vec!['a', 'b']));
    }

    // --- smap ---

    #[test]
    fn smap_ok() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 5);
        let n = b.smap(|&v| Imperfect::Ok(v * 2));
        assert_eq!(n.result().ok(), Some(&10));
        assert!(!n.is_partial());
    }

    #[test]
    fn smap_returns_partial() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 5);
        let n = b.smap(|&v| Imperfect::Partial(v * 2, ShannonLoss::new(0.5)));
        assert!(n.is_partial());
        assert_eq!(n.result().ok(), Some(&10));
    }

    #[test]
    fn smap_returns_err() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 5);
        let n = b.smap(|_| Imperfect::<u32, String>::Err("nope".into()));
        assert!(n.is_err());
    }

    #[test]
    fn smap_on_partial_accumulates_loss() {
        let b: PureBeam<(), u32> = PureBeam::partial((), 5, ShannonLoss::new(1.0));
        let n = b.smap(|&v| Imperfect::Partial(v * 2, ShannonLoss::new(0.5)));
        assert!(n.is_partial());
    }

    #[test]
    #[should_panic(expected = "smap on Err beam")]
    fn smap_on_err_panics() {
        let b: PureBeam<(), u32, String> = PureBeam::err((), "err".into());
        let _ = b.smap(|&v| Imperfect::<u32, String>::Ok(v));
    }
}
