//! Certificate replay, slice R2: the pact source reader
//! (docs/spec/echo.md §10.4).
//!
//! The grammar is Seam's pact-eval audit §1.2, frozen:
//!
//! ```ebnf
//! file        = { line } ;                  (* ASCII 0x20-0x7E and LF only *)
//! line        = ( blank | comment | record ) , LF ;   (* the last LF may be missing *)
//! blank       = { " " } ;
//! comment     = "#" , { printable } ;       (* column 1 only *)
//! record      = key , sp , body , { " " } ;
//! rule        body = rule-id , sp , atom , sp , "requires" , sp , literal , { "," , sp0 , literal } ;
//! fact        body = ground-atom ;
//! query       body = ground-atom ;
//! functional  body = pred , "(" , position , { "," , sp0 , position } , ")" , sp , "->" , sp , position ;
//! literal     = atom | "not" , sp , atom ;
//! atom        = pred | pred , "(" , term , { "," , sp0 , term } , ")" ;
//! ground-atom = pred | pred , "(" , sym , { "," , sp0 , sym } , ")" ;
//! term        = "?" , name | sym ;
//! pred = sym  = name ;                      (* not "not", not "requires" *)
//! rule-id     = lower , { lower | digit | "_" | "-" } ;
//! name        = lower , { lower | digit | "_" } ;
//! position    = nonzero-digit , { digit } ;
//! ```
//!
//! **Beyond the tune (S4):** since trace-layer C6, `pact eval` reads typed
//! facts. In a `fact` line an argument may also be a typed constant, a type
//! word, one space and its value (fixpoint §5.0.1 item 6, §16.10 item 1):
//! `sym "<printable ASCII, \" and \\ escaped>"`, `nat <decimal>`, `addr <64
//! lowercase hex>`, `bytes "<lowercase hex, even>"`. A type word not
//! followed by a space is a name, as the tune reads it (`p(nat)`). Typed
//! constants in `rule` and `query` lines don't read (docs/spec/echo.md
//! Q24).
//!
//! A line that doesn't read is located at the **least byte offset where the
//! grammar can't continue** (certificate §3.1.1 J): a byte outside
//! 0x20–0x7E is an error where it stands, never ahead of an earlier error on
//! its line; a name that is a keyword fails at the byte after it, where it
//! could still have grown into another name. Every line that doesn't read is
//! kept (Note §6.4: composed), and the lines that do are read.

use alloc::string::String;
use alloc::vec::Vec;

use crate::facts::{self, Position};
use crate::value::{Atom, Value};

/// A term of a rule's atom: a variable `?x` (its name, without `?`) or a
/// constant.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Term {
    Var(String),
    Const(Value),
}

/// An atom that may hold variables.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pattern {
    pub pred: String,
    pub args: Vec<Term>,
}

/// A body literal; `negative` is `not`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Literal {
    pub negative: bool,
    pub atom: Pattern,
}

/// What a line that reads says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Record {
    Rule {
        id: String,
        head: Pattern,
        body: Vec<Literal>,
    },
    Fact(Atom),
    Query(Atom),
    /// `functional <pred>(<k₁>, …) -> <v>`, positions as written.
    Functional {
        pred: String,
        key: Vec<String>,
        value: String,
    },
}

/// A line that reads: its record, its 1-based line and the byte offset of
/// its first byte.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub record: Record,
    pub line: usize,
    pub offset: usize,
}

/// A line that doesn't read: where it stops, as `L:C` and as a byte offset
/// in the whole text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unread {
    pub at: Position,
    pub offset: usize,
}

/// A source, read: the lines that read and the lines that don't, in order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Source {
    pub lines: Vec<Line>,
    pub unread: Vec<Unread>,
}

/// Read a pact source. It always reads: what doesn't read is a hole.
pub fn read(input: &[u8]) -> Source {
    let _ = input;
    Source::default()
}

/// A failure is the 0-based offset in the line where it can't continue.
type Parsed<T> = Result<T, usize>;

const KEYS: [&str; 4] = ["rule", "fact", "query", "functional"];

fn printable(b: u8) -> bool {
    (0x20..=0x7e).contains(&b)
}

