//! A session over scripted stdin (docs/spec/mcp.md §2, §4, §5.1):
//! initialize → initialized → tools/list → tools/call echo, and echo's
//! bytes against the binary's.

use std::io::Write;
use std::process::{Command, Stdio};

use prism_mcp::json::Value;
use prism_mcp::tools;

use crate::common::{
    answers, b3, call, echo_arguments, fixture, get, has, initialize, initialized, is_error, lit,
    ready, repo, session, str_of, texts,
};
use crate::digests::{PARITY, WITNESS};

#[test]
fn initialize_answers_with_the_revision_and_tools_only() {
    let out = session(&[initialize(1), initialized()]);
    assert_eq!(out.len(), 1, "a notification gets no answer: {out:?}");
    let a = &out[0];
    assert_eq!(str_of(get(a, &["jsonrpc"])), "2.0");
    assert_eq!(get(a, &["id"]), &Value::Int(1));
    assert_eq!(str_of(get(a, &["result", "protocolVersion"])), "2025-06-18");
    assert_eq!(
        prism_mcp::json::print(get(a, &["result", "capabilities"])),
        r#"{"tools":{}}"#
    );
    assert_eq!(
        str_of(get(a, &["result", "serverInfo", "name"])),
        "prism-mcp"
    );
    assert_eq!(
        str_of(get(a, &["result", "serverInfo", "version"])),
        env!("CARGO_PKG_VERSION")
    );
    assert!(str_of(get(a, &["result", "instructions"])).contains("Read-only"));
    assert!(!has(a, "error"));
}

/// §2: "Otherwise, the server MUST respond with another protocol version
/// it supports."
#[test]
fn another_revision_is_answered_with_ours() {
    let other = initialize(7).replace("2025-06-18", "2024-11-05");
    let out = session(&[other]);
    assert_eq!(
        str_of(get(&out[0], &["result", "protocolVersion"])),
        "2025-06-18"
    );
}

