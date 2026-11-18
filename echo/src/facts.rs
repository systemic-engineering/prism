//! P1, the facts reader: the reflection's format, `mirror.facts`' subset of
//! pact text.
//!
//! The grammar is assembled from mirror's spec text (docs/spec/echo.md §4
//! gives it whole, with the open questions):
//! - reflection seam A1.4: `# <key> <value>` header lines, `fact` lines with
//!   `addr`/`nat`/`bytes`/`sym` arguments, `#` comments, nothing else; an
//!   unknown line form is refused with its location, never dropped;
//! - Seam's pact-eval audit §1.2 (the frozen pact-text reader): the line,
//!   blank, comment and `fact` record shapes, names, the byte set;
//! - fixpoint §5.0.1 items 6–7 and §16.10 item 1: the typed constants;
//! - fixpoint §16.2.1: positions `L:C`, columns count bytes.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use crate::value::{Atom, Value};

/// A place in the input: 1-based line, 1-based byte column (fixpoint
/// §16.2.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// Why a line doesn't read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unreadable {
    /// A byte outside the line grammar: CR, TAB, NUL, DEL, or a byte that
    /// isn't printable ASCII outside a `sym` string.
    Byte(u8),
    /// A line that is neither blank, a `#` line, nor a `fact` record:
    /// `rule`, `query`, `functional`, an indented record, anything else.
    Form,
    /// Something else stood where this was expected.
    Expected(&'static str),
    /// `not` or `requires`, or a type word (`sym`, `nat`, `addr`, `bytes`)
    /// without its value, where a name belongs.
    Keyword,
    /// `int` or `ratio`: reserved, refused until pinned (§16.10 item 1).
    Reserved,
    /// An uppercase hex digit: hex is lowercase (§5.0.1 item 7).
    Uppercase,
    /// An `addr` that isn't exactly 64 hex digits.
    AddrLength,
    /// A `bytes` string with an odd number of hex digits.
    OddHex,
    /// A `nat` with a leading zero.
    LeadingZero,
    /// A `nat` above 2⁶⁴ − 1 (its encoding is `u64(n)`).
    NatOverflow,
    /// A `\` in a `sym` string not followed by `"` or `\`.
    Escape,
    /// A `sym` string that isn't UTF-8.
    Utf8,
}

impl fmt::Display for Unreadable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unreadable::Byte(b) => write!(f, "byte 0x{b:02x}"),
            Unreadable::Form => f.write_str("unknown line form"),
            Unreadable::Expected(what) => write!(f, "expected {what}"),
            Unreadable::Keyword => f.write_str("a keyword where a name belongs"),
            Unreadable::Reserved => f.write_str("a reserved type"),
            Unreadable::Uppercase => f.write_str("an uppercase hex digit"),
            Unreadable::AddrLength => f.write_str("an addr is 64 hex digits"),
            Unreadable::OddHex => f.write_str("an odd number of hex digits"),
            Unreadable::LeadingZero => f.write_str("a leading zero"),
            Unreadable::NatOverflow => f.write_str("a nat above 2^64 - 1"),
            Unreadable::Escape => f.write_str("an unknown escape"),
            Unreadable::Utf8 => f.write_str("invalid UTF-8"),
        }
    }
}

/// A located refusal: the first place the input doesn't read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub at: Position,
    pub what: Unreadable,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.at, self.what)
    }
}

/// A `# <key> <value>` header line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub key: String,
    pub value: String,
    pub line: usize,
}

/// A `fact` line's atom, with the line it was read from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fact {
    pub atom: Atom,
    pub line: usize,
}

/// What a facts text says: its headers and its facts, in input order.
/// Duplicates are kept: forming the set is the caller's step (fixpoint
/// §5.0.1 item 2).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    pub headers: Vec<Header>,
    pub facts: Vec<Fact>,
}

/// Read a facts text. Positions count from line 1.
pub fn read(input: &[u8]) -> Result<Facts, Refusal> {
    read_from(input, 1)
}

/// Cut `s` (P3) and read its reflection, in the whole file's coordinates.
/// `None` when `s` is unreflected. The part above the seam is never read.
pub fn read_reflection(s: &[u8]) -> Result<Option<Facts>, Refusal> {
    match crate::cut::cut(s).reflection {
        None => Ok(None),
        Some(r) => read_from(r.bytes, r.seam_line + 1).map(Some),
    }
}

pub(crate) fn read_from(input: &[u8], first_line: usize) -> Result<Facts, Refusal> {
    let mut out = Facts::default();
    // Headers are the `# <key> <value>` lines of the leading block of `#`
    // lines; any other `#` line is a comment (docs/spec/echo.md §7 Q3).
    let mut leading = true;
    for (i, bytes) in lines(input).enumerate() {
        let line = first_line + i;
        let mut r = LineReader {
            bytes,
            pos: 0,
            line,
        };
        if bytes.first() == Some(&b'#') {
            r.printable()?;
            if leading {
                if let Some((key, value)) = header(bytes) {
                    out.headers.push(Header { key, value, line });
                }
            }
            continue;
        }
        leading = false;
        if bytes.iter().all(|&b| b == b' ') {
            continue;
        }
        let atom = r.record()?;
        out.facts.push(Fact { atom, line });
    }
    Ok(out)
}

