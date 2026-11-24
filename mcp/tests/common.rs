//! Helpers: messages as text, a session over scripted stdin, and reading
//! the answers back with the crate's own reader.

use std::path::PathBuf;

use prism_mcp::json::{print, read, Value};
use prism_mcp::server;

/// A JSON string literal for `text`.
pub fn lit(text: &str) -> String {
    print(&Value::Str(text.into()))
}

pub fn initialize(id: i64) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"initialize","params":{{"protocolVersion":"2025-06-18","capabilities":{{}},"clientInfo":{{"name":"test","version":"0"}}}}}}"#
    )
}

pub fn initialized() -> String {
    r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#.into()
}

/// `tools/call` of `name` with `arguments` (JSON text).
pub fn call(id: i64, name: &str, arguments: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/call","params":{{"name":{},"arguments":{arguments}}}}}"#,
        lit(name)
    )
}

/// Run the server over `lines`, each followed by LF; every output line read
/// back as JSON.
pub fn session(lines: &[String]) -> Vec<Value> {
    let mut input = Vec::new();
    for l in lines {
        input.extend_from_slice(l.as_bytes());
        input.push(b'\n');
    }
    answers(&server::session(&input))
}

/// Every line of an output, read back; each must end in LF.
pub fn answers(out: &[u8]) -> Vec<Value> {
    assert!(out.is_empty() || out.ends_with(b"\n"), "{out:?}");
    out.split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .map(|l| read(l).unwrap_or_else(|r| panic!("{r:?}: {}", String::from_utf8_lossy(l))))
        .collect()
}

/// The session's answers after initialize and initialized.
pub fn ready(lines: &[String]) -> Vec<Value> {
    let mut all = vec![initialize(0), initialized()];
    all.extend_from_slice(lines);
    let mut out = session(&all);
    assert_eq!(get(&out[0], &["id"]), &Value::Int(0));
    out.remove(0);
    out
}

pub fn get<'a>(v: &'a Value, path: &[&str]) -> &'a Value {
    let mut v = v;
    for p in path {
        v = match v {
            Value::Object(m) => m.get(*p).unwrap_or_else(|| panic!("no {p} in {v:?}")),
            Value::Array(items) => &items[p.parse::<usize>().expect("index")],
            _ => panic!("{p} of {v:?}"),
        };
    }
    v
}

pub fn has(v: &Value, key: &str) -> bool {
    matches!(v, Value::Object(m) if m.contains_key(key))
}

pub fn str_of(v: &Value) -> &str {
    match v {
        Value::Str(s) => s,
        _ => panic!("not a string: {v:?}"),
    }
}

/// The error code of an answer.
pub fn code(v: &Value) -> i64 {
    match get(v, &["error", "code"]) {
        Value::Int(n) => *n,
        other => panic!("{other:?}"),
    }
}

/// The texts of a tool result's content, in order.
pub fn texts(v: &Value) -> Vec<String> {
    match get(v, &["result", "content"]) {
        Value::Array(items) => items
            .iter()
            .map(|c| {
                assert_eq!(str_of(get(c, &["type"])), "text");
                str_of(get(c, &["text"])).to_string()
            })
            .collect(),
        other => panic!("{other:?}"),
    }
}

pub fn is_error(v: &Value) -> bool {
    match get(v, &["result", "isError"]) {
        Value::Bool(b) => *b,
        other => panic!("{other:?}"),
    }
}

pub fn hex(a: &[u8; 32]) -> String {
    a.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn b3(bytes: &[u8]) -> String {
    hex(&echo::address::hash(bytes))
}

pub fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(rel)
}

/// A fixture's inputs as text, its `expect` redirections followed (as
/// echo/tests/witness.rs reads them): source, certificate, record, score.
pub fn fixture(name: &str) -> [Option<String>; 4] {
    let data = |rel: &str| repo("docs/spec/echo-vectors/mirror").join(rel);
    let expect = std::fs::read_to_string(data(&format!("fixtures/{name}/expect"))).expect("expect");
    let path = |key: &str, default: &str| -> Option<PathBuf> {
        let given = expect
            .lines()
            .find_map(|l| l.strip_prefix(key).and_then(|r| r.strip_prefix(' ')))
            .map(str::trim);
        match given {
            Some(p) if p.starts_with("../../vectors/") => Some(data(&p["../../vectors/".len()..])),
            Some(p) => Some(data(&format!("fixtures/{name}/{p}"))),
            None if default.is_empty() => None,
            None => Some(data(&format!("fixtures/{name}/{default}"))),
        }
    };
    let text = |p: Option<PathBuf>| {
        p.map(|p| String::from_utf8(std::fs::read(&p).expect("fixture")).expect("utf-8"))
    };
    [
        text(path("source", "source.pact")),
        text(path("certificate", "certificate")),
        text(path("record", "record")),
        text(path("score", "")),
    ]
}

/// The echo tool's arguments for a fixture.
pub fn echo_arguments(name: &str, witness: bool) -> String {
    let [source, certificate, record, score] = fixture(name);
    let mut a = format!(
        r#"{{"source":{},"certificate":{},"record":{}"#,
        lit(&source.expect("source")),
        lit(&certificate.expect("certificate")),
        lit(&record.expect("record"))
    );
    if let Some(s) = score {
        a.push_str(&format!(r#","score":{}"#, lit(&s)));
    }
    if witness {
        a.push_str(r#","witness":true"#);
    }
    a.push('}');
    a
}
