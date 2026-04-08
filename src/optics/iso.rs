//! Iso — the total invertible optic.
//!
//! An Iso<A, B> is a pair of functions (forward: A → B, backward: B → A)
//! such that `backward(forward(a)) = a` and `forward(backward(b)) = b`.
//! This is the only optic where refract is genuinely lossless and the
//! round-trip holds as a law.

use crate::{Beam, Prism, Stage};
use std::marker::PhantomData;

// Types go here.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_round_trip() {
        // An iso between String and Vec<char>.
        let iso: Iso<String, Vec<char>> = Iso::new(
            |s: String| s.chars().collect::<Vec<char>>(),
            |v: Vec<char>| v.into_iter().collect::<String>(),
        );

        let input = "hello".to_string();
        // Apply forward then backward — should get the original back.
        let forward = iso.forward("hello".to_string());
        assert_eq!(forward, vec!['h', 'e', 'l', 'l', 'o']);
        let backward = iso.backward(forward);
        assert_eq!(backward, "hello");
    }

    #[test]
    fn iso_refract_is_lossless() {
        let iso: Iso<String, Vec<char>> = Iso::new(
            |s: String| s.chars().collect::<Vec<char>>(),
            |v: Vec<char>| v.into_iter().collect::<String>(),
        );

        let beam = Beam::new("test".to_string());
        let projected = iso.project(iso.focus(beam));
        assert_eq!(projected.result, vec!['t', 'e', 's', 't']);
        assert!(projected.loss.is_zero());

        let refracted = iso.refract(projected);
        assert_eq!(refracted.stage, Stage::Refracted);
    }
}
