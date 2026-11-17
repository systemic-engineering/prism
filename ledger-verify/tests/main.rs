//! ledger-verify's tests, one binary.
//!
//! Vectors come from docs/spec (ledger-vectors.json, generated from RFC 9162's
//! text by ledger-vectors.py; ledger-probes.json, transparency-dev/merkle's
//! published probes) and C2SP's own signed-note example. Keys are throwaway
//! keypairs from fixed seeds, made here; the library never sees one.

mod common;

mod anchor;
mod checkpoint;
mod commitment;
mod merkle;
mod note;
mod quorum;
