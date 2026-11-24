//! The tools (docs/spec/mcp.md §5): `echo` and `chorus`, read-only.
//!
//! Arguments are checked against the tool's schema first: a missing,
//! unknown or mistyped argument is a protocol error (`-32602`). What the
//! tool then finds is its answer, with `isError: false`, except when the
//! ruler itself is broken (a vkey that doesn't parse, bytes that don't
//! decode): then there is no verdict, and the result says `isError: true`.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;

use chorus::checkpoint::{self, CheckpointError, Ed25519Key, LogKeys, MlDsaKey};
use chorus::merkle::{self, ProofError};
use chorus::note::{decode_base64, encode_base64, NoteError, Vkey};
use echo::replay::{self, Inputs, BUDGET, REPLAYED, STRATA};
use echo::witness;

use crate::json::Value;
use crate::server::{object, text, Error, INVALID_PARAMS};

type Args = BTreeMap<String, Value>;

/// The echo tool's description (§5.1; Seam, prism-025 §2), from echo's own
/// constants: R1 is the surface, R2 onwards one stratum each.
pub fn echo_description() -> String {
    format!(
        "An independent replayer of the positive fragment (R1\u{2013}R{} of `docs/spec/echo.md` \
         \u{a7}10: the surface, then the strata {}), sharing blake3 1.8.7 and the spec's \
         authorship lineage with mirror. Strata it can't replay ({}) answer `unknown`, never \
         `rings`.",
        REPLAYED + 1,
        STRATA[..REPLAYED].join(", "),
        STRATA[REPLAYED..].join(", "),
    )
}

const CHORUS: &str = "Verifies a C2SP checkpoint note under the hybrid AND rule (Ed25519 and \
                      ML-DSA-44 must both verify; docs/spec/ledger.md) and, optionally, an RFC \
                      9162 inclusion proof against it. It grades and never admits: whether an \
                      attested fact is admitted is the caller's policy. Keys and bytes travel \
                      inline (base64 canonical); nothing is fetched.";

fn typed(kind: &str, description: &str) -> Value {
    object([("description", text(description)), ("type", text(kind))])
}

fn names<const N: usize>(names: [&str; N]) -> Value {
    Value::Array(names.iter().map(|n| text(n)).collect())
}

fn schema<const N: usize, const R: usize>(
    properties: [(&str, Value); N],
    required: [&str; R],
) -> Value {
    object([
        ("additionalProperties", Value::Bool(false)),
        ("properties", object(properties)),
        ("required", names(required)),
        ("type", text("object")),
    ])
}

fn echo_tool() -> Value {
    object([
        ("name", text("echo")),
        ("title", text("echo: replay a certificate")),
        ("description", Value::Str(echo_description())),
        (
            "inputSchema",
            schema(
                [
                    ("source", typed("string", "The source file's contents.")),
                    (
                        "certificate",
                        typed("string", "The certificate's contents."),
                    ),
                    ("record", typed("string", "The record's contents.")),
                    ("score", typed("string", "The score's contents, if any.")),
                    (
                        "witness",
                        typed(
                            "boolean",
                            "Also the witness record (docs/spec/echo.md §11).",
                        ),
                    ),
                ],
                ["source", "certificate", "record"],
            ),
        ),
    ])
}

fn chorus_tool() -> Value {
    let inclusion = object([
        ("additionalProperties", Value::Bool(false)),
        (
            "description",
            text("An RFC 9162 inclusion proof of one leaf in the checkpoint's tree."),
        ),
        (
            "properties",
            object([
                (
                    "index",
                    object([("minimum", Value::Int(0)), ("type", text("integer"))]),
                ),
                ("leaf", typed("string", "The leaf's bytes, base64.")),
                (
                    "path",
                    object([
                        (
                            "description",
                            text("The proof's hashes, base64, 32 bytes each."),
                        ),
                        ("items", object([("type", text("string"))])),
                        ("type", text("array")),
                    ]),
                ),
            ]),
        ),
        ("required", names(["index", "leaf", "path"])),
        ("type", text("object")),
    ]);
    object([
        ("name", text("chorus")),
        ("title", text("chorus: verify a checkpoint")),
        ("description", text(CHORUS)),
        (
            "inputSchema",
            schema(
                [
                    ("note", typed("string", "The signed checkpoint note.")),
                    ("ed25519", typed("string", "The log's Ed25519 vkey.")),
                    ("mldsa44", typed("string", "The log's ML-DSA-44 vkey.")),
                    ("inclusion", inclusion),
                ],
                ["note", "ed25519", "mldsa44"],
            ),
        ),
    ])
}

/// `tools/list`'s result.
pub fn list() -> Value {
    object([(
        "tools",
        Value::Array(alloc::vec![echo_tool(), chorus_tool()]),
    )])
}

