//! `prism-mcp`: the server over stdio (docs/spec/mcp.md §2, §4).
//!
//! It reads stdin and writes stdout, and nothing else: no file, no
//! environment, no clock, no network, no process, and nothing on stderr.
//! Each chunk read is split at LF and answered at once; at end of input a
//! last line without LF is read, and the server exits 0. A failed read or
//! write exits 1.

use std::io::{ErrorKind, Read, Write};
use std::process::ExitCode;

use prism_mcp::server::{Lines, Server};

fn flush(output: &mut impl Write, out: &mut Vec<u8>) -> std::io::Result<()> {
    if !out.is_empty() {
        output.write_all(out)?;
        output.flush()?;
        out.clear();
    }
    Ok(())
}

fn main() -> ExitCode {
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    let mut server = Server::new();
    let mut lines = Lines::new();
    let mut chunk = vec![0u8; 1 << 16];
    let mut out = Vec::new();
    loop {
        let n = match input.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(_) => return ExitCode::from(1),
        };
        lines.push(&chunk[..n], &mut server, &mut out);
        if flush(&mut output, &mut out).is_err() {
            return ExitCode::from(1);
        }
    }
    lines.finish(&mut server, &mut out);
    match flush(&mut output, &mut out) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::from(1),
    }
}
