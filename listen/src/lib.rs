//! prism's second reader of mirror's reflections (prism-022, S4/S5).
//!
//! Read from mirror's spec text and vectors only, never from mirror's
//! engine source (docs/spec/listen.md §2). The spec is
//! `docs/spec/listen.md`.
//!
//! No I/O, no clock, no sampling, no network: `#![no_std]` with `alloc`.
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;