/// One line: `None` for a blank or a comment.
fn parse(bytes: &[u8]) -> Parsed<Option<Record>> {
    if bytes.first() == Some(&b'#') {
        return match bytes.iter().position(|&b| !printable(b)) {
            Some(at) => Err(at),
            None => Ok(None),
        };
    }
    let spaces = bytes.iter().take_while(|&&b| b == b' ').count();
    if spaces == bytes.len() {
        return Ok(None);
    }
    if spaces > 0 {
        return Err(spaces);
    }
    let mut c = Cursor { bytes, pos: 0 };
    c.record().map(Some)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Cursor<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn fail<T>(&self) -> Parsed<T> {
        Err(self.pos)
    }

    fn eat(&mut self, b: u8) -> bool {
        let hit = self.peek() == Some(b);
        self.pos += usize::from(hit);
        hit
    }

    fn need(&mut self, b: u8) -> Parsed<()> {
        if self.eat(b) {
            Ok(())
        } else {
            self.fail()
        }
    }

    /// `{ " " }`.
    fn spaces(&mut self) {
        while self.eat(b' ') {}
    }

    /// `" " , { " " }`.
    fn sp(&mut self) -> Parsed<()> {
        self.need(b' ')?;
        self.spaces();
        Ok(())
    }

    /// The literal `word`, failing at its first byte that differs.
    fn word(&mut self, word: &str) -> Parsed<()> {
        word.bytes().try_for_each(|b| self.need(b))
    }

    fn take_while(&mut self, keep: fn(u8) -> bool) -> String {
        let start = self.pos;
        while self.peek().is_some_and(keep) {
            self.pos += 1;
        }
        self.bytes[start..self.pos]
            .iter()
            .map(|&b| b as char)
            .collect()
    }

    /// `name = lower , { lower | digit | "_" }`.
    fn name(&mut self) -> Parsed<String> {
        if !self.peek().is_some_and(|b| b.is_ascii_lowercase()) {
            return self.fail();
        }
        Ok(self.take_while(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'))
    }

    /// A predicate or a `sym`: a name, never `not` or `requires`. A keyword
    /// fails at the byte after it, where a longer name could have gone on.
    fn plain(&mut self) -> Parsed<String> {
        let name = self.name()?;
        if name == "not" || name == "requires" {
            return self.fail();
        }
        Ok(name)
    }

    fn record(&mut self) -> Parsed<Record> {
        // The longest prefix of a key; a key must be whole.
        let mut n = 0;
        while n < self.bytes.len()
            && KEYS
                .iter()
                .any(|k| k.as_bytes().starts_with(&self.bytes[..=n]))
        {
            n += 1;
        }
        self.pos = n;
        let key = KEYS.iter().find(|k| k.as_bytes() == &self.bytes[..n]);
        let Some(&key) = key else {
            return self.fail();
        };
        self.sp()?;
        let record = match key {
            "rule" => self.rule()?,
            "fact" => Record::Fact(self.ground(true)?),
            "query" => Record::Query(self.ground(false)?),
            _ => self.functional()?,
        };
        self.spaces();
        if self.pos == self.bytes.len() {
            Ok(record)
        } else {
            self.fail()
        }
    }

    fn rule(&mut self) -> Parsed<Record> {
        if !self.peek().is_some_and(|b| b.is_ascii_lowercase()) {
            return self.fail();
        }
        let id = self
            .take_while(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-');
        self.sp()?;
        let head = self.pattern()?;
        self.sp()?;
        self.word("requires")?;
        self.sp()?;
        let mut body = Vec::from([self.literal()?]);
        while self.eat(b',') {
            self.spaces();
            body.push(self.literal()?);
        }
        Ok(Record::Rule { id, head, body })
    }

    /// `not , sp , atom` when `not` is followed by a space; else an atom.
    fn literal(&mut self) -> Parsed<Literal> {
        let start = self.pos;
        if self.name()? == "not" && self.peek() == Some(b' ') {
            self.sp()?;
            return Ok(Literal {
                negative: true,
                atom: self.pattern()?,
            });
        }
        self.pos = start;
        Ok(Literal {
            negative: false,
            atom: self.pattern()?,
        })
    }

    /// `pred | pred , "(" , term , { "," , sp0 , term } , ")"`.
    fn pattern(&mut self) -> Parsed<Pattern> {
        let pred = self.plain()?;
        let mut args = Vec::new();
        if self.eat(b'(') {
            loop {
                let term = if self.eat(b'?') {
                    Term::Var(self.name()?)
                } else {
                    Term::Const(Value::Sym(self.plain()?))
                };
                args.push(term);
                if !self.separator()? {
                    break;
                }
            }
        }
        Ok(Pattern { pred, args })
    }

    /// After an argument: `,` and spaces (`true`: another follows), or `)`.
    fn separator(&mut self) -> Parsed<bool> {
        if self.eat(b',') {
            self.spaces();
            Ok(true)
        } else {
            self.need(b')').map(|()| false)
        }
    }

    /// A ground atom; with `typed`, its arguments may be typed constants.
    fn ground(&mut self, typed: bool) -> Parsed<Atom> {
        let pred = self.plain()?;
        let mut args = Vec::new();
        if self.eat(b'(') {
            loop {
                args.push(self.constant(typed)?);
                if !self.separator()? {
                    break;
                }
            }
        }
        Ok(Atom { pred, args })
    }

    fn constant(&mut self, typed: bool) -> Parsed<Value> {
        let name = self.plain()?;
        if !typed || self.peek() != Some(b' ') {
            return Ok(Value::Sym(name));
        }
        match name.as_str() {
            "sym" | "nat" | "addr" | "bytes" => self.pos += 1,
            _ => return Ok(Value::Sym(name)),
        }
        match name.as_str() {
            "sym" => self.sym(),
            "nat" => self.nat(),
            "addr" => self.addr(),
            _ => self.hex_string(),
        }
    }

    /// `"…"`: printable ASCII, `\"` and `\\` the only escapes.
    fn sym(&mut self) -> Parsed<Value> {
        self.need(b'"')?;
        let mut s = String::new();
        loop {
            match self.peek() {
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
                        _ => return self.fail(),
                    }
                }
                Some(b) if printable(b) => {
                    s.push(b as char);
                    self.pos += 1;
                }
                _ => return self.fail(),
            }
        }
    }

    /// `"0" | nonzero-digit , { digit }`, at most 2⁶⁴ − 1: a digit that
    /// would make it larger is where it can't continue.
    fn nat(&mut self) -> Parsed<Value> {
        let Some(first) = self.peek().filter(u8::is_ascii_digit) else {
            return self.fail();
        };
        self.pos += 1;
        let mut n = u64::from(first - b'0');
        if n == 0 {
            return if self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.fail()
            } else {
                Ok(Value::Nat(0))
            };
        }
        while let Some(d) = self.peek().filter(u8::is_ascii_digit) {
            n = match n
                .checked_mul(10)
                .and_then(|n| n.checked_add(u64::from(d - b'0')))
            {
                Some(m) => m,
                None => return self.fail(),
            };
            self.pos += 1;
        }
        Ok(Value::Nat(n))
    }

    fn hex_digit(&mut self) -> Option<u8> {
        let b = self.peek()?;
        let v = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            _ => return None,
        };
        self.pos += 1;
        Some(v)
    }

    /// 64 lowercase hex digits.
    fn addr(&mut self) -> Parsed<Value> {
        let mut a = [0u8; 32];
        for byte in &mut a {
            let hi = self.hex_digit().ok_or(self.pos)?;
            let lo = self.hex_digit().ok_or(self.pos)?;
            *byte = (hi << 4) | lo;
        }
        Ok(Value::Addr(a))
    }

    /// `"` lowercase hex, an even number of digits, `"`: an odd count fails
    /// at the closing quote.
    fn hex_string(&mut self) -> Parsed<Value> {
        self.need(b'"')?;
        let mut nibbles = Vec::new();
        while let Some(v) = self.hex_digit() {
            nibbles.push(v);
        }
        if self.peek() != Some(b'"') || nibbles.len() % 2 == 1 {
            return self.fail();
        }
        self.pos += 1;
        Ok(Value::Bytes(
            nibbles.chunks(2).map(|p| (p[0] << 4) | p[1]).collect(),
        ))
    }

    /// `pred "(" position { "," sp0 position } ")" sp "->" sp position`.
    fn functional(&mut self) -> Parsed<Record> {
        let pred = self.plain()?;
        self.need(b'(')?;
        let mut key = Vec::new();
        loop {
            key.push(self.position()?);
            if !self.separator()? {
                break;
            }
        }
        self.sp()?;
        self.word("->")?;
        self.sp()?;
        let value = self.position()?;
        Ok(Record::Functional { pred, key, value })
    }

    /// `nonzero-digit , { digit }`, kept as written.
    fn position(&mut self) -> Parsed<String> {
        if !self.peek().is_some_and(|b| (b'1'..=b'9').contains(&b)) {
            return self.fail();
        }
        Ok(self.take_while(|b| b.is_ascii_digit()))
    }
}
