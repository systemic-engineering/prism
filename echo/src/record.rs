//! The record's line form (mirror certificate §3.2 N2; Note v3.3.2 §6.7
//! H4), as far as R1 reads it (docs/spec/echo.md §10.1).
//!
//! Each line that isn't blank or a `#` comment is a key, one or more
//! spaces and a value. The keys are the record's: `verdict`, `file`,
//! `shaped_by`, `true`, `undefined`, `false`, `findings`, `finding`,
//! `hole`, `shadow`, `basis`, `dark`, and `rounds` (ignored if present,
//! §3.2). `verdict` stands exactly once and is one of Alex's four words
//! (Note §6.7 H3). A `hole` or `dark` value is split at its first ` #`: the
//! text after it is the instrument's utterance, opaque bytes (Note §6.4
//! (a)). Every other byte is printable ASCII.
//!
//! The values themselves (counts, atoms, findings) are read in R6.

use alloc::vec::Vec;

use crate::facts::{self, Position, Refusal, Unreadable};

const KEYS: [&str; 13] = [
    "verdict",
    "file",
    "shaped_by",
    "true",
    "undefined",
    "false",
    "findings",
    "finding",
    "hole",
    "shadow",
    "basis",
    "dark",
    "rounds",
];

const VERDICTS: [&str; 4] = ["clear", "uncertain", "unknown", "paradox"];

/// One record line: its key, its value's bytes and its line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub key: &'static str,
    pub value: Vec<u8>,
    pub line: usize,
}

/// A record's lines, in input order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub lines: Vec<Line>,
}

fn refuse<T>(line: usize, at: usize, what: Unreadable) -> Result<T, Refusal> {
    Err(Refusal {
        at: Position {
            line,
            column: at + 1,
        },
        what,
    })
}

fn printable_from(bytes: &[u8], from: usize, line: usize) -> Result<(), Refusal> {
    match bytes[from..].iter().position(|&b| !facts::printable(b)) {
        Some(i) => refuse(line, from + i, Unreadable::Byte(bytes[from + i])),
        None => Ok(()),
    }
}

/// Read a record's lines. Positions count from line 1.
pub fn read(input: &[u8]) -> Result<Record, Refusal> {
    let mut lines = Vec::new();
    let mut verdict = false;
    let mut count = 0;
    for (i, bytes) in facts::lines(input).enumerate() {
        let line = i + 1;
        count = line;
        if bytes.first() == Some(&b'#') {
            printable_from(bytes, 0, line)?;
            continue;
        }
        if bytes.iter().all(|&b| b == b' ') {
            continue;
        }
        let len = bytes
            .iter()
            .take_while(|&&b| b.is_ascii_lowercase() || b == b'_')
            .count();
        let Some(&key) = KEYS.iter().find(|k| k.as_bytes() == &bytes[..len]) else {
            return match bytes[0] {
                b if !facts::printable(b) => refuse(line, 0, Unreadable::Byte(b)),
                _ => refuse(line, 0, Unreadable::Form),
            };
        };
        let mut pos = len;
        match bytes.get(pos) {
            Some(b' ') => {}
            Some(&b) if !facts::printable(b) => return refuse(line, pos, Unreadable::Byte(b)),
            _ => return refuse(line, pos, Unreadable::Expected("a space")),
        }
        while bytes.get(pos) == Some(&b' ') {
            pos += 1;
        }
        let value = &bytes[pos..];
        if value.is_empty() {
            return refuse(line, pos, Unreadable::Expected("a value"));
        }
        // The utterance after a `hole`'s or `dark`'s first ` #` is opaque.
        let checked = match key {
            "hole" | "dark" => value
                .windows(2)
                .position(|w| w == b" #")
                .map_or(bytes.len(), |i| pos + i),
            _ => bytes.len(),
        };
        printable_from(&bytes[..checked], pos, line)?;
        if key == "verdict" {
            if verdict {
                return refuse(line, 0, Unreadable::Repeated);
            }
            verdict = true;
            if !VERDICTS.iter().any(|v| v.as_bytes() == value) {
                return refuse(
                    line,
                    pos,
                    Unreadable::Expected("clear, uncertain, unknown or paradox"),
                );
            }
        }
        lines.push(Line {
            key,
            value: value.to_vec(),
            line,
        });
    }
    if verdict {
        Ok(Record { lines })
    } else {
        refuse(count + 1, 0, Unreadable::Expected("a `verdict` line"))
    }
}
