"""Reference encoder for docs/spec/ruler.md, written from the spec, not from prism.

Writes ruler-vectors.json: each case's preimage is built here, byte by byte,
and hashed with Python's `blake3` (keyed with K_public). Every address is then
recomputed with the b3sum CLI over the same preimage bytes.

    nix shell --impure --expr 'with import (builtins.getFlake "nixpkgs") {};
      [ b3sum (python3.withPackages (ps: [ ps.blake3 ])) ]' \
      -c python3 docs/spec/ruler-vectors.py
"""

import json
import os
import subprocess
import tempfile
from fractions import Fraction

import blake3

CONTEXT = "systemic.engineering://visibility/public/v1"
K_PUBLIC = blake3.blake3(b"", derive_key_context=CONTEXT).digest()
assert K_PUBLIC.hex() == "18d4e1ec48377a474bd2709f1a5705ae2c95c11e00b1b490cf1af92a86258ba0"


def u64le(n):
    return n.to_bytes(8, "little")


def field(b):
    return u64le(len(b)) + b


def magnitude(n):
    """Minimal little-endian bytes of n >= 0; zero is the single byte 00."""
    if n == 0:
        return b"\x00"
    return n.to_bytes((n.bit_length() + 7) // 8, "little")


def rational(text):
    r = Fraction(text)  # reduced, denominator > 0
    sign = 0 if r == 0 else (1 if r > 0 else 2)
    return bytes([sign]) + field(magnitude(abs(r.numerator))) + field(magnitude(r.denominator))


EDGE = {"open": 1, "closed": 2, "unbounded": 3}
QUESTION = {"any": 1, "max": 2, "product": 3, "sum": 4}


def edge(e):
    out = bytes([EDGE[e["edge"]]])
    if e["edge"] != "unbounded":
        out += rational(e["value"])
    return out


def region(r):
    return edge(r["lo"]) + edge(r["hi"])


def ruler(case):
    out = b"ruler\x00"
    out += region(case["region"])
    out += bytes([QUESTION[case["question"]]])
    out += rational(case["alpha"]) + rational(case["beta"])
    c = case["clock"]
    out += u64le(c["reaction"]) + u64le(c["importance"]) + u64le(c["delay"])
    return out


def calibration(case):
    out = b"calibration\x00"
    out += field(case["instrument"].encode("utf-8"))
    out += region(case["region"])
    out += rational(case["p0"]) + rational(case["p1"])
    out += field(case["method"].encode("utf-8"))
    return out


def independence(case):
    names = sorted(set(i.encode("utf-8") for i in case["instruments"]))
    assert len(names) == len(case["instruments"]) >= 2, case["name"]
    out = b"independence\x00" + u64le(len(names))
    for n in names:
        out += field(n)
    out += region(case["region"])
    return out


def b3sum(preimage):
    with tempfile.NamedTemporaryFile(delete=False) as f:
        f.write(preimage)
    try:
        out = subprocess.run(
            ["b3sum", "--keyed", "--no-names", f.name],
            input=K_PUBLIC,
            capture_output=True,
            check=True,
        )
    finally:
        os.unlink(f.name)
    return out.stdout.decode().strip()


def seal(case, encode):
    pre = encode(case)
    addr = blake3.blake3(pre, key=K_PUBLIC).hexdigest()
    assert addr == b3sum(pre), case["name"]
    return {**case, "preimage_hex": pre.hex(), "address": addr}


def e(kind, value=None):
    return {"edge": kind} if value is None else {"edge": kind, "value": value}


RULERS = [
    {
        "name": "above 1, max, alpha = beta = 1/20, clock (10, 1, 100)",
        "region": {"lo": e("open", "1"), "hi": e("unbounded")},
        "question": "max",
        "alpha": "1/20",
        "beta": "1/20",
        "clock": {"reaction": 10, "importance": 1, "delay": 100},
    },
    {
        "name": "closed [-3/2, 7/4], sum, negative edge, zero clock",
        "region": {"lo": e("closed", "-3/2"), "hi": e("closed", "7/4")},
        "question": "sum",
        "alpha": "1/10",
        "beta": "1/5",
        "clock": {"reaction": 0, "importance": 0, "delay": 0},
    },
    {
        "name": "(-inf, 0), any, zero edge, delay u64 max",
        "region": {"lo": e("unbounded"), "hi": e("open", "0")},
        "question": "any",
        "alpha": "1/100",
        "beta": "1/50",
        "clock": {"reaction": 1, "importance": 2, "delay": 18446744073709551615},
    },
    {
        "name": "[n, inf) with a numerator wider than 64 bits, product",
        "region": {"lo": e("closed", "12345678901234567890123/1000"), "hi": e("unbounded")},
        "question": "product",
        "alpha": "1/3",
        "beta": "1/3",
        "clock": {"reaction": 256, "importance": 65536, "delay": 4294967296},
    },
]

ABOVE_1 = {"lo": e("open", "1"), "hi": e("unbounded")}

CALIBRATIONS = [
    {
        "name": "knee for (1, inf), p0 = 1/6, p1 = 1/2",
        "instrument": "knee",
        "region": ABOVE_1,
        "p0": "1/6",
        "p1": "1/2",
        "method": "600 labelled contexts",
    },
    {
        "name": "knee recalibrated for (1, inf), p0 = 1/10, p1 = 3/5",
        "instrument": "knee",
        "region": ABOVE_1,
        "p0": "1/10",
        "p1": "3/5",
        "method": "600 labelled contexts",
    },
    {
        "name": "knee for (3, inf), the first case's rates in another region",
        "instrument": "knee",
        "region": {"lo": e("open", "3"), "hi": e("unbounded")},
        "p0": "1/6",
        "p1": "1/2",
        "method": "600 labelled contexts",
    },
    {
        "name": "multi-byte method, empty instrument, (-inf, 0]",
        "instrument": "",
        "region": {"lo": e("unbounded"), "hi": e("closed", "0")},
        "p0": "1/1000",
        "p1": "999/1000",
        "method": "Grüße, 世界 \U0001f308",
    },
]

INDEPENDENCES = [
    {
        "name": "knee and ear for (1, inf), given out of order",
        "instruments": ["knee", "ear"],
        "region": ABOVE_1,
    },
    {
        "name": "three instruments, one multi-byte, for [-3/2, 7/4]",
        "instruments": ["hand", "ärä", "ear"],
        "region": {"lo": e("closed", "-3/2"), "hi": e("closed", "7/4")},
    },
]

out = {
    "spec": "docs/spec/ruler.md",
    "source": "preimages built by docs/spec/ruler-vectors.py from the spec (not prism), "
    "hashed with Python's blake3 package and recomputed with the b3sum CLI 1.8.7 "
    "(b3sum --keyed, key K_public); checked against prism by prismqueer/tests/ruler_vectors.rs",
    "function": "address = BLAKE3-keyed(K_public, preimage), 32 bytes, lowercase hex",
    "k_public": K_PUBLIC.hex(),
    "rulers": [seal(c, ruler) for c in RULERS],
    "calibrations": [seal(c, calibration) for c in CALIBRATIONS],
    "independences": [seal(c, independence) for c in INDEPENDENCES],
}

here = os.path.dirname(os.path.abspath(__file__))
with open(os.path.join(here, "ruler-vectors.json"), "w") as f:
    json.dump(out, f, indent=2, ensure_ascii=False)
    f.write("\n")
