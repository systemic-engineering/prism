# S3*: what a sampled audit of the verdict cache can say

*prism, 2026-11-17. prism-020 ask 4 (prism-Reed's Response §4). The maths is exact, with no floats: `prismqueer/src/audit.rs`, checked by `prismqueer/tests/audit.rs`. The rate and confidence a gate accepts are a ruler's (`ruler.md`), signed. Whether a sample meets them is measured.*

## The point, first

**S3\* bounds a drift *rate*. It doesn't catch a single wrong item.**

At a 5 % sample, one wrong item among 1000 is caught with probability 5 %. It is missed 95 % of the time. What covers an individual change is **Mara's rule**:

- every changed item is recomputed (100 %);
- the pin is recomputed on every run.

S3\* audits the **unchanged** mass, the verdicts nobody touched. What it can establish is a statement about the whole cache: "fewer than p of these are wrong, at confidence c". It can't vouch for any single item.

Beer's S3\* is the sporadic audit: the control function doesn't only trust what it's told; now and then it looks for itself. Here the "looking" is Mara's sampled recompute.

## Setting

- N unchanged items in the cache, each with a cached verdict.
- k of them are wrong (drifted): recomputing gives a different verdict. k is unknown.
- The audit recomputes n of the N, chosen **uniformly, without replacement**.

## 1. Finding drift that exists

If k items are wrong, the chance the sample misses all of them is

```
P(miss) = C(N−k, n) / C(N, n) = ∏ᵢ₌₀ⁿ⁻¹ (N−k−i) / (N−i)
```

It is an exact rational. It is 0 when n > N − k, since the sample can't avoid every bad item. The code is `miss_probability(N, k, n)`. It refuses n > N and k > N.

Hand checks (`tests/audit.rs`):

| N | k | n | P(miss) |
|---|---|---|---|
| 10 | 1 | 1 | 9/10 |
| 10 | 2 | 3 | C(8,3)/C(10,3) = 56/120 = **7/15** |
| 1000 | 1 | 50 | 950/1000 = **19/20** |

Sampling *with* replacement would give (8/10)³ = 64/125 for the second row, not 7/15. The test tells the two apart.

## 2. What a clean sample proves

Say all n recomputes come back clean. The claim "the drift rate k/N is below p, at confidence c" holds iff

```
(1 − p)ⁿ ≤ 1 − c
```

**Why.** Suppose the rate were p or more. Then each factor (N−k−i)/(N−i) is at most (N−k)/N ≤ 1 − p, so a clean sample would have probability at most (1 − p)ⁿ. Without replacement is never likelier than with. When that is at most 1 − c, a clean sample rejects "rate ≥ p" at confidence c.

- `bounds_rate(n, p, c)` decides the inequality exactly: with 1 − p = a/b and 1 − c = m/d, it compares aⁿ·d ≤ m·bⁿ in big integers.
- `smallest_sample(p, c)` gives the least n that satisfies it. (1 − p)ⁿ falls strictly as n grows, so it doubles until the inequality holds, then bisects.
- Both refuse p or c outside the open interval (0, 1).

The usual **rule of three** (p ≈ 3/n at 95 %) is the float approximation of this, since ln 20 ≈ 3. The exact neighbourhood:

| n | (19/20)ⁿ | ≤ 1/20? |
|---|---|---|
| 58 | ≈ 0.05105 | no |
| 59 | ≈ 0.04849 | **yes** |

So `smallest_sample(1/20, 19/20) = 59`, where the rule of three says 60.

### The worked example

5 % of 1000 items is **50 samples, all clean**. The bound is 1 − (1/20)^(1/50) ≈ 0.058155. Rounded, "the rate is below about 5.8 % at 95 %". Exactly:

| p | (1 − p)⁵⁰ | bounded at 95 %? |
|---|---|---|
| 581/10000 | ≈ 0.050146 | no |
| 582/10000 | ≈ 0.049881 | **yes** |
| 58/1000 | ≈ 0.050413 | no |
| 59/1000 | ≈ 0.047806 | **yes** |

Both sides of the boundary are tested as exact comparisons. Note that "below 5.8 %" itself is *not* established by 50 clean samples; "below 5.82 %" is.

## 3. The gate's verdict over a sample

`audit(N, n, drifted, p, c) -> Imperfect<(), Finding, Sampled>`:

| Situation | Verdict |
|---|---|
| a recompute disagreed (`drifted > 0`) | `Failure(Drift { found, sampled })`: a located wrong item is red |
| clean, n = N | `Success(())`: only a full recompute (or a proved law) is green |
| clean, n < N | `Partial((), bound)`: **never `Success`**. The loss names the `Bound { population, sampled, rate, confidence, established }` |
| n > N, drifted > n, or p, c outside (0, 1) | `Failure(Refused(…))` with total loss: refused, not judged |

`established` says whether the sample meets the claim. A clean sample too small for its claim is still `Partial`, and says it didn't establish it.

`Sampled`, the loss, is the set of bounds an audit stands on. Composing two audits unions their bounds; nothing is averaged (the same line as `gate.md`'s "compose, don't average"). An audit that couldn't be made is the absorbing total.

In the gate's exit mapping (prism-020 Response §2, item 7), a clean sample is `Partial` (exit 1), never 0. **The bound is a measurement, while the p and c to accept are a signed ruler.** That is the measured-vs-judged line, the same as `ruler.md`'s split between a ruler and its calibration.

## What this doesn't do

- **Choose p and c.** That's a judgment, and it gets signed.
- **Choose which items to sample.** Uniformity is assumed. A sampler that isn't uniform voids the bound, and checking the sampler is a separate audit.
- **Cover changed items.** Mara's rule does that, at 100 %.
- **Estimate the rate from a sample with drift in it.** A found drift is a `Failure` and goes to a human (the algedonic bypass). It isn't a number to average away.
