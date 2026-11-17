"""Reference vectors for ledger-verify's Merkle checks, written from RFC 9162, not from prism.

Writes two files next to this script:

ledger-vectors.json
    Every inclusion proof (0 <= index < size <= 8) and every consistency proof
    (0 < first < second <= 8) over the RFC 6962 test leaves, generated here
    with RFC 9162 §2.1.1 (MTH), §2.1.3.1 (PATH) and §2.1.4.1 (PROOF and
    SUBPROOF), with hashlib's SHA-256. The tree hashes are checked against the
    published root hashes of transparency-dev/merkle (testonly/constants.go)
    before anything is written.

ledger-probes.json (only with --probes DIR)
    transparency-dev/merkle's published inclusion and consistency probes
    (testdata/inclusion, testdata/consistency; Apache-2.0), bundled into one
    file unchanged: field names, base64 values, `desc` and `wantErr` as
    published. DIR is a checkout's testdata/ directory.

    python3 docs/spec/ledger-vectors.py
    python3 docs/spec/ledger-vectors.py --probes ~/src/merkle/testdata
"""

import base64
import hashlib
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))

# transparency-dev/merkle at this commit (testonly/constants.go).
TDM_COMMIT = "c39fbf350a405f68571bf4c42a6a518eb0a46a4b"

# The RFC 6962 test leaves (LeafInputs()).
LEAVES = [
    bytes.fromhex(h)
    for h in [
        "",
        "00",
        "10",
        "2021",
        "3031",
        "40414243",
        "5051525354555657",
        "606162636465666768696a6b6c6d6e6f",
    ]
]

# RootHashes(): the published MTH for sizes 0..8.
PUBLISHED_ROOTS = [
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d",
    "fac54203e7cc696cf0dfcb42c92a1d9dbaf70ad9e621f4bd8d98662f00e3c125",
    "aeb6bcfe274b70a14fb067a5e5578264db0fa9b51af5e0ba159158f329e06e77",
    "d37ee418976dd95753c1c73862b9398fa2a2cf9b4ff0fdfe8b30cd95209614b7",
    "4e3bbb1f7b478dcfe71fb631631519a3bca12c9aefca1612bfce4c13a86264d4",
    "76e67dadbcdf1e10e1b74ddc608abd2f98dfb16fbce75277b5232a127f2087ef",
    "ddb89be403809e325750d3d263cd78929c2942b7942a34b77e122c9594a74c8c",
    "5dc9da79a70659a9ad559cb701ded9a2ab9d823aad2f4960cfe370eff4604328",
]


def sha256(b):
    return hashlib.sha256(b).digest()


def k_of(n):
    """The largest power of two smaller than n (n > 1)."""
    k = 1
    while k * 2 < n:
        k *= 2
    return k


# RFC 9162 §2.1.1
def mth(d):
    n = len(d)
    if n == 0:
        return sha256(b"")
    if n == 1:
        return sha256(b"\x00" + d[0])
    k = k_of(n)
    return sha256(b"\x01" + mth(d[:k]) + mth(d[k:]))


# RFC 9162 §2.1.3.1
def path(m, d):
    n = len(d)
    if n == 1:
        return []
    k = k_of(n)
    if m < k:
        return path(m, d[:k]) + [mth(d[k:])]
    return path(m - k, d[k:]) + [mth(d[:k])]


# RFC 9162 §2.1.4.1
def proof(m, d):
    return subproof(m, d, True)


def subproof(m, d, b):
    n = len(d)
    if m == n:
        return [] if b else [mth(d)]
    k = k_of(n)
    if m <= k:
        return subproof(m, d[:k], b) + [mth(d[k:])]
    return subproof(m - k, d[k:], False) + [mth(d[:k])]


def generated():
    roots = [mth(LEAVES[:n]) for n in range(len(LEAVES) + 1)]
    got = [r.hex() for r in roots]
    assert got == PUBLISHED_ROOTS, "MTH disagrees with transparency-dev's RootHashes"

    inclusion = []
    for n in range(1, len(LEAVES) + 1):
        for i in range(n):
            inclusion.append(
                {
                    "index": i,
                    "size": n,
                    "leaf_hash": sha256(b"\x00" + LEAVES[i]).hex(),
                    "path": [h.hex() for h in path(i, LEAVES[:n])],
                    "root": roots[n].hex(),
                }
            )

    consistency = []
    for n in range(2, len(LEAVES) + 1):
        for m in range(1, n):
            consistency.append(
                {
                    "first": m,
                    "second": n,
                    "path": [h.hex() for h in proof(m, LEAVES[:n])],
                    "first_root": roots[m].hex(),
                    "second_root": roots[n].hex(),
                }
            )

    return {
        "source": "docs/spec/ledger-vectors.py: RFC 9162 §2.1.1, §2.1.3.1, §2.1.4.1 "
        "over the RFC 6962 test leaves; roots checked against "
        f"transparency-dev/merkle@{TDM_COMMIT} testonly/constants.go",
        "leaves": [l.hex() for l in LEAVES],
        "roots": got,
        "inclusion": inclusion,
        "consistency": consistency,
    }


def probes(testdata):
    out = {
        "source": f"https://github.com/transparency-dev/merkle/tree/{TDM_COMMIT}/testdata",
        "license": "Apache-2.0 (transparency-dev/merkle)",
        "note": "Bundled unchanged by docs/spec/ledger-vectors.py --probes. "
        "Values are base64; a value that does not decode to 32 bytes is part of the probe.",
    }
    for kind in ["inclusion", "consistency"]:
        found = []
        root = os.path.join(testdata, kind)
        for dirpath, _, files in sorted(os.walk(root)):
            for f in sorted(files):
                if f.endswith(".json"):
                    p = os.path.join(dirpath, f)
                    with open(p) as fh:
                        probe = json.load(fh)
                    probe["file"] = os.path.relpath(p, testdata)
                    found.append(probe)
        found.sort(key=lambda p: p["file"])
        out[kind] = found
    return out


def write(name, value):
    with open(os.path.join(HERE, name), "w") as fh:
        json.dump(value, fh, indent=1, ensure_ascii=False)
        fh.write("\n")


if __name__ == "__main__":
    write("ledger-vectors.json", generated())
    if len(sys.argv) == 3 and sys.argv[1] == "--probes":
        write("ledger-probes.json", probes(sys.argv[2]))