/// `tools/call`.
pub fn call(params: &Args) -> Result<Value, Error> {
    let Some(Value::Str(name)) = params.get("name") else {
        return Err(Error::params("name", "string"));
    };
    let empty = Args::new();
    let args = match params.get("arguments") {
        None => &empty,
        Some(Value::Object(a)) => a,
        Some(_) => return Err(Error::params("arguments", "object")),
    };
    match name.as_str() {
        "echo" => echo(args),
        "chorus" => chorus(args),
        _ => Err(Error::new(INVALID_PARAMS, &format!("Unknown tool: {name}"))
            .with(object([("tool", text(name))]))),
    }
}

/// A tool result: text contents and `isError`.
fn result(texts: Vec<String>, is_error: bool) -> Value {
    let content = texts
        .into_iter()
        .map(|t| object([("text", Value::Str(t)), ("type", text("text"))]))
        .collect();
    object([
        ("content", Value::Array(content)),
        ("isError", Value::Bool(is_error)),
    ])
}

/// The first argument not in `known`, under `prefix`.
fn only(args: &Args, known: &[&str], prefix: &str) -> Result<(), Error> {
    match args.keys().find(|k| !known.contains(&k.as_str())) {
        Some(k) => Err(Error::params(&format!("{prefix}{k}"), "none")),
        None => Ok(()),
    }
}

fn string<'a>(args: &'a Args, key: &str, prefix: &str) -> Result<Option<&'a str>, Error> {
    match args.get(key) {
        None => Ok(None),
        Some(Value::Str(s)) => Ok(Some(s)),
        Some(_) => Err(Error::params(&format!("{prefix}{key}"), "string")),
    }
}

fn required<'a>(args: &'a Args, key: &str, prefix: &str) -> Result<&'a str, Error> {
    string(args, key, prefix)?.ok_or_else(|| Error::params(&format!("{prefix}{key}"), "string"))
}

// ----------------------------------------------------------------- echo

fn echo(args: &Args) -> Result<Value, Error> {
    only(
        args,
        &["source", "certificate", "record", "score", "witness"],
        "",
    )?;
    let source = required(args, "source", "")?;
    let certificate = required(args, "certificate", "")?;
    let record = required(args, "record", "")?;
    let score = string(args, "score", "")?;
    let witnessed = match args.get("witness") {
        None => false,
        Some(Value::Bool(b)) => *b,
        Some(_) => return Err(Error::params("witness", "boolean")),
    };
    let inputs = Inputs {
        source: Some(source.as_bytes()),
        certificate: Some(certificate.as_bytes()),
        record: Some(record.as_bytes()),
        score: score.map(|s| Some(s.as_bytes())),
    };
    // The library's answer, printed as the binary prints it; never a
    // verdict of this crate's own.
    let mut texts = Vec::new();
    let answer = if witnessed {
        let (answer, w) = witness::witness(&inputs, BUDGET);
        texts.push(answer.print());
        texts.push(w.print());
        answer
    } else {
        let answer = replay::replay(&inputs, BUDGET);
        texts.push(answer.print());
        answer
    };
    texts.push(format!("exit {}\n", answer.exit()));
    Ok(result(texts, false))
}

// --------------------------------------------------------------- chorus

fn note_word(e: NoteError) -> &'static str {
    match e {
        NoteError::Malformed => "malformed",
        NoteError::ControlCharacter => "control-character",
        NoteError::KeyName => "key-name",
        NoteError::Base64 => "base64",
        NoteError::ShortSignature => "short-signature",
        NoteError::KeyId => "key-id",
        NoteError::Vkey => "vkey",
    }
}

fn checkpoint_words(e: CheckpointError) -> String {
    match e {
        CheckpointError::Note(n) => format!("note {}", note_word(n)),
        CheckpointError::Body => "body".into(),
        CheckpointError::Extensions => "extensions".into(),
        CheckpointError::Key => "key".into(),
        CheckpointError::Duplicate => "duplicate".into(),
        CheckpointError::Unsigned { ed25519, mldsa44 } => {
            format!("unsigned ed25519 {ed25519} mldsa44 {mldsa44}")
        }
    }
}

fn proof_word(e: ProofError) -> &'static str {
    match e {
        ProofError::IndexOutOfRange => "index-out-of-range",
        ProofError::SizeOrder => "size-order",
        ProofError::EmptyFirst => "empty-first",
        ProofError::EmptyPath => "empty-path",
        ProofError::PathTooLong => "path-too-long",
        ProofError::PathTooShort => "path-too-short",
        ProofError::FirstRootMismatch => "first-root-mismatch",
        ProofError::RootMismatch => "root-mismatch",
    }
}

/// The inclusion argument, schema-checked: index, leaf, path (base64).
struct Inclusion<'a> {
    index: u64,
    leaf: &'a str,
    path: Vec<&'a str>,
}

