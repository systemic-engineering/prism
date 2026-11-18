//! The performance: the certificate's printed form (mirror certificate
//! §3.2), read as text (docs/spec/echo.md §10.1).
//!
//! Mirror-text, one record per line: a key, one or more spaces, a value.
//! `#` comments at column 1 and blank lines are allowed anywhere. The keys
//! are §3.2's: the header (`certificate 1`, `result`, `program`, `facts`,
//! `engine`, `dark-order`, each once), `layer`, `derive`, `context`,
//! `dark`, `violating`; Note §6.5 R5's `erasure … budget count <n>
//! <unit>`; and Note §6.7 H5's `hole <text>`. Atoms are read by the facts
//! grammar (docs/spec/echo.md §4), so typed constants read (S4).
//!
//! A line that doesn't read is refused at the least byte offset where it
//! can't continue (certificate §3.1.1 J): a separator cut short is refused
//! where it stops.
//!
//! Reading says nothing about truth: every value here is a claim, compared
//! later with echo's own recomputation from the source (§2.2).

use alloc::string::String;
use alloc::vec::Vec;

use crate::address::{self, Address, Tau};
use crate::facts::{self, Position, Refusal, Unreadable};
use crate::value::Atom;

/// Which run a line belongs to: the positive run (`tick`, printed as no
/// word on `layer` and `derive` lines), or W's `under` and `over`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Run {
    Tick,
    Under,
    Over,
}

/// The six header lines' claims. `None` is a printed `none`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Header {
    pub result: Option<Address>,
    pub program: Option<Address>,
    pub facts: Option<Address>,
    pub engine: Option<Address>,
    pub dark_order: Option<Address>,
}

/// `layer [under|over] <t> <atom>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layer {
    pub run: Run,
    pub tick: u64,
    pub atom: Atom,
    pub line: usize,
}

/// `derive [under|over] <head> by <rule-id> [from <p₁> & …]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Derive {
    pub run: Run,
    pub head: Atom,
    pub rule: String,
    pub premises: Vec<Atom>,
    pub line: usize,
}

/// `context under|over <k> = [<atom> & …]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    pub run: Run,
    pub index: u64,
    pub atoms: Vec<Atom>,
    pub line: usize,
}

/// `functional <pred>[(<k₁>, …)] -> <v>`, a canonical declaration line
/// (certificate §3.1.1 B, G).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Functional {
    pub pred: String,
    pub key: Vec<u64>,
    pub value: u64,
}

/// An engine Dark's check (certificate §3.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Engine {
    Arity(String),
    Safety { rule: String, var: String },
    Declaration(String),
}

/// A `dark` line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dark {
    /// `dark 2 at <line>:<col>`
    At { line: u64, column: u64 },
    /// `dark 2 engine …`
    Engine(Engine),
    /// `dark 255 <canonical functional line>`
    Declined(Functional),
}

/// `violating <atom> [by <decl>]` (§3.2 N1: the short form is a reader's
/// tolerance).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violating {
    pub atom: Atom,
    pub by: Option<Functional>,
    pub line: usize,
}

/// `erasure <run> budget count <n> <unit>` (Note §6.5 R5, R6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Erasure {
    pub run: Run,
    pub count: u64,
    pub rounds: bool,
    pub line: usize,
}

/// Everything a certificate prints, line by line, in input order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Performance {
    pub header: Header,
    pub layers: Vec<Layer>,
    pub derives: Vec<Derive>,
    pub contexts: Vec<Context>,
    pub dark: Vec<(Dark, usize)>,
    pub violating: Vec<Violating>,
    pub erasures: Vec<Erasure>,
    /// `hole <text>` (Note §6.7 H5), the text as printed.
    pub holes: Vec<(String, usize)>,
}

impl Performance {
    /// Whether the performance prints a model: a `result`, a layer, a
    /// derivation or a context. A Dark source's performance prints none
    /// (check 2).
    pub fn carries_model(&self) -> bool {
        self.header.result.is_some()
            || !self.layers.is_empty()
            || !self.derives.is_empty()
            || !self.contexts.is_empty()
    }

