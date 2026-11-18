//! prism's second reader of mirror's reflections (prism-022, S4/S5).
//!
//! Echo can only give back the last words she was given, and she never
//! speaks first. The checker never authors: it replays the engine's own
//! record from the text and gives it back exactly, so where the two differ,
//! the difference is named and located.
//!
//! Read from mirror's spec text and vectors only, never from mirror's
//! engine source (docs/spec/echo.md §2). The spec is
//! `docs/spec/echo.md`.
//!
//! No I/O, no clock, no sampling, no network: `#![no_std]` with `alloc`.
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

pub mod address;
pub mod certificate;
pub mod cut;
pub mod facts;
pub mod integrity;
pub mod record;
pub mod replay;
pub mod value;

pub use address::{Address, Tau};
pub use cut::{cut, Cut, Reflection};
pub use facts::{read, read_reflection, Fact, Facts, Header, Position, Refusal, Unreadable};
pub use value::{Atom, Value};
