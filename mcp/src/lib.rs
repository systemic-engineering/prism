//! prism's floor as a read-only MCP server (prism-025 §2; docs/spec/mcp.md).
//!
//! - The wire, [`json`], hand-written from RFC 8259 and RFC 7493 (I-JSON):
//!   a reader whose every refusal is typed and located, and a deterministic
//!   printer. Numbers are exact or refused; never a float.
//! - The [`server`]: JSON-RPC 2.0 under MCP 2025-06-18's lifecycle, one
//!   message per line.
//! - The [`tools`]: `echo` (the echo library, byte for byte the binary's
//!   lines) and `chorus` (the AND checkpoint and an inclusion proof); and,
//!   on prism's floor (prism-laws), `check` (agreement) and `measure` (a
//!   precision matrix over ℚ).
//!
//! No I/O, no clock, no environment: `#![no_std]` with `alloc`, bytes in,
//! bytes out. Only the binary (`src/bin/prism-mcp.rs`) touches stdin and
//! stdout. (prism-laws, which `check` and `measure` call, is a std crate, so
//! the build links std; this crate's own code still names none of it.)
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod exact;
pub mod json;
pub mod server;
pub mod tools;
