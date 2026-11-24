//! Bad input never panics (docs/spec/mcp.md §4): a seeded property over
//! random byte strings and over byte mutations of valid messages, through
//! the server's line loop. Every answer reads back as a JSON-RPC response.

use std::panic;

use prism_mcp::json::Value;
use prism_mcp::server;

use crate::common::{answers, call, echo_arguments, get, has, initialize, initialized, str_of};

/// xorshift64*, seeded: the same cases every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Every output line is a response: jsonrpc 2.0, an id, and exactly one of
/// result and error.
fn well_formed(out: &[u8]) {
    for a in answers(out) {
        assert_eq!(str_of(get(&a, &["jsonrpc"])), "2.0");
        assert!(has(&a, "id"));
        assert!(has(&a, "result") != has(&a, "error"), "{a:?}");
        if has(&a, "error") {
            assert!(matches!(get(&a, &["error", "code"]), Value::Int(_)));
        }
    }
}

fn survive(input: &[u8]) {
    let out = panic::catch_unwind(|| server::session(input))
        .unwrap_or_else(|_| panic!("panicked on {:?}", String::from_utf8_lossy(input)));
    well_formed(&out);
}

#[test]
fn random_bytes_never_panic() {
    let mut rng = Rng(0x5EED_2026_1124);
    let alphabet = b"{}[]\",:0123456789-+.eEtruefalsn \\u\n\r\t\x00\xc3\xa9\xff\xed\xa0\x80abc";
    for _ in 0..3000 {
        let len = rng.below(200);
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                if rng.below(4) == 0 {
                    rng.next() as u8
                } else {
                    alphabet[rng.below(alphabet.len())]
                }
            })
            .collect();
        survive(&bytes);
    }
}

#[test]
fn mutated_messages_never_panic() {
    let ready = format!("{}\n{}\n", initialize(1), initialized());
    let seeds = [
        initialize(1),
        call(2, "echo", &echo_arguments("pos-v01", true)),
        call(
            3,
            "chorus",
            r#"{"note":"a\n\n","ed25519":"x+00000000+AA==","mldsa44":"y","inclusion":{"index":0,"leaf":"","path":["AA=="]}}"#,
        ),
        r#"{"jsonrpc":"2.0","id":"p","method":"tools/list","params":{}}"#.into(),
    ];
    let mut rng = Rng(0xC0FF_EE00_0025);
    for seed in &seeds {
        for _ in 0..150 {
            let mut m = seed.clone().into_bytes();
            for _ in 0..1 + rng.below(4) {
                let at = rng.below(m.len());
                match rng.below(3) {
                    0 => m[at] = rng.next() as u8,
                    1 => {
                        m.remove(at);
                    }
                    _ => {
                        let structural = b"\"{}[],:\\0e-";
                        m.insert(at, structural[rng.below(structural.len())])
                    }
                }
            }
            let mut input = ready.clone().into_bytes();
            input.extend_from_slice(&m);
            input.push(b'\n');
            survive(&input);
        }
    }
}
