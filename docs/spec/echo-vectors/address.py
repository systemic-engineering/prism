"""prism's own address-law vectors for echo (prism-020 P2), from the spec text.

Written from mirror `docs/specs/mirror-fixpoint-parser.md` §5.0, §5.0.1
items 1, 2, 6, 10, 11, §4.1 (fact store bytes), §15.2 (the altitude of
grammar facts) and §16.10 item 1 (`bytes`, the F_G file, `set_addr`), and
from nothing else: not from mirror's engine, not from the tune, not from
echo. It hashes with Python's `blake3` package (default, unkeyed mode, as
§5.0 says) and, when `b3sum` is on PATH, hashes every preimage a second time.

    uv run --no-project --python 3.13 --with blake3==1.0.8 \
      python docs/spec/echo-vectors/address.py

It writes `address.vectors` next to itself: one record per blank-line
separated block, in the key/value shape of mirror's `.vector` files, so
one reader in echo's tests serves both. It also recomputes addr(F_G) from
`mirror/mirror.facts` with its own small facts reader and checks it against
that file's `# grammar` header.
"""

import os
import re
import shutil
import subprocess

import blake3

HERE = os.path.dirname(os.path.abspath(__file__))


def H(b):
    """§5.0: BLAKE3, default (unkeyed) mode, 32-byte output."""
    return blake3.blake3(b).digest()


def u64(n):
    """§5.0: 8 bytes, unsigned, big-endian."""
    assert 0 <= n < 2**64
    return n.to_bytes(8, "big")


def enc(b):
    """§5.0: u64(len(b)) ‖ b."""
    return u64(len(b)) + b


def tau(name, grammar=None):
    """§5.0: UTF-8 of the altitude @-ref, 0x00, then 0x00 (root) or
    0x01 ‖ 32 raw bytes of addr(format grammar)."""
    head = name.encode("utf-8") + b"\x00"
    return head + (b"\x00" if grammar is None else b"\x01" + grammar)


# Arguments: §5.0.1 item 11, every argument is tag ‖ enc(bytes);
# tags sym 0x00, nat 0x01, addr 0x02 (§4.1), bytes 0x03 (§16.10 item 1).
def sym(s):
    return ("sym", s)


def nat(n):
    return ("nat", n)


def addr(a):
    return ("addr", a)


def byt(b):
    return ("bytes", b)


def arg_bytes(v):
    kind, x = v
    if kind == "sym":
        return b"\x00" + enc(x.encode("utf-8"))
    if kind == "nat":
        return b"\x01" + enc(u64(x))
    if kind == "addr":
        assert len(x) == 32
        return b"\x02" + enc(x)
    if kind == "bytes":
        return b"\x03" + enc(x)
    raise ValueError(kind)


def content(pred, args):
    """§5.0.1 item 10: enc(p) ‖ u64(k) ‖ arg₀ ‖ …"""
    return enc(pred.encode("utf-8")) + u64(len(args)) + b"".join(map(arg_bytes, args))


def atom_preimage(t, pred, args):
    """§5.0: H("splinter\\0" ‖ τ ‖ enc(content))."""
    return b"splinter\x00" + t + enc(content(pred, args))


def fragment_preimage(t, label, children, kind=b"fragment\x00"):
    """§5.0: H("fragment\\0" ‖ τ ‖ enc(ℓ) ‖ u64(k) ‖ addr(x₀) ‖ …)."""
    return kind + t + enc(label.encode("utf-8")) + u64(len(children)) + b"".join(children)


def set_children(children):
    """§5.0.1 items 1, 2: ascending unsigned byte-wise; strictly, so a
    repeated member is a caller bug, never deduplicated."""
    s = sorted(children)
    for x, y in zip(s, s[1:]):
        if x == y:
            raise ValueError("repeated member in a sorted set")
    return s


# The canonical text (§5.0.1 item 6, §16.10 item 1).
def text(v):
    kind, x = v
    if kind == "sym":
        return 'sym "' + x.replace("\\", "\\\\").replace('"', '\\"') + '"'
    if kind == "nat":
        return "nat %d" % x
    if kind == "addr":
        return "addr " + x.hex()
    return 'bytes "' + x.hex() + '"'


def atom_text(pred, args):
    return pred if not args else pred + "(" + ", ".join(map(text, args)) + ")"