fn inclusion(args: &Args) -> Result<Option<Inclusion<'_>>, Error> {
    let inc = match args.get("inclusion") {
        None => return Ok(None),
        Some(Value::Object(m)) => m,
        Some(_) => return Err(Error::params("inclusion", "object")),
    };
    only(inc, &["index", "leaf", "path"], "inclusion.")?;
    let natural = || Error::params("inclusion.index", "integer \u{2265} 0");
    let index = match inc.get("index").and_then(Value::integer) {
        Some(Ok(n)) => u64::try_from(n).map_err(|_| natural())?,
        // §3.2: an exponent where an integer is expected, located.
        Some(Err(r)) => {
            let mut e = natural();
            if let Some(Value::Object(d)) = &mut e.data {
                d.insert("refusal".into(), text(r.kind.word()));
                d.insert(
                    "at".into(),
                    Value::Int(i64::try_from(r.at).unwrap_or(i64::MAX)),
                );
            }
            return Err(e);
        }
        None => return Err(natural()),
    };
    let leaf = required(inc, "leaf", "inclusion.")?;
    let strings = || Error::params("inclusion.path", "array of strings");
    let Some(Value::Array(items)) = inc.get("path") else {
        return Err(strings());
    };
    let path = items
        .iter()
        .map(|p| match p {
            Value::Str(s) => Ok(s.as_str()),
            _ => Err(strings()),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(Inclusion { index, leaf, path }))
}

fn hex(out: &mut String, bytes: &[u8]) {
    for b in echo::address::hash(bytes) {
        let _ = write!(out, "{b:02x}");
    }
}

/// No verdict: the ruler is broken (§5.2).
fn broken(why: String) -> Result<Value, Error> {
    Ok(result(alloc::vec![why + "\n"], true))
}

fn chorus(args: &Args) -> Result<Value, Error> {
    only(args, &["note", "ed25519", "mldsa44", "inclusion"], "")?;
    let note = required(args, "note", "")?;
    let ed_text = required(args, "ed25519", "")?;
    let ml_text = required(args, "mldsa44", "")?;
    let inc = inclusion(args)?;

    // The ruler: both keys must parse and be of their type.
    let ed = match Vkey::parse(ed_text) {
        Ok(v) => v,
        Err(e) => return broken(format!("error ed25519 {}", note_word(e))),
    };
    let ed = match Ed25519Key::from_vkey(&ed) {
        Ok(k) => k,
        Err(e) => return broken(format!("error ed25519 {}", checkpoint_words(e))),
    };
    let ml = match Vkey::parse(ml_text) {
        Ok(v) => v,
        Err(e) => return broken(format!("error mldsa44 {}", note_word(e))),
    };
    let ml = match MlDsaKey::from_vkey(&ml) {
        Ok(k) => k,
        Err(e) => return broken(format!("error mldsa44 {}", checkpoint_words(e))),
    };
    // The proof's bytes must decode.
    let proof = match &inc {
        None => None,
        Some(i) => {
            let Ok(leaf) = decode_base64(i.leaf) else {
                return broken("error inclusion.leaf base64".into());
            };
            let mut path = Vec::new();
            for (n, p) in i.path.iter().enumerate() {
                let Ok(bytes) = decode_base64(p) else {
                    return broken(format!("error inclusion.path {n} base64"));
                };
                let Ok(hash) = chorus::Hash::try_from(bytes) else {
                    return broken(format!("error inclusion.path {n} length"));
                };
                path.push(hash);
            }
            Some((i.index, leaf, path))
        }
    };

    let mut heard = String::from("heard note ");
    hex(&mut heard, note.as_bytes());
    heard.push_str(" ed25519 ");
    hex(&mut heard, ed_text.as_bytes());
    heard.push_str(" mldsa44 ");
    hex(&mut heard, ml_text.as_bytes());
    match &proof {
        Some((_, leaf, path)) => {
            heard.push_str(" leaf ");
            hex(&mut heard, leaf);
            heard.push_str(" path ");
            hex(&mut heard, &path.concat());
        }
        None => heard.push_str(" leaf none path none"),
    }

    let keys = LogKeys {
        ed25519: ed,
        mldsa44: ml,
    };
    let out = match checkpoint::verify(note, &keys) {
        Err(e) => {
            let inclusion = match &proof {
                Some((index, _, _)) => format!("inclusion {index} unchecked"),
                None => "inclusion none".into(),
            };
            format!("reject {}\n{heard}\n{inclusion}\n", checkpoint_words(e))
        }
        Ok(v) => {
            let c = &v.checkpoint;
            let (first, inclusion) = match &proof {
                None => ("accept", String::from("inclusion none")),
                Some((index, leaf, path)) => {
                    let leaf = merkle::leaf_hash(leaf);
                    match merkle::verify_inclusion(*index, c.size, &leaf, path, &c.root) {
                        Ok(()) => ("accept", format!("inclusion {index} accept")),
                        Err(e) => (
                            "reject inclusion",
                            format!("inclusion {index} reject {}", proof_word(e)),
                        ),
                    }
                }
            };
            format!(
                "{first}\n{heard}\ncheckpoint {} {} {}\ntimestamp {}\n{inclusion}\n",
                c.origin,
                c.size,
                encode_base64(&c.root),
                v.timestamp
            )
        }
    };
    Ok(result(alloc::vec![out], false))
}
