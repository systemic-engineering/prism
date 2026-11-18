//! Halevi–Micali commitments (CRYPTO '96) over SHA-256, k = 256.
//!
//! The committer lives here, in the tests, and only here: the library checks
//! openings and never commits (it has no RNG). Its coins are a seeded SHA-256
//! stream, so every run is the same run. It is written from the paper
//! (§2.1 p.9, §3 p.11, fn.4 p.11, §4 p.15) and the layout in
//! docs/spec/ledger.md §5, not from the library. Independent vectors:
//! docs/spec/commitment-vectors.json, made by commitment-vectors.py (hashlib).

use chorus::commitment::{
    verify_opening, OpeningError, COMMITMENT, DIAGONALS, K, L, N, OPENING, SCHEME,
};
use serde_json::Value;

use crate::common::{flip, hex, sha256};

const VECTORS: &str = include_str!("../../docs/spec/commitment-vectors.json");

// --- bit strings, MSB first (docs/spec/ledger.md §5) ---------------------

fn bits(bytes: &[u8], count: usize) -> Vec<u8> {
    (0..count)
        .map(|i| (bytes[i / 8] >> (7 - i % 8)) & 1)
        .collect()
}

fn pack(bits: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; bits.len().div_ceil(8)];
    for (i, bit) in bits.iter().enumerate() {
        out[i / 8] |= bit << (7 - i % 8);
    }
    out
}

/// `count` fixed "random" bits: SHA-256(label ‖ seed ‖ i), i = 0, 1, ...
fn coins(label: &[u8], seed: u32, count: usize) -> Vec<u8> {
    let mut data = Vec::new();
    let mut i = 0u32;
    while data.len() * 8 < count {
        data.extend_from_slice(&sha256(&[label, &seed.to_be_bytes(), &i.to_be_bytes()]));
        i += 1;
    }
    bits(&data, count)
}

/// §2.1 (p.9): A[i][j] = t[i − j + L − 1], an n × L Toeplitz matrix.
/// h_{A,b}(r) = A r + b over GF(2), row by row: the definition.
fn apply_by_rows(t: &[u8], b: &[u8], r: &[u8]) -> Vec<u8> {
    (0..N)
        .map(|i| (0..L).fold(b[i], |acc, j| acc ^ (t[i + L - 1 - j] & r[j])))
        .collect()
}

/// The same product, column by column (faster, for the hiding check): the
/// sum of the columns j with r[j] = 1; column j is t[L − 1 − j ..][..n].
fn apply_by_columns(t: &[u8], b: &[u8], r: &[u8]) -> Vec<u8> {
    let mut acc = b.to_vec();
    for j in (0..L).filter(|&j| r[j] == 1) {
        for (i, a) in acc.iter_mut().enumerate() {
            *a ^= t[L - 1 - j + i];
        }
    }
    acc
}

/// Send(1^k, m) with its coins (r, t) given (§4 over §3):
/// s = SHA-256(m); y = SHA-256(r); b = s − A r (fn.4); c = 0x01 ‖ t ‖ b ‖ y.
fn commit_with(m: &[u8], r: &[u8], t: &[u8]) -> (Vec<u8>, Vec<u8>) {
    assert_eq!((r.len(), t.len()), (L, DIAGONALS));
    let s = bits(&sha256(&[m]), K);
    let opening = pack(r);
    let y = sha256(&[&opening]);
    let a_r = apply_by_columns(t, &[0; N], r);
    let b: Vec<u8> = s.iter().zip(&a_r).map(|(s, a)| s ^ a).collect();
    let mut c = vec![SCHEME];
    c.extend(pack(t));
    c.extend(pack(&b));
    c.extend(y);
    (c, opening)
}

/// A commitment to `m` with seeded coins.
fn commit(m: &[u8], seed: u32) -> (Vec<u8>, Vec<u8>) {
    commit_with(m, &coins(b"r", seed, L), &coins(b"t", seed, DIAGONALS))
}

/// The commitment's fields as bit strings: (t, b, y).
fn fields(c: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let t_end = 1 + DIAGONALS.div_ceil(8);
    (
        bits(&c[1..t_end], DIAGONALS),
        bits(&c[t_end..t_end + N / 8], N),
        c[t_end + N / 8..].to_vec(),
    )
}

/// The paper's equation, written out: Receive accepts (c, (m, r)) iff
/// y = MD(r) (§3, p.11) and h(r) = MD(m) (§4, p.15).
fn paper_accepts(c: &[u8], m: &[u8], opening: &[u8]) -> bool {
    let (t, b, y) = fields(c);
    y == sha256(&[opening]) && apply_by_rows(&t, &b, &bits(opening, L)) == bits(&sha256(&[m]), K)
}