def tau_text(name, grammar):
    return name + (" root" if grammar is None else " grammar " + grammar.hex())


# --- a small facts reader of my own, for mirror.facts (§16.10 item 1) ------

ARG = re.compile(
    r'addr (?P<addr>[0-9a-f]{64})|nat (?P<nat>0|[1-9][0-9]*)|bytes "(?P<bytes>(?:[0-9a-f]{2})*)"'
    r'|sym "(?P<sym>(?:[^"\\]|\\["\\])*)"|(?P<name>[a-z][a-z0-9_]*)'
)


def read_facts(path):
    headers, facts = [], []
    with open(path, "rb") as f:
        for raw in f.read().decode("utf-8").split("\n"):
            if raw == "":
                continue
            if raw.startswith("# "):
                k, v = raw[2:].split(" ", 1)
                headers.append((k, v))
                continue
            m = re.fullmatch(r"fact ([a-z][a-z0-9_]*)(?:\((.*)\))?", raw)
            assert m, raw
            pred, rest, args = m.group(1), m.group(2), []
            while rest:
                a = ARG.match(rest)
                assert a, rest
                if a.group("addr") is not None:
                    args.append(addr(bytes.fromhex(a.group("addr"))))
                elif a.group("nat") is not None:
                    args.append(nat(int(a.group("nat"))))
                elif a.group("bytes") is not None:
                    args.append(byt(bytes.fromhex(a.group("bytes"))))
                elif a.group("sym") is not None:
                    s = a.group("sym").replace('\\"', '"').replace("\\\\", "\\")
                    args.append(sym(s))
                else:
                    args.append(sym(a.group("name")))
                rest = rest[a.end():]
                rest = rest[2:] if rest.startswith(", ") else rest
            facts.append((pred, args))
    return headers, facts


# --- the cases --------------------------------------------------------------

DATALOG = ("@datalog/fact", None)
ITEM = ("@parse/item", None)  # §15.2: grammar facts, root flag
JSON_G = bytes(range(32))     # a stand-in grammar address, like mirror's 12
HEXNAME = "ab" * 32           # 64 lowercase hex digits, letter-led: also a pact name

records = []
by_name = {}


def emit(rec):
    rec["length"] = str(len(rec["_pre"]))
    rec["preimage"] = rec["_pre"].hex()
    rec["blake3"] = H(rec["_pre"]).hex()
    by_name[rec["vector"]] = rec
    records.append(rec)


def atom_case(name, t, pred, args, note=None, expect="equal"):
    tb = tau(*t)
    rec = {"vector": name, "kind": "atom", "tau": tau_text(*t), "atom": atom_text(pred, args),
           "tau_hex": tb.hex(), "content": content(pred, args).hex(), "expect": expect,
           "_pre": atom_preimage(tb, pred, args)}
    if note:
        rec["note"] = note
    emit(rec)


def frag_case(name, t, label, kids, kind="fragment", sorted_set=False, note=None):
    tb = tau(*t)
    addrs = [bytes.fromhex(by_name[k]["blake3"]) for k in kids]
    if sorted_set:
        addrs = set_children(addrs)
    prefix = {"fragment": b"fragment\x00", "lens": b"lens\x00", "set": b"fragment\x00"}[kind]
    rec = {"vector": name, "kind": kind, "tau": tau_text(*t), "label": '"' + label + '"',
           "k": str(len(kids)), "children": " ".join(kids) if kids else "none",
           "tau_hex": tb.hex(), "expect": "equal",
           "_pre": fragment_preimage(tb, label, addrs, prefix)}
    if note:
        rec["note"] = note
    emit(rec)


atom_case("p01-sym", DATALOG, "p", [sym("a")])
atom_case("p02-bare-pred", DATALOG, "start", [], note="k = 0: content is enc(p) ‖ u64(0)")
atom_case("p03-nat-edges", DATALOG, "n", [nat(0), nat(1), nat(2**64 - 1)])
atom_case("p04-addr", ITEM, "node", [addr(bytes.fromhex(HEXNAME)), sym("seq")])
atom_case("p05-sym-same-hex", ITEM, "node", [sym(HEXNAME), sym("seq")],
          note="p04's addr as a sym with the same 64 hex digits (§5.0.1 item 11 tags 0x02 vs 0x00): a different address")
