//! prism-mcp's tests (docs/spec/mcp.md §7). One binary, as echo and chorus.
//!
//! Vectors are read as data from docs/spec: JSONTestSuite (nst/JSONTestSuite
//! 1ef36fa0, `test_parsing/`), chorus's AND checkpoint
//! (mcp-vectors/checkpoint.json), and mirror's echo fixtures
//! (echo-vectors/mirror/fixtures).

mod common;
mod digests;

mod chorus;
mod fuzz;
mod guard;
mod json;
mod protocol;
mod session;
mod suite;
