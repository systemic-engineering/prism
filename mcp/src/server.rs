//! The server (docs/spec/mcp.md §4): JSON-RPC 2.0 messages, one per line.

use alloc::string::String;
use alloc::vec::Vec;

/// The MCP revision this server speaks (§2).
pub const PROTOCOL: &str = "2025-06-18";

/// One session's state.
#[derive(Debug, Default)]
pub struct Server {}

impl Server {
    pub fn new() -> Server {
        Server {}
    }

    /// Answer one message (without its LF): the answer's line, or nothing.
    pub fn handle(&mut self, message: &[u8]) -> Option<String> {
        let _ = message;
        None
    }
}

/// stdin split at LF (§4).
#[derive(Debug, Default)]
pub struct Lines {}

impl Lines {
    pub fn new() -> Lines {
        Lines {}
    }

    /// Read a chunk; every answer goes to `out`, each line ending in LF.
    pub fn push(&mut self, chunk: &[u8], server: &mut Server, out: &mut Vec<u8>) {
        let _ = (chunk, server, out);
    }

    /// End of input: a last line without LF is read.
    pub fn finish(&mut self, server: &mut Server, out: &mut Vec<u8>) {
        let _ = (server, out);
    }
}

/// A whole session over `input`: the bytes the server writes.
pub fn session(input: &[u8]) -> Vec<u8> {
    let _ = input;
    Vec::new()
}
