//! JSON-RPC 2.0 and MCP's message rules (docs/spec/mcp.md §4): every
//! malformed message gets its error, notifications get nothing.

use prism_mcp::json::{print, Value, MAX_BYTES};
use prism_mcp::server;

use crate::common::{
    answers, call, code, get, has, initialize, initialized, ready, session, str_of,
};

fn one(line: &str) -> Value {
    let out = session(&[line.to_string()]);
    assert_eq!(out.len(), 1, "{line}: {out:?}");
    out.into_iter().next().unwrap()
}

fn error_data(a: &Value) -> String {
    print(get(a, &["error", "data"]))
}

#[test]
fn a_line_that_does_not_read_is_a_parse_error_with_its_place() {
    let a = one(r#"{"jsonrpc":"2.0","id":1,"method":"ping","id":2}"#);
    assert_eq!(code(&a), -32700);
    assert_eq!(str_of(get(&a, &["error", "message"])), "Parse error");
    assert_eq!(get(&a, &["id"]), &Value::Null);
    assert_eq!(error_data(&a), r#"{"at":40,"refusal":"duplicate"}"#);
    let a = one("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"} x");
    assert_eq!(error_data(&a), r#"{"at":41,"refusal":"trailing"}"#);
    let a = one("[1,]");
    assert_eq!(code(&a), -32700);
}

#[test]
fn what_is_not_a_request_is_an_invalid_request() {
    for line in [
        r#"[]"#,
        r#"[{"jsonrpc":"2.0","id":1,"method":"ping"}]"#,
        r#""ping""#,
        r#"{"id":1,"method":"ping"}"#,
        r#"{"jsonrpc":"1.0","id":1,"method":"ping"}"#,
        r#"{"jsonrpc":2,"id":1,"method":"ping"}"#,
        r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#,
        r#"{"jsonrpc":"2.0","id":true,"method":"ping"}"#,
        r#"{"jsonrpc":"2.0","id":[1],"method":"ping"}"#,
        r#"{"jsonrpc":"2.0","id":1}"#,
        r#"{"jsonrpc":"2.0","id":1,"method":5}"#,
        r#"{"jsonrpc":"2.0","method":5}"#,
        r#"{"jsonrpc":"2.0","id":1,"method":"ping","extra":0}"#,
    ] {
        let a = one(line);
        assert_eq!(code(&a), -32600, "{line}");
        assert_eq!(str_of(get(&a, &["error", "message"])), "Invalid Request");
    }
    // the id is echoed when it reads as one
    let a = one(r#"{"jsonrpc":"1.0","id":"x","method":"ping"}"#);
    assert_eq!(get(&a, &["id"]), &Value::Str("x".into()));
    let a = one(r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#);
    assert_eq!(get(&a, &["id"]), &Value::Null);
}

/// §3.2: an exponent where an integer is expected, located.
#[test]
fn a_decimal_id_is_refused_at_its_first_byte() {
    let a = one(r#"{"jsonrpc":"2.0","id":1e0,"method":"ping"}"#);
    assert_eq!(code(&a), -32600);
    assert_eq!(get(&a, &["id"]), &Value::Null);
    assert_eq!(error_data(&a), r#"{"at":22,"refusal":"not-integer"}"#);
    let a = one(r#"{"jsonrpc":"2.0","id":1.0,"method":"ping"}"#);
    assert_eq!(error_data(&a), r#"{"at":22,"refusal":"not-integer"}"#);
}

#[test]
fn ids_are_strings_or_integers_and_come_back_as_sent() {
    let out = session(&[
        r#"{"jsonrpc":"2.0","id":"aé","method":"ping"}"#.into(),
        r#"{"jsonrpc":"2.0","id":-9223372036854775808,"method":"ping"}"#.into(),
        r#"{"jsonrpc":"2.0","id":0,"method":"ping","params":{}}"#.into(),
    ]);
    assert_eq!(get(&out[0], &["id"]), &Value::Str("aé".into()));
    assert_eq!(get(&out[1], &["id"]), &Value::Int(i64::MIN));
    assert_eq!(print(&out[2]), r#"{"id":0,"jsonrpc":"2.0","result":{}}"#);
}

#[test]
fn unknown_methods_are_not_found() {
    let out = ready(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"resources/list"}"#.into(),
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/lis"}"#.into(),
    ]);
    assert_eq!(out.len(), 2);
    for a in &out {
        assert_eq!(code(a), -32601);
        assert_eq!(str_of(get(a, &["error", "message"])), "Method not found");
    }
    // before initialize too
    assert_eq!(
        code(&one(r#"{"jsonrpc":"2.0","id":1,"method":"nope"}"#)),
        -32601
    );
}

/// "The receiver MUST NOT send a response" to a notification, whatever it
/// is; a client's response is ignored (the server sends no requests).
#[test]
fn notifications_and_responses_get_nothing() {
    let out = session(&[
        initialized(),
        r#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":1}}"#.into(),
        r#"{"jsonrpc":"2.0","method":"tools/list"}"#.into(),
        r#"{"jsonrpc":"2.0","method":"whatever","params":[1]}"#.into(),
        r#"{"jsonrpc":"2.0","id":5,"result":{}}"#.into(),
        r#"{"jsonrpc":"2.0","id":6,"error":{"code":1,"message":"x"}}"#.into(),
    ]);
    assert!(out.is_empty(), "{out:?}");
}

#[test]
fn before_initialize_only_initialize_and_ping() {
    let out = session(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#.into(),
        call(2, "echo", "{}"),
        r#"{"jsonrpc":"2.0","id":3,"method":"ping"}"#.into(),
        initialize(4),
        initialize(5),
        r#"{"jsonrpc":"2.0","id":6,"method":"tools/list"}"#.into(),
    ]);
    assert_eq!(code(&out[0]), -32000);
    assert_eq!(
        str_of(get(&out[0], &["error", "message"])),
        "Server not initialized"
    );
    assert_eq!(code(&out[1]), -32000);
    assert!(has(&out[2], "result"));
    assert!(has(&out[3], "result"));
    assert_eq!(code(&out[4]), -32001);
    assert_eq!(
        str_of(get(&out[4], &["error", "message"])),
        "Already initialized"
    );
    // served once initialize is answered, before `initialized`
    assert!(has(&out[5], "result"));
}

#[test]
fn initialize_needs_its_params() {
    for params in [
        "",
        r#","params":{}"#,
        r#","params":{"protocolVersion":1,"capabilities":{},"clientInfo":{"name":"a","version":"b"}}"#,
        r#","params":{"protocolVersion":"2025-06-18","capabilities":[],"clientInfo":{"name":"a","version":"b"}}"#,
        r#","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"a"}}"#,
        r#","params":{"protocolVersion":"2025-06-18","capabilities":{}}"#,
        r#","params":[]"#,
    ] {
        let line = format!(r#"{{"jsonrpc":"2.0","id":1,"method":"initialize"{params}}}"#);
        let a = one(&line);
        assert_eq!(code(&a), -32602, "{line}");
        assert_eq!(str_of(get(&a, &["error", "message"])), "Invalid params");
    }
    // a failed initialize leaves the session fresh
    let out = session(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#.into(),
        initialize(2),
    ]);
    assert!(has(&out[1], "result"));
}

#[test]
fn params_must_be_an_object() {
    let out = ready(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"ping","params":[]}"#.into(),
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":"x"}"#.into(),
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/list","params":{"cursor":"c"}}"#.into(),
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call"}"#.into(),
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":1}}"#.into(),
        r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"echo","arguments":[]}}"#
            .into(),
    ]);
    assert_eq!(out.len(), 6);
    for a in &out {
        assert_eq!(code(a), -32602, "{a:?}");
    }
}

/// server/tools: "Unknown tools" and "Invalid arguments" are protocol
/// errors.
#[test]
fn unknown_tools_and_bad_arguments_are_invalid_params() {
    let good = r#""source":"","certificate":"","record":"""#;
    let cases = [
        (call(1, "check", "{}"), r#"{"tool":"check"}"#),
        (
            call(2, "echo", r#"{"source":"","certificate":""}"#),
            r#"{"argument":"record","expected":"string"}"#,
        ),
        (
            call(3, "echo", &format!("{{{good},\"score\":1}}")),
            r#"{"argument":"score","expected":"string"}"#,
        ),
        (
            call(4, "echo", &format!("{{{good},\"witness\":\"yes\"}}")),
            r#"{"argument":"witness","expected":"boolean"}"#,
        ),
        (
            call(5, "echo", &format!("{{{good},\"path\":\"/etc\"}}")),
            r#"{"argument":"path","expected":"none"}"#,
        ),
        (
            call(6, "chorus", r#"{"note":"","ed25519":""}"#),
            r#"{"argument":"mldsa44","expected":"string"}"#,
        ),
    ];
    let lines: Vec<String> = cases.iter().map(|(l, _)| l.clone()).collect();
    let out = ready(&lines);
    for (a, (line, data)) in out.iter().zip(&cases) {
        assert_eq!(code(a), -32602, "{line}");
        assert_eq!(error_data(a), *data, "{line}");
    }
    assert_eq!(
        str_of(get(&out[0], &["error", "message"])),
        "Unknown tool: check"
    );
}

/// §4: framing. Empty lines are skipped, a CR is whitespace, a last line
/// without LF is read.
#[test]
fn lines_frame_messages() {
    let ping = |id: u32| format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"ping"}}"#);
    let input = format!("\n{}\r\n\n\n{}", ping(1), ping(2));
    let out = answers(&server::session(input.as_bytes()));
    let ids: Vec<&Value> = out.iter().map(|a| get(a, &["id"])).collect();
    assert_eq!(ids, [&Value::Int(1), &Value::Int(2)]);
    // chunked anywhere, the same bytes
    let whole = server::session(input.as_bytes());
    for cut in 0..input.len() {
        let mut s = server::Server::new();
        let mut lines = server::Lines::new();
        let mut out = Vec::new();
        lines.push(&input.as_bytes()[..cut], &mut s, &mut out);
        lines.push(&input.as_bytes()[cut..], &mut s, &mut out);
        lines.finish(&mut s, &mut out);
        assert_eq!(out, whole, "cut at {cut}");
    }
}

/// §3.3: a line past 8 MiB is discarded to its newline and refused; the
/// session goes on.
#[test]
fn an_oversized_line_is_refused_and_the_session_goes_on() {
    let mut input = vec![b' '; MAX_BYTES + 10];
    input.push(b'\n');
    input.extend_from_slice(br#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#);
    input.push(b'\n');
    input.extend_from_slice(&vec![b' '; MAX_BYTES + 1]);
    let out = answers(&server::session(&input));
    assert_eq!(out.len(), 3);
    assert_eq!(code(&out[0]), -32700);
    assert_eq!(
        error_data(&out[0]),
        format!(r#"{{"at":{MAX_BYTES},"refusal":"size"}}"#)
    );
    assert_eq!(get(&out[1], &["id"]), &Value::Int(1));
    assert_eq!(code(&out[2]), -32700);
    // exactly the limit is read
    let mut at_limit = vec![b' '; MAX_BYTES - 2];
    at_limit.extend_from_slice(b"[]");
    at_limit.push(b'\n');
    let out = answers(&server::session(&at_limit));
    assert_eq!(code(&out[0]), -32600);
}

/// §3.5: answers are printed canonically: members in byte order.
#[test]
fn answers_are_canonical() {
    let out = server::session(
        concat!(
            r#"{"method":"ping","id":1,"jsonrpc":"2.0"}"#,
            "\n",
            r#"{"method":"nope","id":2,"jsonrpc":"2.0"}"#,
            "\n"
        )
        .as_bytes(),
    );
    assert_eq!(
        String::from_utf8(out).unwrap(),
        concat!(
            r#"{"id":1,"jsonrpc":"2.0","result":{}}"#,
            "\n",
            r#"{"error":{"code":-32601,"message":"Method not found"},"id":2,"jsonrpc":"2.0"}"#,
            "\n"
        )
    );
}