atom_case("p06-bytes", ITEM, "term_lit", [addr(bytes.fromhex(HEXNAME)), byt(b"\x2f")])
atom_case("p07-bytes-empty", ITEM, "term_lit", [addr(bytes.fromhex(HEXNAME)), byt(b"")])
atom_case("p08-bytes-same-hex", ITEM, "term_lit", [addr(bytes.fromhex(HEXNAME)), sym("2f")],
          note="p06's bytes \"2f\" as sym \"2f\" (tags 0x03 vs 0x00): a different address")
atom_case("p09-utf8-escapes", DATALOG, "s", [sym("λ"), sym('q"b\\s'), sym("")])
atom_case("p10-grammar-tau", ("@data/json", JSON_G), "p", [sym("a")],
          note="τ = name 0x00 0x01 ‖ 32 bytes (§5.0), the stand-in grammar 00 01 … 1f")
atom_case("p11-root-tau", ("@data/json", None), "p", [sym("a")],
          note="p10 with the root flag: a different address")
atom_case("p12-nat-vs-sym", DATALOG, "q", [nat(7)])
atom_case("p13-sym-seven", DATALOG, "q", [sym("7")], note="p12's nat 7 as sym \"7\": a different address")

frag_case("p20-fragment-k0", ("@mirror", None), "shard", [])
frag_case("p21-fragment-k3", ("@mirror", None), "shard", ["p01-sym", "p03-nat-edges", "p12-nat-vs-sym"])
frag_case("p22-fragment-k3-reversed", ("@mirror", None), "shard", ["p12-nat-vs-sym", "p03-nat-edges", "p01-sym"])
frag_case("p23-lens", ("@mirror", None), "source", ["p21-fragment-k3", "p20-fragment-k0"], kind="lens")
frag_case("p24-set-grammar", ITEM, "grammar", ["p04-addr", "p06-bytes", "p07-bytes-empty"], kind="set", sorted_set=True,
          note="set_addr (§16.10 item 1): children strictly ascending by raw address bytes, whatever order they are listed in")

# F_G itself: every fact of mirror.facts at @parse/item root, set_addr ℓ = grammar.
headers, facts = read_facts(os.path.join(HERE, "mirror", "mirror.facts"))
t_item = tau(*ITEM)
fg_children = set_children([H(atom_preimage(t_item, p, a)) for p, a in facts])
fg_pre = fragment_preimage(t_item, "grammar", fg_children)
claimed = dict(headers)["grammar"]
assert H(fg_pre).hex() == claimed, (H(fg_pre).hex(), claimed)
listed = [H(atom_preimage(t_item, p, a)) for p, a in facts]
assert listed == fg_children, "mirror.facts is not listed in strictly ascending address order"
records.append({"vector": "p30-mirror-facts-grammar", "kind": "set (F_G)",
                "tau": tau_text(*ITEM), "label": '"grammar"', "k": str(len(facts)),
                "children": "every fact of mirror/mirror.facts, as atoms at @parse/item root",
                "expect": "equal", "length": str(len(fg_pre)), "preimage": fg_pre.hex(),
                "blake3": H(fg_pre).hex()})

# A second implementation, when one is on PATH.
b3sum = shutil.which("b3sum")
if b3sum:
    for r in records:
        out = subprocess.run([b3sum, "--no-names"], input=bytes.fromhex(r["preimage"]),
                             capture_output=True, check=True).stdout.decode().strip()
        assert out == r["blake3"], r["vector"]

ORDER = ["vector", "kind", "note", "tau", "atom", "label", "k", "children", "tau_hex",
         "content", "expect", "length", "preimage", "blake3"]
with open(os.path.join(HERE, "address.vectors"), "w") as f:
    f.write("# echo's own address-law vectors: written by address.py from mirror's fixpoint §5.0 text.\n")
    f.write("# blake3: Python blake3 %s%s.\n" % (
        blake3.__version__, ", and b3sum agreed on every preimage" if b3sum else "; b3sum was not on PATH"))
    for r in records:
        f.write("\n")
        for k in ORDER:
            if k in r:
                f.write("%-10s %s\n" % (k, r[k]))
print("%d vectors; addr(F_G) = %s (%d facts)%s" % (
    len(records), claimed, len(facts), "; b3sum agreed" if b3sum else ""))
