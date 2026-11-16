# Ruler: the canonical ruler address, and the calibration beside it

*prism, 2026-11-16. **prism-Reed's lean; Alex has the final call** (prism-018 ask 1, prism-017 ask 5, prism-019 order item 2). The one ruler address for the whole stack, defined once, here. Test vectors: `ruler-vectors.json`, whose preimages are built by `ruler-vectors.py` from this text (not from prism), hashed with Python's `blake3` and recomputed with the `b3sum` CLI (1.8.7). `prismqueer/tests/ruler_vectors.rs` holds prism to them.*

## The law

A ruler is a signed judgment: a region, a question, the signer's error costs (α, β) and a clock. Its address is a scoped content address (`identity.md`) in the public scope:

```
ruler_oid = BLAKE3-keyed(K_public, "ruler\0" ‖ region ‖ question ‖ rational(α) ‖ rational(β) ‖ clock)
```

Two things never enter it:

- **The signer.** It sits beside the address. One judgment signed by two people has one address and two signers (`identity.md`: the observer appears through its boundary, never its name).
- **p₀ and p₁.** They are the instrument's operating characteristic, a measurement (below). A measurement never enters the address of what it measures, so recalibrating an instrument never moves a ruler.

## The encoding

Every field is self-delimiting, so the preimage decodes uniquely. All integers are little-endian.

```
u64le(n)    = n as 8 bytes, little-endian
bytes(x)    = u64le(len(x)) ‖ x
magnitude(n)= the minimal little-endian bytes of n ≥ 0; zero is the single byte 00
rational(r) = sign ‖ bytes(magnitude(|numerator|)) ‖ bytes(magnitude(denominator))
              r reduced, denominator ≥ 1; sign: 00 zero, 01 positive, 02 negative

edge        = 01 ‖ rational(a)      open:   the end a, excluded
            | 02 ‖ rational(a)      closed: the end a, included
            | 03                    unbounded
region      = edge(lo) ‖ edge(hi)
question    = 01 any | 02 max | 03 product | 04 sum              one byte
clock       = u64le(reaction) ‖ u64le(importance) ‖ u64le(delay)
```

- The region is the one a ruler accepts: non-empty, edges exact rationals. An unbounded lower edge is `−∞`, an unbounded upper edge `+∞`.
- One rational has one encoding: it is reduced, the sign is a byte of its own, and zero is `00 ‖ bytes(00) ‖ bytes(01)`.
- The question tag and the edge tags are closed sets. A new question is a new tag, never a reuse.

Example, the first vector (`(1, ∞)`, max, α = β = 1/20, clock (10, 1, 100)):

```
72756c657200                      "ruler\0"
01 01 0100000000000000 01 0100000000000000 01      lo: open 1/1
03                                hi: unbounded
02                                max
01 0100000000000000 01 0100000000000000 14         α = 1/20
01 0100000000000000 01 0100000000000000 14         β = 1/20
0a00000000000000 0100000000000000 6400000000000000 clock
```

## The calibration

p₀ is the chance that a context certifies `In` when the tension isn't real, and p₁ the chance when it is: the instrument's operating characteristic (its d′ in signal-detection terms). That is measured, not judged. It lives in its own crystal:

```
calibration_oid = BLAKE3-keyed(K_public, "calibration\0" ‖ bytes(instrument) ‖ rational(p₀) ‖ rational(p₁) ‖ bytes(method))
```

- `instrument` is the instrument's address as its UTF-8 string; `method` is how the calibration was measured, UTF-8.
- It is valid when `0 < p₀ < p₁ < 1`. The ruler's own validity is `0 < α, β < 1` and `α + β < 1`.
- Its address is its own. It is never folded into the ruler's.

**Persistence reads both.** `persist(evidence, ruler, calibration)` runs Wald's test with the ruler's α, β and the calibration's p₀, p₁, over the decisions the calibration's instrument made under the ruler. Its verdict carries the ruler's address and the calibration's side by side, so a reader knows which calibration the verdict depended on. Decisions (T1 crossings) depend on the region only and don't carry a calibration.

## mirror's `rust/ruler`

mirror's `rust/ruler` (on `reed/ship`) also hashes under `"ruler\0"`, but it is a different record under the same prefix: `BLAKE3(“ruler\0” ‖ “@mirror/ruler\0\0” ‖ u64be(len) ‖ family ‖ body ‖ reader ‖ u64be(n) ‖ unit)`, plain (unkeyed) BLAKE3 over a budget `Count`. Under this spec, `"ruler\0"` names this judgment only; mirror reads this spec and its `rust/ruler` retires. Where its budget goes is mirror's call.

## Vectors

`ruler-vectors.json` holds:

- `k_public`, as in `identity-vectors.json`;
- `rulers`: four judgments covering every edge kind, all four questions (one per case), a negative edge, a zero edge, a numerator wider than 64 bits and a delay of `2⁶⁴ − 1`; each with its `preimage_hex` and `address`;
- `calibrations`: one instrument calibrated twice (two addresses, same rulers), and a multi-byte method with an empty instrument.

Rationals are written `"n/d"` (or `"n"`), reduced. To recompute an address with the CLI:

```
printf '%s' "$PREIMAGE_HEX" | xxd -r -p > in
printf '%s' "$K_PUBLIC" | xxd -r -p | b3sum --keyed --no-names in
```