const MESSAGES: [&[u8]; 4] = [b"", b"abc", b"a record that will be erased", &[0xff; 200]];

// --- sizes ----------------------------------------------------------------

#[test]
fn the_sizes_are_the_papers_at_k_256() {
    assert_eq!(L, 1540, "L = 4k + 2n + 4 with n = k (p.11, p.15)");
    assert_eq!(L, 6 * K + 4, "|r| = 6k + 4");
    assert_eq!(DIAGONALS, 1795, "an n × L Toeplitz matrix: n + L − 1 (p.9)");
    // |h| + |y| = (2n + L − 1) + k = 9k + 3 bits: the paper's "9k" (p.15)
    // drops the constant.
    assert_eq!(DIAGONALS + N + K, 9 * K + 3);
    assert_eq!(OPENING, 193);
    assert_eq!(COMMITMENT, 290, "0x01 ‖ t (225) ‖ b (32) ‖ y (32)");
    for m in MESSAGES.iter().copied().chain([&[7u8; 10_000][..]]) {
        let (c, d) = commit(m, 0);
        assert_eq!(
            (c.len(), d.len()),
            (COMMITMENT, OPENING),
            "independent of |m|"
        );
    }
}

#[test]
fn the_committer_is_the_definition() {
    for seed in 0..3 {
        let (t, r, b) = (
            coins(b"t", seed, DIAGONALS),
            coins(b"r", seed, L),
            coins(b"b", seed, N),
        );
        assert_eq!(apply_by_columns(&t, &b, &r), apply_by_rows(&t, &b, &r));
    }
}

// --- the independent vectors ----------------------------------------------

/// (name, message, opening, commitment).
type Vector = (String, Vec<u8>, Vec<u8>, Vec<u8>);

fn vectors() -> Vec<Vector> {
    let json: Value = serde_json::from_str(VECTORS).unwrap();
    assert_eq!(json["k"], 256);
    assert_eq!(json["L"], 1540);
    assert_eq!(json["scheme"], SCHEME);
    json["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            let s = |k: &str| hex(v[k].as_str().unwrap());
            (
                v["name"].as_str().unwrap().to_owned(),
                s("message"),
                s("opening"),
                s("commitment"),
            )
        })
        .collect()
}

#[test]
fn the_python_vectors_open() {
    let vectors = vectors();
    assert_eq!(vectors.len(), 4);
    for (name, m, d, c) in vectors {
        assert_eq!(verify_opening(&c, &m, &d), Ok(()), "{name}");
        assert!(paper_accepts(&c, &m, &d), "{name}");
        // The tests' committer, from the same coins, makes the same bytes.
        let (t, _, _) = fields(&c);
        assert_eq!(commit_with(&m, &bits(&d, L), &t), (c, d), "{name}");
    }
}

#[test]
fn mutated_python_vectors_reject() {
    for (name, m, d, c) in vectors() {
        let mut other = m.clone();
        other.push(0);
        assert_eq!(
            verify_opening(&c, &other, &d),
            Err(OpeningError::Message),
            "{name}"
        );
        let r = bits(&d, L);
        for at in 0..COMMITMENT * 8 {
            let got = verify_opening(&flip_bit(&c, at), &m, &d);
            let unread_t_bit = (8..8 + DIAGONALS).contains(&at) && unread(&r, at - 8);
            assert_eq!(got.is_ok(), unread_t_bit, "{name}: c bit {at}");
        }
        for at in 0..OPENING * 8 {
            assert!(
                verify_opening(&c, &m, &flip_bit(&d, at)).is_err(),
                "{name}: r bit {at}"
            );
        }
        assert_eq!(
            verify_opening(&c[..COMMITMENT - 1], &m, &d),
            Err(OpeningError::Length)
        );
        assert_eq!(
            verify_opening(&c, &m, &d[..OPENING - 1]),
            Err(OpeningError::Length)
        );
    }
}

/// Diagonal bit t[o] enters A r only through the columns j with
/// L − 1 − j ≤ o ≤ L − 1 − j + (n − 1). If r is 0 on every one of them,
/// flipping t[o] leaves h(r) unchanged. That happens only at the corners of
/// A (o < n − 1 or o > L − 1), each with probability 2^−(columns).
fn unread(r: &[u8], o: usize) -> bool {
    let lo = (L - 1).saturating_sub(o);
    let hi = (L + N - 2 - o).min(L - 1);
    (lo..=hi).all(|j| r[j] == 0)
}

fn flip_bit(bytes: &[u8], at: usize) -> Vec<u8> {
    let mut b = bytes.to_vec();
    b[at / 8] ^= 0x80 >> (at % 8);
    b
}