    /// The model's address, recomputed from the layers: the positive run's
    /// atoms, or W's under-run's, as `fragment(@pact/fact root, "model",
    /// sorted)` (certificate §3.1 contexts and `result`; §3.1.1 A). `None`
    /// when an atom is listed twice: that makes no set (fixpoint §5.0.1
    /// item 2).
    pub fn model(&self) -> Option<Address> {
        let tau = Tau::pact_fact();
        let members: Vec<Address> = self
            .layers
            .iter()
            .filter(|l| l.run != Run::Over)
            .map(|l| address::atom(&tau, &l.atom))
            .collect();
        address::set(&tau, "model", &members).ok()
    }
}

const HEADER: [&str; 6] = [
    "certificate",
    "result",
    "program",
    "facts",
    "engine",
    "dark-order",
];

const MISSING: [&str; 6] = [
    "a `certificate` line",
    "a `result` line",
    "a `program` line",
    "a `facts` line",
    "an `engine` line",
    "a `dark-order` line",
];

/// Read a certificate. Positions count from line 1.
pub fn read(input: &[u8]) -> Result<Performance, Refusal> {
    let mut out = Performance::default();
    let mut seen = [false; 6];
    let mut count = 0;
    for (i, bytes) in facts::lines(input).enumerate() {
        let line = i + 1;
        count = line;
        let mut c = Cursor {
            bytes,
            pos: 0,
            line,
        };
        if bytes.first() == Some(&b'#') {
            c.comment()?;
            continue;
        }
        if bytes.iter().all(|&b| b == b' ') {
            continue;
        }
        let key = c.key()?;
        if let Some(h) = HEADER.iter().position(|&k| k == key) {
            if seen[h] {
                return c.refuse(0, Unreadable::Repeated);
            }
            seen[h] = true;
            c.header(h, &mut out.header)?;
        } else {
            c.record(key, &mut out)?;
        }
        c.end()?;
    }
    match seen.iter().position(|s| !s) {
        Some(h) => Err(Refusal {
            at: Position {
                line: count + 1,
                column: 1,
            },
            what: Unreadable::Expected(MISSING[h]),
        }),
        None => Ok(out),
    }
}

const KEYS: [&str; 7] = [
    "layer",
    "derive",
    "context",
    "dark",
    "violating",
    "erasure",
    "hole",
];

/// One line, read left to right; `pos` is the 0-based byte offset.
struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
    line: usize,
}

impl<'a> Cursor<'a> {
    fn refuse<T>(&self, at: usize, what: Unreadable) -> Result<T, Refusal> {
        Err(Refusal {
            at: Position {
                line: self.line,
                column: at + 1,
            },
            what,
        })
    }

    /// Refuse at `at`: a byte outside the grammar as itself, anything else
    /// (the end of the line included) as not what was expected.
    fn expected<T>(&self, at: usize, what: &'static str) -> Result<T, Refusal> {
        match self.bytes.get(at) {
            Some(&b) if !facts::printable(b) => self.refuse(at, Unreadable::Byte(b)),
            _ => self.refuse(at, Unreadable::Expected(what)),
        }
    }