#[test]
fn tools_list_names_echo_and_chorus_with_their_schemas() {
    let out = ready(&[r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#.into()]);
    assert_eq!(out.len(), 1);
    let tools = match get(&out[0], &["result", "tools"]) {
        Value::Array(t) => t,
        other => panic!("{other:?}"),
    };
    let names: Vec<&str> = tools.iter().map(|t| str_of(get(t, &["name"]))).collect();
    assert_eq!(names, ["echo", "chorus"]);
    assert!(!has(get(&out[0], &["result"]), "nextCursor"));
    let echo = &tools[0];
    assert_eq!(
        str_of(get(echo, &["description"])),
        tools::echo_description()
    );
    assert_eq!(str_of(get(echo, &["inputSchema", "type"])), "object");
    assert_eq!(
        prism_mcp::json::print(get(echo, &["inputSchema", "required"])),
        r#"["source","certificate","record"]"#
    );
    assert_eq!(
        get(echo, &["inputSchema", "additionalProperties"]),
        &Value::Bool(false)
    );
    for p in ["source", "certificate", "record", "score"] {
        assert_eq!(
            str_of(get(echo, &["inputSchema", "properties", p, "type"])),
            "string"
        );
    }
    assert_eq!(
        str_of(get(echo, &["inputSchema", "properties", "witness", "type"])),
        "boolean"
    );
    let chorus = &tools[1];
    assert_eq!(
        prism_mcp::json::print(get(chorus, &["inputSchema", "required"])),
        r#"["note","ed25519","mldsa44"]"#
    );
    assert_eq!(
        str_of(get(
            chorus,
            &[
                "inputSchema",
                "properties",
                "inclusion",
                "properties",
                "index",
                "type"
            ]
        )),
        "integer"
    );
}

/// Seam's naming, generated from echo's own constants (§5.1).
#[test]
fn the_echo_description_is_generated_from_echo() {
    let d = tools::echo_description();
    assert_eq!(
        d,
        "An independent replayer of the positive fragment (R1\u{2013}R4 of `docs/spec/echo.md` \
         §10: the surface, then the strata source, program, model), sharing blake3 1.8.7 and \
         the spec's authorship lineage with mirror. Strata it can't replay (contexts, verdict, \
         budget, address, score) answer `unknown`, never `rings`."
    );
    let replayed = &echo::replay::STRATA[..echo::replay::REPLAYED];
    assert!(d.contains(&replayed.join(", ")));
    assert!(d.contains(&echo::replay::STRATA[echo::replay::REPLAYED..].join(", ")));
    let manifest = std::fs::read_to_string(repo("echo/Cargo.toml")).expect("manifest");
    assert!(manifest.contains(r#"blake3 = { version = "=1.8.7""#));
}

/// §5.1: the texts, concatenated, are the binary's stdout followed by
/// `exit <code>\n`, byte for byte, on every fixture.
#[test]
fn the_echo_tool_is_the_binary_byte_for_byte() {
    let calls: Vec<String> = PARITY
        .iter()
        .enumerate()
        .map(|(i, (name, _))| call(i as i64 + 10, "echo", &echo_arguments(name, false)))
        .collect();
    let out = ready(&calls);
    assert_eq!(out.len(), PARITY.len());
    for (a, (name, want)) in out.iter().zip(PARITY) {
        assert!(!is_error(a), "{name}");
        let t = texts(a);
        assert_eq!(t.len(), 2, "{name}");
        assert!(t[1].starts_with("exit ") && t[1].ends_with('\n'), "{name}");
        assert_eq!(b3(t.concat().as_bytes()), want, "{name}: {t:?}");
    }
}

/// §5.1: with `witness: true` the record comes between, exactly as
/// `prism echo --witness` prints it.
#[test]
fn the_witness_is_the_binary_byte_for_byte() {
    for (i, (name, want)) in WITNESS.iter().enumerate() {
        let out = ready(&[call(i as i64, "echo", &echo_arguments(name, true))]);
        let t = texts(&out[0]);
        assert_eq!(t.len(), 3, "{name}");
        assert!(t[1].starts_with("---\nwitness 1\n"), "{name}");
        assert_eq!(b3(t.concat().as_bytes()), *want, "{name}: {t:?}");
        let plain = ready(&[call(0, "echo", &echo_arguments(name, false))]);
        assert_eq!(texts(&plain[0])[0], t[0], "{name}: parity lines unchanged");
    }
}

/// The floor: where the library says `unknown`, the tool never says `rings`.
#[test]
fn the_echo_tool_answers_what_the_library_answers() {
    for (name, _) in PARITY {
        let [source, certificate, record, score] = fixture(name);
        let inputs = echo::replay::Inputs {
            source: source.as_deref().map(str::as_bytes),
            certificate: certificate.as_deref().map(str::as_bytes),
            record: record.as_deref().map(str::as_bytes),
            score: score.as_deref().map(|s| Some(s.as_bytes())),
        };
        let answer = echo::replay::replay(&inputs, echo::replay::BUDGET);
        let out = ready(&[call(1, "echo", &echo_arguments(name, false))]);
        let t = texts(&out[0]);
        let first = t[0].lines().next().expect("a verdict");
        if matches!(answer.verdict, echo::replay::Verdict::Unreplayed(_)) {
            assert!(first.starts_with("unknown unreplayed "), "{name}: {first}");
        }
        assert_ne!(first, "rings", "{name}");
        assert_eq!(t[1], format!("exit {}\n", answer.exit()), "{name}");
    }
}

/// `heard` names the BLAKE3 of the exact bytes received, a score included.
#[test]
fn heard_names_the_bytes_received() {
    let args = format!(
        r#"{{"source":{},"certificate":{},"record":{},"score":{}}}"#,
        lit("a.\n"),
        lit("é"),
        lit(""),
        lit("s")
    );
    let out = ready(&[call(1, "echo", &args)]);
    let t = texts(&out[0]);
    let heard = t[0].lines().nth(1).expect("heard");
    assert_eq!(
        heard,
        format!(
            "heard source {} certificate {} record {} score {}",
            b3(b"a.\n"),
            b3("é".as_bytes()),
            b3(b""),
            b3(b"s")
        )
    );
    let no_score = ready(&[call(
        1,
        "echo",
        &format!(r#"{{"source":"","certificate":"","record":"","witness":false}}"#),
    )]);
    let t = texts(&no_score[0]);
    assert_eq!(t.len(), 2);
    assert!(t[0].lines().nth(1).unwrap().ends_with(" score none"));
}

/// The binary over a real pipe: stdin scripted, stdout only MCP messages,
/// nothing on stderr, exit 0 at end of input.
#[test]
fn the_binary_serves_a_scripted_stdin() {
    let (name, want) = PARITY[0];
    let script = [
        initialize(1),
        initialized(),
        r#"{"jsonrpc":"2.0","id":2,"method":"ping"}"#.to_string(),
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/list"}"#.to_string(),
        call(4, "echo", &echo_arguments(name, false)),
    ]
    .join("\n")
        + "\n";
    let mut child = Command::new(env!("CARGO_BIN_EXE_prism-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .spawn()
        .expect("spawns");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(script.as_bytes())
        .expect("writes");
    let o = child.wait_with_output().expect("exits");
    assert_eq!(o.status.code(), Some(0));
    assert!(
        o.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let out = answers(&o.stdout);
    let ids: Vec<&Value> = out.iter().map(|a| get(a, &["id"])).collect();
    assert_eq!(
        ids,
        [
            &Value::Int(1),
            &Value::Int(2),
            &Value::Int(3),
            &Value::Int(4)
        ]
    );
    assert_eq!(prism_mcp::json::print(get(&out[1], &["result"])), "{}");
    assert_eq!(b3(texts(&out[3]).concat().as_bytes()), want);
    // the same bytes as the library's session
    assert_eq!(o.stdout, prism_mcp::server::session(script.as_bytes()));
}

/// Two runs of one script give identical bytes.
#[test]
fn a_session_is_deterministic() {
    let script = format!(
        "{}\n{}\n{}\n",
        initialize(1),
        initialized(),
        call(2, "echo", &echo_arguments("pos-v01", true))
    );
    let a = prism_mcp::server::session(script.as_bytes());
    assert!(!a.is_empty());
    assert_eq!(a, prism_mcp::server::session(script.as_bytes()));
}