/// The lines of `input`, without their LFs; the last LF may be missing
/// (pact-eval §1.2), and an empty input has no lines.
fn lines(input: &[u8]) -> impl Iterator<Item = &[u8]> {
    let body = input.strip_suffix(b"\n").unwrap_or(input);
    let none = input.is_empty();
    body.split(|&b| b == b'\n').filter(move |_| !none)
}

/// `# <key> <value>`, key `lower { lower | digit | "_" | "-" }`, value
/// non-empty and not starting with a space.
fn header(bytes: &[u8]) -> Option<(String, String)> {
    let rest = bytes.strip_prefix(b"# ")?;
    let space = rest.iter().position(|&b| b == b' ')?;
    let (key, value) = (&rest[..space], &rest[space + 1..]);
    let key_ok = key.first().is_some_and(u8::is_ascii_lowercase)
        && key
            .iter()
            .all(|&b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-');
    let value_ok = value.first().is_some_and(|&b| b != b' ');
    (key_ok && value_ok).then(|| (ascii(key), ascii(value)))
}

/// Bytes already checked to be ASCII, as a string.
fn ascii(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

fn printable(b: u8) -> bool {
    (0x20..=0x7e).contains(&b)
}

fn name_byte(b: u8) -> bool {
    b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'
}

/// The value of a lowercase hex digit (only called on one).
fn nibble(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        _ => b - b'a' + 10,
    }
}

fn unhex(digits: &[u8]) -> Vec<u8> {
    digits
        .chunks(2)
        .map(|p| (nibble(p[0]) << 4) | nibble(p[1]))
        .collect()
}

/// One line, read left to right; `pos` is the 0-based byte offset.
struct LineReader<'a> {
    bytes: &'a [u8],
    pos: usize,
    line: usize,
}

