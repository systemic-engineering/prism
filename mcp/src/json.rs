//! The wire (docs/spec/mcp.md §3): RFC 8259, narrowed by I-JSON (RFC 7493)
//! and by an exact-number rule.
//!
//! - [`read`]: one message in, a [`Value`] or a [`Refusal`] out. Every
//!   refusal is typed and located at a byte offset. Recursive descent, its
//!   depth bounded by [`MAX_DEPTH`].
//! - [`print`]: one line, no whitespace, members in byte order of their
//!   names, so identical values give identical bytes.
//!
//! Numbers are exact or refused (§3.2): an integer is an `i64`, and a
//! number with a fraction or an exponent is its lexeme, never converted.
//! There is no floating point in this crate.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;

/// The largest message read, in bytes (§3.3).
pub const MAX_BYTES: usize = 8 * 1024 * 1024;

/// The deepest nesting of arrays and objects read (§3.3).
pub const MAX_DEPTH: usize = 64;

/// A JSON value. Objects keep their members in byte order of their names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Null,
    Bool(bool),
    /// An integer: no fraction, no exponent, within `i64`.
    Int(i64),
    /// A number with a fraction or an exponent: its exact lexeme.
    Decimal(Decimal),
    Str(String),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

/// A number with a fraction or an exponent, kept as read (§3.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decimal {
    /// The lexeme, byte for byte.
    pub text: String,
    /// Its first byte's offset in the message.
    pub at: usize,
}

/// Why reading stopped (§3.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The message is over [`MAX_BYTES`].
    Size,
    /// An array or object past [`MAX_DEPTH`].
    Depth,
    /// The message ended inside a value.
    End,
    /// A byte that can't start or continue the value here.
    Unexpected,
    /// Something other than whitespace after the value.
    Trailing,
    /// An unescaped byte below 0x20 in a string.
    Control,
    /// An escape that isn't one of RFC 8259 §7's.
    Escape,
    /// A `\u` surrogate escape without its partner.
    Surrogate,
    /// Bytes in a string that aren't UTF-8.
    Utf8,
    /// A noncharacter (RFC 7493 §2.1).
    Noncharacter,
    /// A member name the object already has (RFC 7493 §2.3).
    Duplicate,
    /// A digit after a leading `0`.
    LeadingZero,
    /// `-0`, the second spelling of zero.
    NegativeZero,
    /// An integer outside `i64`.
    IntegerRange,
    /// A fraction or an exponent where an integer is expected.
    NotInteger,
}

impl Kind {
    /// The refusal's word, as `data.refusal` carries it.
    pub fn word(self) -> &'static str {
        match self {
            Kind::Size => "size",
            Kind::Depth => "depth",
            Kind::End => "end",
            Kind::Unexpected => "unexpected",
            Kind::Trailing => "trailing",
            Kind::Control => "control",
            Kind::Escape => "escape",
            Kind::Surrogate => "surrogate",
            Kind::Utf8 => "utf8",
            Kind::Noncharacter => "noncharacter",
            Kind::Duplicate => "duplicate",
            Kind::LeadingZero => "leading-zero",
            Kind::NegativeZero => "negative-zero",
            Kind::IntegerRange => "integer-range",
            Kind::NotInteger => "not-integer",
        }
    }
}

/// A refusal: its kind, and the byte offset where reading stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub kind: Kind,
    pub at: usize,
}

fn refuse<T>(kind: Kind, at: usize) -> Result<T, Refusal> {
    Err(Refusal { kind, at })
}

