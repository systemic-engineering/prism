//! The server (docs/spec/mcp.md §2, §4): JSON-RPC 2.0 messages, one per
//! line, under MCP 2025-06-18's lifecycle.
//!
//! Every path out of [`Server::handle`] is a value: a line that doesn't
//! read, a message that isn't a request, an unknown method, a bad argument
//! each get their JSON-RPC error, and a notification gets nothing. The
//! server holds no state but the lifecycle's.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::json::{self, Kind, Refusal, Value, MAX_BYTES};
use crate::tools;

/// The MCP revision this server speaks (§2).
pub const PROTOCOL: &str = "2025-06-18";

/// JSON-RPC 2.0 §5.1's codes, and two from its server-error range (§4).
pub const PARSE_ERROR: i64 = -32700;
pub const INVALID_REQUEST: i64 = -32600;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;
pub const NOT_INITIALIZED: i64 = -32000;
pub const ALREADY_INITIALIZED: i64 = -32001;

/// What `initialize` answers with in `instructions`.
const INSTRUCTIONS: &str = "Read-only: prism's echo and chorus verifiers. Inputs travel inline \
                            in each call; nothing is signed, stored, fetched or remembered.";

/// An object from its members.
pub fn object<const N: usize>(members: [(&str, Value); N]) -> Value {
    Value::Object(
        members
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    )
}

/// A string value.
pub fn text(s: &str) -> Value {
    Value::Str(s.into())
}

/// A JSON-RPC error object (§5.1): code, message, and data when there is
/// any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

impl Error {
    pub fn new(code: i64, message: &str) -> Error {
        Error {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn with(mut self, data: Value) -> Error {
        self.data = Some(data);
        self
    }

    /// `data: {at, refusal}` for a refusal of the reader's.
    pub fn refusal(code: i64, message: &str, r: Refusal) -> Error {
        Error::new(code, message).with(object([
            ("at", Value::Int(offset(r.at))),
            ("refusal", text(r.kind.word())),
        ]))
    }

    /// `-32602` with `data: {argument, expected}`.
    pub fn params(argument: &str, expected: &str) -> Error {
        Error::new(INVALID_PARAMS, "Invalid params").with(object([
            ("argument", text(argument)),
            ("expected", text(expected)),
        ]))
    }

    fn value(self) -> Value {
        let mut m = BTreeMap::new();
        m.insert("code".to_string(), Value::Int(self.code));
        m.insert("message".to_string(), Value::Str(self.message));
        if let Some(d) = self.data {
            m.insert("data".to_string(), d);
        }
        Value::Object(m)
    }
}

/// A byte offset as an integer: offsets are bounded by [`MAX_BYTES`].
fn offset(at: usize) -> i64 {
    i64::try_from(at).unwrap_or(i64::MAX)
}

fn invalid() -> Error {
    Error::new(INVALID_REQUEST, "Invalid Request")
}

/// A response line: `id`, `jsonrpc`, and the result or the error.
fn answer(id: Value, outcome: Result<Value, Error>) -> String {
    let (key, v) = match outcome {
        Ok(result) => ("result", result),
        Err(e) => ("error", e.value()),
    };
    json::print(&object([("id", id), ("jsonrpc", text("2.0")), (key, v)]))
}

/// The lifecycle (§4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum State {
    /// Before `initialize`: only `initialize` and `ping`.
    #[default]
    Fresh,
    /// `initialize` answered.
    Initialized,
    /// `notifications/initialized` received.
    Ready,
}

/// One session's state.
#[derive(Debug, Default)]
pub struct Server {
    state: State,
}

/// The members a request may have (JSON-RPC 2.0 §4).
const REQUEST: [&str; 4] = ["jsonrpc", "id", "method", "params"];

impl Server {
    pub fn new() -> Server {
        Server::default()
    }

    /// Answer one message (without its LF): the answer's line, or nothing.
    pub fn handle(&mut self, message: &[u8]) -> Option<String> {
        let m = match json::read(message) {
            Ok(Value::Object(m)) => m,
            // Not an object, an array included: 2025-06-18 has no batches.
            Ok(_) => return Some(answer(Value::Null, Err(invalid()))),
            Err(r) => {
                let e = Error::refusal(PARSE_ERROR, "Parse error", r);
                return Some(answer(Value::Null, Err(e)));
            }
        };
        // A client's response: the server sends no requests.
        if !m.contains_key("method") && (m.contains_key("result") || m.contains_key("error")) {
            return None;
        }
        let id = match m.get("id") {
            None => None,
            Some(v @ (Value::Str(_) | Value::Int(_))) => Some(v.clone()),
            Some(v) => {
                let e = match v.integer() {
                    Some(Err(r)) => Error::refusal(INVALID_REQUEST, "Invalid Request", r),
                    _ => invalid(),
                };
                return Some(answer(Value::Null, Err(e)));
            }
        };
        let reply = id.clone().unwrap_or(Value::Null);
        let method = match (m.get("jsonrpc"), m.get("method")) {
            (Some(Value::Str(v)), Some(Value::Str(method)))
                if v == "2.0" && m.keys().all(|k| REQUEST.contains(&k.as_str())) =>
            {
                method.as_str()
            }
            _ => return Some(answer(reply, Err(invalid()))),
        };
        let Some(id) = id else {
            // A notification gets nothing back, whatever it is.
            if method == "notifications/initialized" && self.state == State::Initialized {
                self.state = State::Ready;
            }
            return None;
        };
        let empty = BTreeMap::new();
        let params = match m.get("params") {
            None => &empty,
            Some(Value::Object(p)) => p,
            Some(_) => return Some(answer(id, Err(Error::params("params", "object")))),
        };
        Some(answer(id, self.dispatch(method, params)))
    }