    fn rest(&self) -> &'a [u8] {
        &self.bytes[self.pos..]
    }

    fn at_end(&self) -> bool {
        self.pos == self.bytes.len()
    }

    /// The literal `lit`, refused at the first byte that differs.
    fn lit(&mut self, lit: &'static str) -> Result<(), Refusal> {
        for &want in lit.as_bytes() {
            if self.bytes.get(self.pos) != Some(&want) {
                return self.expected(self.pos, lit);
            }
            self.pos += 1;
        }
        Ok(())
    }

    /// Take `word` if the rest starts with it.
    fn take(&mut self, word: &str) -> bool {
        let hit = self.rest().starts_with(word.as_bytes());
        if hit {
            self.pos += word.len();
        }
        hit
    }

    fn end(&self) -> Result<(), Refusal> {
        if self.at_end() {
            Ok(())
        } else {
            self.expected(self.pos, "the end of the line")
        }
    }

    /// A `#` line: printable ASCII only.
    fn comment(&self) -> Result<(), Refusal> {
        match self.bytes.iter().position(|&b| !facts::printable(b)) {
            Some(at) => self.refuse(at, Unreadable::Byte(self.bytes[at])),
            None => Ok(()),
        }
    }

    /// The key at column 1 and the spaces after it.
    fn key(&mut self) -> Result<&'a str, Refusal> {
        let len = self
            .bytes
            .iter()
            .take_while(|&&b| b.is_ascii_lowercase() || b == b'-')
            .count();
        let key = core::str::from_utf8(&self.bytes[..len]).unwrap_or_default();
        if !HEADER.contains(&key) && !KEYS.contains(&key) {
            return match self.bytes.first() {
                Some(&b) if !facts::printable(b) => self.refuse(0, Unreadable::Byte(b)),
                _ => self.refuse(0, Unreadable::Form),
            };
        }
        self.pos = len;
        if self.bytes.get(self.pos) != Some(&b' ') {
            return self.expected(self.pos, "a space");
        }
        while self.bytes.get(self.pos) == Some(&b' ') {
            self.pos += 1;
        }
        Ok(key)
    }

    fn header(&mut self, h: usize, header: &mut Header) -> Result<(), Refusal> {
        if h == 0 {
            return self.lit("1");
        }
        let claim = if self.rest() == b"none" {
            self.pos += 4;
            None
        } else {
            let (a, pos) = facts::addr_at(self.bytes, self.pos, self.line)?;
            self.pos = pos;
            Some(a)
        };
        let slot = match h {
            1 => &mut header.result,
            2 => &mut header.program,
            3 => &mut header.facts,
            4 => &mut header.engine,
            _ => &mut header.dark_order,
        };
        *slot = claim;
        Ok(())
    }

    fn record(&mut self, key: &str, out: &mut Performance) -> Result<(), Refusal> {
        let line = self.line;
        match key {
            "layer" => {
                let run = self.run();
                let tick = self.nat()?;
                self.lit(" ")?;
                let atom = self.atom()?;
                out.layers.push(Layer {
                    run,
                    tick,
                    atom,
                    line,
                });
            }
            "derive" => {
                let run = self.run();
                let head = self.atom()?;
                self.lit(" by ")?;
                let rule = self.rule_id()?;
                let mut premises = Vec::new();
                if !self.at_end() {
                    self.lit(" from ")?;
                    premises.push(self.atom()?);
                    while !self.at_end() {
                        self.lit(" & ")?;
                        premises.push(self.atom()?);
                    }
                }
                out.derives.push(Derive {
                    run,
                    head,
                    rule,
                    premises,
                    line,
                });
            }
            "context" => {
                let run = match self.run() {
                    Run::Tick => return self.expected(self.pos, "`under` or `over`"),
                    run => run,
                };
                let index = self.nat()?;
                self.lit(" = [")?;
                let mut atoms = Vec::new();
                if !self.take("]") {
                    atoms.push(self.atom()?);
                    while !self.take("]") {
                        self.lit(" & ")?;
                        atoms.push(self.atom()?);
                    }
                }
                out.contexts.push(Context {
                    run,
                    index,
                    atoms,
                    line,
                });
            }
            "dark" => {
                let dark = self.dark()?;
                out.dark.push((dark, line));
            }
            "violating" => {
                let atom = self.atom()?;
                let by = if self.at_end() {
                    None
                } else {
                    self.lit(" by ")?;
                    Some(self.functional()?)
                };
                out.violating.push(Violating { atom, by, line });
            }
            "erasure" => {
                let run = if self.take("tick ") {
                    Run::Tick
                } else {
                    match self.run() {
                        Run::Tick => return self.expected(self.pos, "`tick`, `under` or `over`"),
                        run => run,
                    }
                };
                self.lit("budget count ")?;
                let start = self.pos;
                let count = self.nat()?;
                if count == 0 {
                    return self.refuse(start, Unreadable::Expected("a count of at least 1"));
                }
                self.lit(" ")?;
                let rounds = if self.take("rounds") {
                    true
                } else if self.take("ticks") {
                    false
                } else {
                    return self.expected(self.pos, "`ticks` or `rounds`");
                };
                out.erasures.push(Erasure {
                    run,
                    count,
                    rounds,
                    line,
                });
            }
            _ => {
                // `hole`: the text, printable ASCII, to the end of the line.
                let text = self.rest();
                if let Some(at) = text.iter().position(|&b| !facts::printable(b)) {
                    return self.refuse(self.pos + at, Unreadable::Byte(text[at]));
                }
                if text.is_empty() {
                    return self.expected(self.pos, "a hole's text");
                }
                self.pos = self.bytes.len();
                out.holes
                    .push((text.iter().map(|&b| b as char).collect(), line));
            }
        }
        Ok(())
    }

    /// `under ` or `over ` names W's run; no word is the positive run.
    fn run(&mut self) -> Run {
        if self.take("under ") {
            Run::Under
        } else if self.take("over ") {
            Run::Over
        } else {
            Run::Tick
        }
    }

    fn nat(&mut self) -> Result<u64, Refusal> {
        let (n, pos) = facts::nat_at(self.bytes, self.pos, self.line)?;
        self.pos = pos;
        Ok(n)
    }

    fn atom(&mut self) -> Result<Atom, Refusal> {
        let (atom, pos) = facts::atom_at(self.bytes, self.pos, self.line)?;
        self.pos = pos;
        Ok(atom)
    }

    /// A rule id or a variable's name: `lower { lower | digit | "_" | "-" }`.
    fn rule_id(&mut self) -> Result<String, Refusal> {
        let rest = self.rest();
        if !rest.first().is_some_and(u8::is_ascii_lowercase) {
            return self.expected(self.pos, "a rule id");
        }
        let len = rest
            .iter()
            .take_while(|&&b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-'
            })
            .count();
        self.pos += len;
        Ok(rest[..len].iter().map(|&b| b as char).collect())
    }

    /// A predicate name: an atom with no arguments.
    fn pred(&mut self) -> Result<String, Refusal> {
        let start = self.pos;
        let atom = self.atom()?;
        if !atom.args.is_empty() {
            return self.refuse(start, Unreadable::Expected("a predicate name"));
        }
        Ok(atom.pred)
    }

    fn dark(&mut self) -> Result<Dark, Refusal> {
        let start = self.pos;
        match self.nat()? {
            2 => {
                self.lit(" ")?;
                if self.take("at ") {
                    let line = self.nat()?;
                    self.lit(":")?;
                    let column = self.nat()?;
                    Ok(Dark::At { line, column })
                } else if self.take("engine ") {
                    let engine = if self.take("arity ") {
                        Engine::Arity(self.pred()?)
                    } else if self.take("safety ") {
                        let rule = self.rule_id()?;
                        self.lit(" ?")?;
                        let var = self.rule_id()?;
                        Engine::Safety { rule, var }
                    } else if self.take("declaration ") {
                        Engine::Declaration(self.pred()?)
                    } else {
                        return self.expected(self.pos, "`arity`, `safety` or `declaration`");
                    };
                    Ok(Dark::Engine(engine))
                } else {
                    self.expected(self.pos, "`at` or `engine`")
                }
            }
            255 => {
                self.lit(" ")?;
                Ok(Dark::Declined(self.functional()?))
            }
            _ => self.refuse(start, Unreadable::Expected("`2` or `255`")),
        }
    }

    /// `functional <pred>[(<k₁>, …)] -> <v>`.
    fn functional(&mut self) -> Result<Functional, Refusal> {
        self.lit("functional ")?;
        let start = self.pos;
        let rest = self.rest();
        let len = rest
            .iter()
            .take_while(|&&b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            .count();
        if !rest.first().is_some_and(u8::is_ascii_lowercase) {
            return self.expected(start, "a predicate name");
        }
        self.pos += len;
        let pred = rest[..len].iter().map(|&b| b as char).collect();
        let mut key = Vec::new();
        if self.take("(") {
            key.push(self.nat()?);
            while !self.take(")") {
                self.lit(", ")?;
                key.push(self.nat()?);
            }
        }
        self.lit(" -> ")?;
        let value = self.nat()?;
        Ok(Functional { pred, key, value })
    }
}
