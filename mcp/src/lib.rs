//! prism's floor as a read-only MCP server (prism-025 §2; docs/spec/mcp.md).
//!
//! - [`json`]: the wire, hand-written from RFC 8259 and RFC 7493 (I-JSON):
//!   a reader whose every refusal is typed and located, and a deterministic
//!   printer. Numbers are exact or refused; never a float.
//!
//! No I/O, no clock, no environment: `#![no_std]` with `alloc`, bytes in,
//! bytes out. Only the binary (`src/bin/prism-mcp.rs`) touches stdin and
//! stdout.
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

pub mod json;
