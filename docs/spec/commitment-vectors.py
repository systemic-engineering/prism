"""Reference vectors for ledger-verify's Halevi-Micali commitments, written from the paper, not from prism.

S. Halevi and S. Micali, "Practical and Provably-Secure Commitment Schemes
from Collision-Free Hashing", CRYPTO '96 (shaih.github.io/pubs/hm96.pdf; page
numbers below are that PDF's). Instantiated with MD = SHA-256 and k = 256
(Alex, 2026-11-17). The byte layout is prism's and provisional; it is
docs/spec/ledger.md §5. Only hashlib.

Writes commitment-vectors.json next to this script:

    python3 docs/spec/commitment-vectors.py

The construction, step by step:

  §4 (p.15)   s = MD(m), a k-bit string; commit to s with the first scheme,
              so the first scheme's message length is n = k.
  §3 (p.11)   L = 4k + 2n + 4 (= 1540). MD : {0,1}^L -> {0,1}^k.
              H is a universal family from {0,1}^L to {0,1}^n.
              Commit: pick a random r in {0,1}^L, y = MD(r), then a random
              h in H with h(r) = s. Commit-string c = <h, y>, de-commit d = r.
  §2.1 (p.9)  H = { h_{A,b}(r) = A r + b over GF(2) }, A an n x L Toeplitz
              matrix ("fixed on the diagonals", A[i][j] = A[i+1][j+1]), so h
              is described by 2n + L - 1 bits: n + L - 1 for A's diagonals
              and n for b.
  fn.4 (p.11) a random h with h(r) = s: pick A at random, b = s - A r
              (over GF(2), minus is XOR).
  §3 (p.11),  Receive: check y = MD(r), compute h(r); §4: check that h(r) is
  §4 (p.15)   the committed s and that MD(m) = s.

prism's encodings (not the paper's; it works on bit strings):

  * A bit string is packed most significant bit first: bit i is bit
    (7 - i % 8) of byte i // 8. Trailing pad bits are zero.
  * MD on {0,1}^L is SHA-256 of r's 193-byte packing (4 zero pad bits).
    The packing is injective on {0,1}^L, so a collision of this MD is a
    SHA-256 collision; and the paper's hiding proof (pp.12-15) holds for
    *every* function MD : {0,1}^L -> {0,1}^k, so it holds for this one.
  * s = SHA-256(m) over m's bytes; the k = 256 bits of s are its 32 bytes,
    MSB first.
  * A[i][j] = t[i - j + L - 1] for 0 <= i < n, 0 <= j < L, with t the
    n + L - 1 = 1795 diagonal bits. Any indexing of the diagonals gives the
    same family; this one is prism's choice.
  * commitment = 0x01 || t (225 bytes, 5 zero pad bits) || b (32) || y (32):
    290 bytes. 0x01 names the scheme (provisional).
  * opening = r (193 bytes), with the message m given beside it.
"""

import hashlib
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))

K = 256
N = K  # §4: the first scheme is applied to s = MD(m), a k-bit string
L = 4 * K + 2 * N + 4  # §3, p.11
DIAGONALS = N + L - 1  # §2.1, p.9: an n x L Toeplitz matrix
SCHEME = 0x01


def sha256(data):
    return hashlib.sha256(data).digest()


def to_bits(data, count):
    return [(data[i // 8] >> (7 - i % 8)) & 1 for i in range(count)]


def from_bits(bits):
    out = bytearray((len(bits) + 7) // 8)
    for i, bit in enumerate(bits):
        out[i // 8] |= bit << (7 - i % 8)
    return bytes(out)


def stream(label, count):
    """count fixed 'random' bits: SHA-256(label || i) for i = 0, 1, ..."""
    data = b""
    i = 0
    while len(data) * 8 < count:
        data += sha256(label + i.to_bytes(4, "big"))
        i += 1
    return to_bits(data, count)


def toeplitz(t, i, j):
    """A[i][j] = t[i - j + L - 1]: constant on each diagonal."""
    return t[i - j + L - 1]


def apply(t, b, r):
    """h_{A,b}(r) = A r + b over GF(2), row by row from the definition."""
    out = []
    for i in range(N):
        bit = b[i]
        for j in range(L):
            bit ^= toeplitz(t, i, j) & r[j]
        out.append(bit)
    return out


def commit(m, r, t):
    """Send(1^k, m) with its coins (r, A) given: §4 over §3."""
    assert len(r) == L and len(t) == DIAGONALS
    s = to_bits(sha256(m), K)  # §4
    y = sha256(from_bits(r))  # §3: y = MD(r)
    zero = [0] * N
    a_r = apply(t, zero, r)  # A r
    b = [si ^ ai for si, ai in zip(s, a_r)]  # fn.4: b = s - A r
    commitment = bytes([SCHEME]) + from_bits(t) + from_bits(b) + y
    return commitment, from_bits(r)


def receive(commitment, m, opening):
    """Receive(1^k, c, d): §3 checks y = MD(r) and computes h(r); §4 checks
    MD(m) = s = h(r). Written separately from commit as a self-check."""
    assert commitment[0] == SCHEME
    t = to_bits(commitment[1:226], DIAGONALS)
    b = to_bits(commitment[226:258], N)
    y = commitment[258:290]
    r = to_bits(opening, L)
    return y == sha256(opening) and apply(t, b, r) == to_bits(sha256(m), K)


MESSAGES = [
    ("empty", b""),
    ("abc", b"abc"),
    ("one block", bytes(range(64))),
    ("a longer record", b"prism ledger: a record that will be erased. " * 23),
]


def main():
    vectors = []
    for index, (name, m) in enumerate(MESSAGES):
        r = stream(b"hm96 r " + bytes([index]), L)
        t = stream(b"hm96 t " + bytes([index]), DIAGONALS)
        commitment, opening = commit(m, r, t)
        assert len(commitment) == 290 and len(opening) == 193
        assert receive(commitment, m, opening)
        assert not receive(commitment, m + b"!", opening)
        vectors.append(
            {
                "name": name,
                "message": m.hex(),
                "opening": opening.hex(),
                "commitment": commitment.hex(),
            }
        )
    out = {
        "source": "docs/spec/commitment-vectors.py, from Halevi-Micali CRYPTO '96 "
        "(§2.1 p.9, §3 p.11, §4 p.15), MD = SHA-256, k = 256",
        "k": K,
        "L": L,
        "scheme": SCHEME,
        "vectors": vectors,
    }
    path = os.path.join(HERE, "commitment-vectors.json")
    with open(path, "w") as f:
        json.dump(out, f, indent=2)
        f.write("\n")
    print(f"wrote {len(vectors)} vectors to {path}")


if __name__ == "__main__":
    main()