// --- opening ----------------------------------------------------------------

#[test]
fn a_commitment_opens_with_its_m_and_r() {
    for (seed, m) in MESSAGES.iter().enumerate() {
        let (c, d) = commit(m, seed as u32);
        assert_eq!(verify_opening(&c, m, &d), Ok(()));
    }
}

#[test]
fn the_wrong_message_rejects() {
    let (c, d) = commit(b"abc", 1);
    for m in [&b"abd"[..], b"ab", b"abcd", b"", b"ABC"] {
        assert_eq!(
            verify_opening(&c, m, &d),
            Err(OpeningError::Message),
            "{m:?}"
        );
    }
}

#[test]
fn the_wrong_r_rejects() {
    let (c, _) = commit(b"abc", 1);
    let (_, other) = commit(b"abc", 2);
    assert_eq!(
        verify_opening(&c, b"abc", &other),
        Err(OpeningError::Randomness)
    );
    assert_eq!(
        verify_opening(&c, b"abc", &[0; OPENING]),
        Err(OpeningError::Randomness)
    );
}

#[test]
fn a_truncated_or_extended_commitment_or_opening_rejects() {
    let (c, d) = commit(b"abc", 3);
    let mut longer_c = c.clone();
    longer_c.push(0);
    let mut longer_d = d.clone();
    longer_d.push(0);
    for (c, d) in [
        (&c[..COMMITMENT - 1], &d[..]),
        (&c[..1], &d[..]),
        (&[][..], &d[..]),
        (&longer_c[..], &d[..]),
        (&c[..], &d[..OPENING - 1]),
        (&c[..], &[][..]),
        (&c[..], &longer_d[..]),
    ] {
        assert_eq!(
            verify_opening(c, b"abc", d),
            Err(OpeningError::Length),
            "{} / {}",
            c.len(),
            d.len()
        );
    }
}

/// Every flipped bit rejects, with its reason, **except** a diagonal bit of
/// A that r never reads (see `unread`). That is the paper's construction,
/// not a bug: such a flip gives another commit-string that opens to the
/// *same* m with the same r. It is malleability, not ambiguity; binding
/// (p.12) is about two messages. In the log the leaf holds the exact bytes,
/// so a flipped commitment is another leaf. See docs/spec/ledger.md §5.
#[test]
fn a_flipped_bit_rejects_unless_r_never_reads_it() {
    const M: &[u8] = b"a record that will be erased";
    let (c, d) = commit(M, 4);
    let r = bits(&d, L);
    for at in 0..COMMITMENT * 8 {
        let flipped = flip_bit(&c, at);
        let got = verify_opening(&flipped, M, &d);
        let want = match at {
            0..8 => Err(OpeningError::Scheme),
            // t's 5 pad bits (1795..1800 of t, after the scheme byte).
            1803..1808 => Err(OpeningError::Padding),
            8..1803 if unread(&r, at - 8) => Ok(()),
            // y.
            _ if at >= (COMMITMENT - 32) * 8 => Err(OpeningError::Randomness),
            _ => Err(OpeningError::Message),
        };
        assert_eq!(got, want, "commitment bit {at}");
    }
    for at in 0..OPENING * 8 {
        let got = verify_opening(&c, b"a record that will be erased", &flip_bit(&d, at));
        let want = if at >= L {
            OpeningError::Padding
        } else {
            OpeningError::Randomness
        };
        assert_eq!(got, Err(want), "opening bit {at}");
    }
    assert_eq!(flip(&c, 0)[0], SCHEME ^ 1);
}

/// The python vector "empty" has r₀..r₅ = 0 and r₁₅₃₈ = r₁₅₃₉ = 0, so
/// the 8 corner diagonals only those columns read can be flipped and still
/// open, to that m only.
#[test]
fn an_unread_diagonal_flip_opens_only_to_the_same_message() {
    let (_, m, d, c) = vectors().remove(0);
    let r = bits(&d, L);
    let unread: Vec<usize> = (0..DIAGONALS).filter(|&o| unread(&r, o)).collect();
    assert_eq!(unread, [0, 1, 1789, 1790, 1791, 1792, 1793, 1794]);
    for o in unread {
        let flipped = flip_bit(&c, 8 + o);
        assert_eq!(verify_opening(&flipped, &m, &d), Ok(()));
        assert_eq!(
            verify_opening(&flipped, b"anything else", &d),
            Err(OpeningError::Message)
        );
    }
}