/// Read one message.
pub fn read(bytes: &[u8]) -> Result<Value, Refusal> {
    if bytes.len() > MAX_BYTES {
        return refuse(Kind::Size, MAX_BYTES);
    }
    let mut r = Reader { bytes, at: 0 };
    r.space();
    let v = r.value(0)?;
    r.space();
    if r.at < bytes.len() {
        return refuse(Kind::Trailing, r.at);
    }
    Ok(v)
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

/// RFC 7493 §2.1, by Unicode's definition: U+FDD0..U+FDEF, and the last two
/// code points of every plane.
fn noncharacter(c: u32) -> bool {
    (0xFDD0..=0xFDEF).contains(&c) || c & 0xFFFE == 0xFFFE
}

impl Reader<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    /// RFC 8259 §2: space, tab, LF, CR.
    fn space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    /// The next byte, which must be there.
    fn next(&mut self) -> Result<u8, Refusal> {
        match self.peek() {
            Some(b) => {
                self.at += 1;
                Ok(b)
            }
            None => refuse(Kind::End, self.at),
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, Refusal> {
        match self.peek() {
            None => refuse(Kind::End, self.at),
            Some(b'{') => self.object(depth + 1),
            Some(b'[') => self.array(depth + 1),
            Some(b'"') => self.string().map(Value::Str),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(b't') => self.literal(b"true", Value::Bool(true)),
            Some(b'f') => self.literal(b"false", Value::Bool(false)),
            Some(b'n') => self.literal(b"null", Value::Null),
            Some(_) => refuse(Kind::Unexpected, self.at),
        }
    }

    fn literal(&mut self, word: &[u8], v: Value) -> Result<Value, Refusal> {
        for &w in word {
            if self.next()? != w {
                return refuse(Kind::Unexpected, self.at - 1);
            }
        }
        Ok(v)
    }

    fn open(&mut self, depth: usize) -> Result<(), Refusal> {
        if depth > MAX_DEPTH {
            return refuse(Kind::Depth, self.at);
        }
        self.at += 1;
        self.space();
        Ok(())
    }

    /// After a member or element: `,` (true, more) or the closing byte.
    fn more(&mut self, close: u8) -> Result<bool, Refusal> {
        self.space();
        match self.next()? {
            b',' => {
                self.space();
                Ok(true)
            }
            b if b == close => Ok(false),
            _ => refuse(Kind::Unexpected, self.at - 1),
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value, Refusal> {
        self.open(depth)?;
        let mut items = Vec::new();
        if self.peek() == Some(b']') {
            self.at += 1;
            return Ok(Value::Array(items));
        }
        loop {
            items.push(self.value(depth)?);
            if !self.more(b']')? {
                return Ok(Value::Array(items));
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, Refusal> {
        self.open(depth)?;
        let mut members = BTreeMap::new();
        if self.peek() == Some(b'}') {
            self.at += 1;
            return Ok(Value::Object(members));
        }
        loop {
            let quote = self.at;
            match self.peek() {
                Some(b'"') => {}
                None => return refuse(Kind::End, self.at),
                Some(_) => return refuse(Kind::Unexpected, self.at),
            }
            let name = self.string()?;
            if members.contains_key(&name) {
                return refuse(Kind::Duplicate, quote);
            }
            self.space();
            if self.next()? != b':' {
                return refuse(Kind::Unexpected, self.at - 1);
            }
            self.space();
            let v = self.value(depth)?;
            members.insert(name, v);
            if !self.more(b'}')? {
                return Ok(Value::Object(members));
            }
        }
    }

    /// Four hex digits after `\u`; the escape starts at `escape`.
    fn hex4(&mut self, escape: usize) -> Result<u32, Refusal> {
        let mut n = 0;
        for _ in 0..4 {
            let d = match self.peek() {
                Some(b @ b'0'..=b'9') => b - b'0',
                Some(b @ b'a'..=b'f') => b - b'a' + 10,
                Some(b @ b'A'..=b'F') => b - b'A' + 10,
                _ => return refuse(Kind::Escape, escape),
            };
            self.at += 1;
            n = n * 16 + u32::from(d);
        }
        Ok(n)
    }

    /// A raw run of string bytes, `bytes[start..self.at]`: strict UTF-8, no
    /// noncharacter.
    fn run(&self, start: usize, out: &mut String) -> Result<(), Refusal> {
        let text = match core::str::from_utf8(&self.bytes[start..self.at]) {
            Ok(t) => t,
            Err(e) => return refuse(Kind::Utf8, start + e.valid_up_to()),
        };
        if let Some((i, _)) = text.char_indices().find(|&(_, c)| noncharacter(c as u32)) {
            return refuse(Kind::Noncharacter, start + i);
        }
        out.push_str(text);
        Ok(())
    }

    /// RFC 8259 §7, at the opening quote. The earliest offence wins: a raw
    /// run is checked before whatever ended it.
    fn string(&mut self) -> Result<String, Refusal> {
        self.at += 1;
        let mut out = String::new();
        loop {
            let start = self.at;
            while let Some(b) = self.peek() {
                if b == b'"' || b == b'\\' || b < 0x20 {
                    break;
                }
                self.at += 1;
            }
            self.run(start, &mut out)?;
            let escape = self.at;
            match self.next()? {
                b'"' => return Ok(out),
                b'\\' => {}
                _ => return refuse(Kind::Control, escape),
            }
            let c = match self.next()? {
                b'"' => '"',
                b'\\' => '\\',
                b'/' => '/',
                b'b' => '\u{8}',
                b'f' => '\u{c}',
                b'n' => '\n',
                b'r' => '\r',
                b't' => '\t',
                b'u' => self.unicode(escape)?,
                _ => return refuse(Kind::Escape, escape),
            };
            out.push(c);
        }
    }

    /// `\uXXXX`, or a surrogate pair `\uD8xx\uDCxx`; the first backslash is
    /// at `escape`.
    fn unicode(&mut self, escape: usize) -> Result<char, Refusal> {
        let high = self.hex4(escape)?;
        let c = match high {
            0xD800..=0xDBFF => {
                if self.bytes.get(self.at..self.at + 2) != Some(b"\\u") {
                    return refuse(Kind::Surrogate, escape);
                }
                self.at += 2;
                let low = self.hex4(self.at - 2)?;
                if !(0xDC00..=0xDFFF).contains(&low) {
                    return refuse(Kind::Surrogate, escape);
                }
                0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00)
            }
            0xDC00..=0xDFFF => return refuse(Kind::Surrogate, escape),
            c => c,
        };
        if noncharacter(c) {
            return refuse(Kind::Noncharacter, escape);
        }
        // Every value left is a scalar value: surrogates are gone, and a
        // pair gives at most U+10FFFF.
        char::from_u32(c).map_or(refuse(Kind::Surrogate, escape), Ok)
    }

    fn digits(&mut self) -> Result<(), Refusal> {
        match self.peek() {
            Some(b'0'..=b'9') => {}
            None => return refuse(Kind::End, self.at),
            Some(_) => return refuse(Kind::Unexpected, self.at),
        }
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.at += 1;
        }
        Ok(())
    }

    /// RFC 8259 §6, then §3.2's rule: an integer is an `i64`, anything with
    /// a fraction or an exponent is its lexeme.
    fn number(&mut self) -> Result<Value, Refusal> {
        let start = self.at;
        let negative = self.peek() == Some(b'-');
        if negative {
            self.at += 1;
        }
        let lead = self.at;
        self.digits()?;
        if self.bytes[lead] == b'0' && self.at > lead + 1 {
            return refuse(Kind::LeadingZero, lead + 1);
        }
        let mut integer = true;
        if self.peek() == Some(b'.') {
            self.at += 1;
            self.digits()?;
            integer = false;
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.at += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.at += 1;
            }
            self.digits()?;
            integer = false;
        }
        // ASCII by construction.
        let text = core::str::from_utf8(&self.bytes[start..self.at]).unwrap_or_default();
        if !integer {
            return Ok(Value::Decimal(Decimal {
                text: text.into(),
                at: start,
            }));
        }
        if text == "-0" {
            return refuse(Kind::NegativeZero, start);
        }
        text.parse::<i64>()
            .map(Value::Int)
            .or(refuse(Kind::IntegerRange, start))
    }
}

/// Print a value on one line, members in byte order of their names.
pub fn print(value: &Value) -> String {
    let mut out = String::new();
    write(&mut out, value);
    out
}

fn write(out: &mut String, value: &Value) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Int(n) => {
            let _ = write!(out, "{n}");
        }
        Value::Decimal(d) => out.push_str(&d.text),
        Value::Str(s) => string(out, s),
        Value::Array(items) => {
            out.push('[');
            for (i, v) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write(out, v);
            }
            out.push(']');
        }
        Value::Object(members) => {
            out.push('{');
            for (i, (k, v)) in members.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                string(out, k);
                out.push(':');
                write(out, v);
            }
            out.push('}');
        }
    }
}

/// §3.5: `"` and `\` escaped, the five short escapes, `\u00xx` for every
/// other control byte, everything else raw.
fn string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

impl Value {
    /// The value as an integer where one is expected: `None` if it isn't a
    /// number, `not-integer` if it has a fraction or an exponent.
    pub fn integer(&self) -> Option<Result<i64, Refusal>> {
        match self {
            Value::Int(n) => Some(Ok(*n)),
            Value::Decimal(d) => Some(refuse(Kind::NotInteger, d.at)),
            _ => None,
        }
    }
}