    fn dispatch(&mut self, method: &str, params: &BTreeMap<String, Value>) -> Result<Value, Error> {
        match method {
            "initialize" if self.state != State::Fresh => {
                Err(Error::new(ALREADY_INITIALIZED, "Already initialized"))
            }
            "initialize" => {
                let result = initialize(params)?;
                self.state = State::Initialized;
                Ok(result)
            }
            "ping" => Ok(object([])),
            "tools/list" | "tools/call" if self.state == State::Fresh => {
                Err(Error::new(NOT_INITIALIZED, "Server not initialized"))
            }
            "tools/list" if params.contains_key("cursor") => Err(Error::params("cursor", "none")),
            "tools/list" => Ok(tools::list()),
            "tools/call" => tools::call(params),
            _ => Err(Error::new(METHOD_NOT_FOUND, "Method not found")),
        }
    }
}

/// lifecycle: the client's version, capabilities and implementation.
fn initialize(params: &BTreeMap<String, Value>) -> Result<Value, Error> {
    if !matches!(params.get("protocolVersion"), Some(Value::Str(_))) {
        return Err(Error::params("protocolVersion", "string"));
    }
    if !matches!(params.get("capabilities"), Some(Value::Object(_))) {
        return Err(Error::params("capabilities", "object"));
    }
    let Some(Value::Object(client)) = params.get("clientInfo") else {
        return Err(Error::params("clientInfo", "object"));
    };
    for field in ["name", "version"] {
        if !matches!(client.get(field), Some(Value::Str(_))) {
            return Err(Error::params("clientInfo", "name and version strings"));
        }
    }
    // Another version than ours is answered with ours (§2).
    Ok(object([
        ("capabilities", object([("tools", object([]))])),
        ("instructions", text(INSTRUCTIONS)),
        ("protocolVersion", text(PROTOCOL)),
        (
            "serverInfo",
            object([
                ("name", text("prism-mcp")),
                ("version", text(env!("CARGO_PKG_VERSION"))),
            ]),
        ),
    ]))
}

/// stdin split at LF (§4): an empty line is skipped, a line past
/// [`MAX_BYTES`] is discarded to its LF and refused.
#[derive(Debug, Default)]
pub struct Lines {
    line: Vec<u8>,
    over: bool,
}

impl Lines {
    pub fn new() -> Lines {
        Lines::default()
    }

    /// Read a chunk; every answer goes to `out`, each line ending in LF.
    pub fn push(&mut self, chunk: &[u8], server: &mut Server, out: &mut Vec<u8>) {
        let mut rest = chunk;
        while let Some(i) = rest.iter().position(|&b| b == b'\n') {
            self.take(&rest[..i]);
            self.end(server, out);
            rest = &rest[i + 1..];
        }
        self.take(rest);
    }

    /// End of input: a last line without LF is read.
    pub fn finish(&mut self, server: &mut Server, out: &mut Vec<u8>) {
        if self.over || !self.line.is_empty() {
            self.end(server, out);
        }
    }

    fn take(&mut self, bytes: &[u8]) {
        if self.over {
            return;
        }
        if self.line.len() + bytes.len() > MAX_BYTES {
            self.over = true;
            self.line = Vec::new();
        } else {
            self.line.extend_from_slice(bytes);
        }
    }

    fn end(&mut self, server: &mut Server, out: &mut Vec<u8>) {
        let reply = if self.over {
            let size = Refusal {
                kind: Kind::Size,
                at: MAX_BYTES,
            };
            Some(answer(
                Value::Null,
                Err(Error::refusal(PARSE_ERROR, "Parse error", size)),
            ))
        } else if self.line.is_empty() {
            None
        } else {
            server.handle(&self.line)
        };
        if let Some(r) = reply {
            out.extend_from_slice(r.as_bytes());
            out.push(b'\n');
        }
        self.line.clear();
        self.over = false;
    }
}

/// A whole session over `input`: the bytes the server writes.
pub fn session(input: &[u8]) -> Vec<u8> {
    let mut server = Server::new();
    let mut lines = Lines::new();
    let mut out = Vec::new();
    lines.push(input, &mut server, &mut out);
    lines.finish(&mut server, &mut out);
    out
}