#[test]
fn another_scheme_byte_rejects() {
    let (mut c, d) = commit(b"abc", 5);
    for tag in [0x00, 0x02, 0xff] {
        c[0] = tag;
        assert_eq!(verify_opening(&c, b"abc", &d), Err(OpeningError::Scheme));
    }
}

// --- binding --------------------------------------------------------------

/// The verifier accepts exactly when the paper's equation holds:
/// y = SHA-256(r) and A r + b = SHA-256(m). Checked on honest commitments
/// and on commitments where one half, the other or both fail.
///
/// **Binding reduces to a SHA-256 collision** (p.12, "being able to open
/// the commitment in two different ways implies that the Sender can find a
/// collision in MD"; §4's step, omitted on p.15, is the same one line):
/// let (m, r) ≠ (m′, r′) both verify for c = ⟨t, b, y⟩ with m ≠ m′.
/// - If r ≠ r′: SHA-256(r) = y = SHA-256(r′), a collision (the 193-byte
///   packing is injective on {0,1}^L).
/// - If r = r′: SHA-256(m) = A r + b = A r′ + b = SHA-256(m′), a collision.
///
/// `collision` below is that extractor, and since acceptance is this
/// equation, it applies to anything the verifier accepts.
#[test]
fn the_check_is_the_papers_equation() {
    let mut cases = 0;
    for seed in 0..6u32 {
        let m = format!("message {seed}").into_bytes();
        let (c, d) = commit(&m, seed);
        let (_, other_d) = commit(&m, seed + 100);
        let mut other_b = c.clone();
        other_b[230] ^= 0x10;
        let mut other_y = c.clone();
        other_y[COMMITMENT - 1] ^= 0x01;
        for (c, m, d) in [
            (&c, &m[..], &d),       // both hold
            (&c, b"other", &d),     // y holds, h(r) ≠ MD(m)
            (&c, &m[..], &other_d), // neither: another r
            (&other_b, &m[..], &d), // y holds, b moved
            (&other_y, &m[..], &d), // h holds, y moved
        ] {
            assert_eq!(
                verify_opening(c, m, d).is_ok(),
                paper_accepts(c, m, d),
                "seed {seed}"
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 30);
}

/// Two accepting openings of one commitment to different messages give a
/// SHA-256 collision: the reduction above, as code.
fn collision(
    c: &[u8],
    (m, r): (&[u8], &[u8]),
    (m2, r2): (&[u8], &[u8]),
) -> Option<(Vec<u8>, Vec<u8>)> {
    if m == m2 || verify_opening(c, m, r).is_err() || verify_opening(c, m2, r2).is_err() {
        return None;
    }
    let pair = if r != r2 { (r, r2) } else { (m, m2) };
    assert_eq!(sha256(&[pair.0]), sha256(&[pair.1]), "a SHA-256 collision");
    Some((pair.0.to_vec(), pair.1.to_vec()))
}

#[test]
fn no_second_opening_is_found() {
    let (c, d) = commit(b"abc", 6);
    let (_, d2) = commit(b"abd", 6);
    let (_, d3) = commit(b"abc", 7);
    for (m2, r2) in [(&b"abd"[..], &d), (b"abd", &d2), (b"abd", &d3), (b"", &d)] {
        assert_eq!(collision(&c, (b"abc", &d), (m2, r2)), None);
    }
}

// --- hiding, a smoke check -----------------------------------------------

/// **Not a proof.** Theorem 1 (p.12; proof pp.12–15) is: for all m₁, m₂,
/// ‖C_k(m₁) − C_k(m₂)‖ < 2^−k, for every MD. This only checks a symptom:
/// over 200 seeded commitments to each of two messages, every bit of the
/// commitment is set with frequencies within 0.3 of each other (the
/// difference has σ = 0.05 at fair bits, so 6σ). A committer that skipped
/// the universal hash (b = s) fails it: those bits are constant and differ.
#[test]
fn hiding_smoke_check_per_bit_frequencies_match() {
    const SAMPLES: u32 = 200;
    let frequencies = |m: &[u8], base: u32| -> Vec<f64> {
        let mut ones = vec![0u32; COMMITMENT * 8];
        for seed in base..base + SAMPLES {
            let (c, _) = commit(m, seed);
            for (i, bit) in bits(&c, COMMITMENT * 8).into_iter().enumerate() {
                ones[i] += u32::from(bit);
            }
        }
        ones.into_iter()
            .map(|n| f64::from(n) / f64::from(SAMPLES))
            .collect()
    };
    let a = frequencies(b"yes", 1_000);
    let b = frequencies(b"no", 2_000);
    let worst = a
        .iter()
        .zip(&b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max);
    assert!(worst < 0.3, "largest per-bit frequency gap {worst}");
}