impl<'a> LineReader<'a> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn eat(&mut self, b: u8) -> bool {
        let hit = self.peek() == Some(b);
        self.pos += usize::from(hit);
        hit
    }

    fn spaces(&mut self) {
        while self.eat(b' ') {}
    }

    fn refuse<T>(&self, at: usize, what: Unreadable) -> Result<T, Refusal> {
        Err(Refusal {
            at: Position {
                line: self.line,
                column: at + 1,
            },
            what,
        })
    }

    /// Refuse at the cursor: a byte outside the grammar as itself, anything
    /// else (the end of the line included) as not what was expected.
    fn expected<T>(&self, what: &'static str) -> Result<T, Refusal> {
        match self.peek() {
            Some(b) if !printable(b) => self.refuse(self.pos, Unreadable::Byte(b)),
            _ => self.refuse(self.pos, Unreadable::Expected(what)),
        }
    }

    /// A `#` line: printable ASCII only (pact-eval §1.2).
    fn printable(&self) -> Result<(), Refusal> {
        match self.bytes.iter().position(|&b| !printable(b)) {
            Some(at) => self.refuse(at, Unreadable::Byte(self.bytes[at])),
            None => Ok(()),
        }
    }

    /// A run of name bytes from the cursor (the caller checked the first is
    /// a lowercase letter).
    fn word(&mut self) -> &'a [u8] {
        let start = self.pos;
        while self.peek().is_some_and(name_byte) {
            self.pos += 1;
        }
        &self.bytes[start..self.pos]
    }

    fn run(&mut self, keep: fn(&u8) -> bool) -> &'a [u8] {
        let start = self.pos;
        while self.peek().is_some_and(|b| keep(&b)) {
            self.pos += 1;
        }
        &self.bytes[start..self.pos]
    }

    /// `fact , sp , ground-atom , { " " }`, the whole line.
    fn record(&mut self) -> Result<Atom, Refusal> {
        match self.peek() {
            Some(b) if !printable(b) => return self.refuse(0, Unreadable::Byte(b)),
            Some(b'a'..=b'z') if self.word() == b"fact" => {}
            _ => return self.refuse(0, Unreadable::Form),
        }
        if !self.eat(b' ') {
            return self.expected("a space");
        }
        self.spaces();
        let atom = self.atom()?;
        self.spaces();
        if self.pos != self.bytes.len() {
            return self.expected("the end of the line");
        }
        Ok(atom)
    }

    /// `pred | pred , "(" , arg , { "," , sp0 , arg } , ")"`.
    fn atom(&mut self) -> Result<Atom, Refusal> {
        let start = self.pos;
        if !self.peek().is_some_and(|b| b.is_ascii_lowercase()) {
            return self.expected("a predicate name");
        }
        let pred = self.word();
        if pred == b"not" || pred == b"requires" {
            return self.refuse(start, Unreadable::Keyword);
        }
        let mut args = Vec::new();
        if self.eat(b'(') {
            loop {
                args.push(self.arg()?);
                if self.eat(b',') {
                    self.spaces();
                } else if self.eat(b')') {
                    break;
                } else {
                    return self.expected("`,` or `)`");
                }
            }
        }
        Ok(Atom {
            pred: ascii(pred),
            args,
        })
    }

    /// A bare name (a `sym`), or a type word, one space and its value.
    fn arg(&mut self) -> Result<Value, Refusal> {
        let start = self.pos;
        if !self.peek().is_some_and(|b| b.is_ascii_lowercase()) {
            return self.expected("an argument");
        }
        let word = self.word();
        let typed = self.peek() == Some(b' ');
        match word {
            b"sym" | b"nat" | b"addr" | b"bytes" if typed => {
                self.pos += 1;
                match word {
                    b"sym" => self.sym(),
                    b"nat" => self.nat(),
                    b"addr" => self.addr(),
                    _ => self.bytes(),
                }
            }
            b"int" | b"ratio" => self.refuse(start, Unreadable::Reserved),
            b"sym" | b"nat" | b"addr" | b"bytes" | b"not" | b"requires" => {
                self.refuse(start, Unreadable::Keyword)
            }
            _ => Ok(Value::Sym(ascii(word))),
        }
    }

    /// `"<utf-8>"`, escapes `\"` and `\\` only.
    fn sym(&mut self) -> Result<Value, Refusal> {
        if !self.eat(b'"') {
            return self.expected("`\"`");
        }
        let mut s = String::new();
        loop {
            let at = self.pos;
            match self.peek() {
                None => return self.expected("a closing `\"`"),
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(Value::Sym(s));
                }
                Some(b'\\') => {
                    self.pos += 1;
                    match self.peek() {
                        Some(c @ (b'"' | b'\\')) => {
                            s.push(c as char);
                            self.pos += 1;
                        }
                        _ => return self.refuse(at, Unreadable::Escape),
                    }
                }
                Some(b) if b < 0x20 || b == 0x7f => return self.refuse(at, Unreadable::Byte(b)),
                Some(b) if b < 0x80 => {
                    s.push(b as char);
                    self.pos += 1;
                }
                Some(_) => {
                    let c = self.bytes[at..]
                        .utf8_chunks()
                        .next()
                        .and_then(|chunk| chunk.valid().chars().next());
                    match c {
                        Some(c) => {
                            s.push(c);
                            self.pos += c.len_utf8();
                        }
                        None => return self.refuse(at, Unreadable::Utf8),
                    }
                }
            }
        }
    }

    /// `<decimal>`: no sign, no leading zero, at most 2⁶⁴ − 1.
    fn nat(&mut self) -> Result<Value, Refusal> {
        let start = self.pos;
        let digits = self.run(u8::is_ascii_digit);
        if digits.is_empty() {
            return self.expected("a decimal");
        }
        if digits.len() > 1 && digits[0] == b'0' {
            return self.refuse(start, Unreadable::LeadingZero);
        }
        let mut n: u64 = 0;
        for &d in digits {
            match n
                .checked_mul(10)
                .and_then(|n| n.checked_add(u64::from(d - b'0')))
            {
                Some(m) => n = m,
                None => return self.refuse(start, Unreadable::NatOverflow),
            }
        }
        Ok(Value::Nat(n))
    }

    /// A run of hex digits from the cursor, refusing an uppercase one.
    fn hex_run(&mut self) -> Result<&'a [u8], Refusal> {
        let start = self.pos;
        let digits = self.run(u8::is_ascii_hexdigit);
        match digits.iter().position(u8::is_ascii_uppercase) {
            Some(i) => self.refuse(start + i, Unreadable::Uppercase),
            None => Ok(digits),
        }
    }

    /// `<64 lowercase hex>`.
    fn addr(&mut self) -> Result<Value, Refusal> {
        let start = self.pos;
        let digits = self.hex_run()?;
        let mut a = [0u8; 32];
        if digits.len() != 64 {
            return self.refuse(start, Unreadable::AddrLength);
        }
        a.copy_from_slice(&unhex(digits));
        Ok(Value::Addr(a))
    }

    /// `"<lowercase hex>"`, an even number of digits.
    fn bytes(&mut self) -> Result<Value, Refusal> {
        if !self.eat(b'"') {
            return self.expected("`\"`");
        }
        let digits = self.hex_run()?;
        if !self.eat(b'"') {
            return self.expected("a lowercase hex digit or `\"`");
        }
        if !digits.len().is_multiple_of(2) {
            return self.refuse(self.pos - 1, Unreadable::OddHex);
        }
        Ok(Value::Bytes(unhex(digits)))
    }
}
