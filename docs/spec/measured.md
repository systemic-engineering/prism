# measured: a precision matrix estimated from integer counts, exactly

*prism, 2026-11-23 (prism-023 §1; bullet-up-020 §1–§2, metrix v0). Decided as D6 in `docs/spec/metrix.md`: "an estimate is `Measured<Metrix>` until a window ruler signs it". Names are provisional; Alex names them. Code: `prismqueer::measured` (beside `metrix`, behind `lapack` only because `room` is; it calls no LAPACK). Tests: `prismqueer/tests/measured.rs`.*

## What it is

A `Measured` is a matrix **estimated** from stream windows: the precision matrix of `d` streams, fitted from `n` samples of integer counts under a declared support. It carries what it was measured from and how, and it is **not** a `Metrix`. It becomes one only through a certificate and a seal.

```
measure(window: &Ruler, frame: &Frame, counts: &[Vec<Z>] /* n samples × d streams */, method: Method)
    -> Result<Measured, Refused>

Method  { estimator: Mle /* divide by n */ | Unbiased /* divide by n − 1 */, support: Support }
Support   the declared edges between streams, unordered pairs {i, j}, i ≠ j; the diagonal is implied
Refused { WrongShape(Shape), Unresolved(Support), TooFewSamples { samples, needed },
          NotInvertible { clique }, OtherWindow { expected, found }, Draft(MetrixError) }
```

`Measured` holds: the frame, the window ruler's address, the method, the sample count `n`, the samples' address, and the estimate K̂ (d×d over `room::Q`). It holds **no signer**.

## Exact, with no float

Stream values on the certified path are integer counts (bullet-up-020 §2, rule 7). Their sample mean and sample covariance are rationals, and so is every inverse the fit takes. **No float and no LAPACK appear on this path**, and no float decides anything: equality, singularity and chordality are all exact. "Machine" drops out of the method, because the arithmetic is exact. A source scan in the tests holds the module to it.

- **Mean:** x̄ⱼ = (Σₛ xₛⱼ) / n.
- **Covariance:** Sᵢⱼ = Σₛ (xₛᵢ − x̄ᵢ)(xₛⱼ − x̄ⱼ) / m, with m = n (`Mle`) or m = n − 1 (`Unbiased`).

## The fit: covariance selection, never a mask

The declared edges are the **support of the precision matrix** (bullet-up-020 rule 4, prism-023 §3). A zero in K̂ off the support is declared conditional independence (D6), and K̂ is the covariance-selection estimate (Dempster 1972): the K that is zero off the support and whose inverse agrees with S on the support and the diagonal. It is **not** S⁻¹ with entries zeroed: a masked inverse is in general not the fit, and not even positive definite.

- **Full support** (every pair declared): K̂ = S⁻¹, exactly.
- **Chordal support:** the closed form (Lauritzen 1996, §5.3, Prop. 5.9; Dempster 1972)

  ```
  K̂ = Σ_cliques C [S_C⁻¹]⁰ − Σ_separators T [S_T⁻¹]⁰
  ```

  where S_C is S restricted to the streams of C, [·]⁰ zero-pads back to d×d, the cliques are the maximal cliques of the support graph, and the separators are those of a clique ordering with the running intersection property, counted with multiplicity (an empty separator contributes nothing: two components are independent). Full support is the one-clique case, so one formula covers both.
- **Non-chordal support** (any chordless cycle of length ≥ 4, e.g. the four-cycle): there is no closed form, and the estimate needs iterative proportional scaling, which is not exact over ℚ. It is **refused**, `Unresolved(support)`. Never iterated, never approximated, never masked.

The `Mle` estimator gives the maximum-likelihood estimate of the Gaussian graphical model. `Unbiased` applies the same fit to the n − 1 covariance; since every term scales as 1/m, K̂_Unbiased = ((n − 1)/n)·K̂_Mle. It is a declared method, not a second MLE.

## The chordality check

Exact, by **maximum cardinality search** (Tarjan & Yannakakis 1984):

1. Visit the streams one at a time; each step takes the unvisited stream with the most visited neighbours, ties broken by the lowest index (so the order is a function of the support alone).
2. **Perfect elimination check:** the graph is chordal iff, for every stream, its neighbours visited before it are pairwise adjacent (the reverse visit order is a perfect elimination ordering). The first failure is `Unresolved`.
3. **Cliques:** each stream v with earlier neighbours P(v) gives the candidate {v} ∪ P(v). The maximal cliques are the candidates not strictly inside another, in order of discovery (Blair & Peyton 1993), and that order has the running intersection property.
4. **Separators:** for the k-th clique (k ≥ 2), Tₖ = Cₖ ∩ (C₁ ∪ … ∪ Cₖ₋₁).

`decompose(d, support)` exposes the visit order, the cliques and the separators, so the check is testable on its own, and `covariance(frame, counts, estimator)` exposes S (refusing n = 0, and n = 1 for `Unbiased`, as `TooFewSamples`), so a singular S can be handed to `MetrixDraft` directly.

## Refusals

All are typed values. None panics, and none is "fixed".

| Refusal | When | Why |
|---|---|---|
| `WrongShape(Shape::Sample { index, len, streams })` | a sample's length isn't the frame's dimension d | the columns are the frame's basis |
| `WrongShape(Shape::Edge { from, to })` | an edge names a stream ≥ d, or a stream and itself | the diagonal is implied, never declared |
| `Unresolved(support)` | the support isn't chordal | no exact closed form; iteration is not exact |
| `TooFewSamples { samples, needed }` | n ≤ \|C\| for the largest clique C (n ≤ d for full support): `needed` is \|C\| + 1 | the centred sample has rank ≤ n − 1, so S_C is singular, for either estimator |
| `NotInvertible { clique }` | some S_C is singular (collinear streams) | the MLE exists iff every S_C is positive definite; never pseudo-inverted |

Checked in that order: shape, then support, then sample count, then each clique in discovery order. A separator's S_T is a principal block of a clique's positive definite S_C, so it is invertible whenever the cliques are.

## The address: the signer beside it, never in it

Under the address discipline of `docs/spec/ruler.md` and `identity.md`, with `rational` and `bytes` as `ruler.md` encodes them and integers as rationals of denominator 1:

```
samples_oid  = BLAKE3-keyed(K_public, "samples\0" ‖ u64le(n) ‖ u64le(d) ‖ rational(x₀₀) ‖ … ‖ rational(xₙ₋₁,d₋₁))    row by row
method       = estimator ‖ u64le(m) ‖ u64le(i₁) ‖ u64le(j₁) ‖ … ‖ u64le(iₘ) ‖ u64le(jₘ)
               estimator: 01 Mle | 02 Unbiased; m edges, each i < j, ascending, deduplicated
measured_oid = BLAKE3-keyed(K_public, "measured\0" ‖ bytes(window_oid) ‖ bytes(frame_oid) ‖ method ‖ bytes(samples_oid))
```

- One set of inputs has one address. Another window, frame, estimator, support or sample is another address.
- The estimate isn't in the preimage: it is a function of it.
- **No signer is in it.** `Measured` has none to put there.
- **The frame is in it** (a departure from prism-023 §1, which lists window ‖ method ‖ samples). The columns are the frame's basis, and `metrix.md` already requires a matrix's address to name its basis (Mara §8.1). One sample read in two frames is two measurements.

## From `Measured` to `Metrix`

`Measured` is not a `Metrix` (a `compile_fail` doctest, with a control). The road is the one `metrix.md` already gives every metric:

```
measured.draft()                 = MetrixDraft::new(frame, estimate)   LDLᵀ certificate, or NotDefinite at the failing pivot
MetrixDraft::seal(by)            → Metrix                              signed by `by`, beside the metrix address
measured.seal(window: &Ruler)    → Result<Metrix, Refused>             the typed road (below)
```

- **The window ruler signs.** D6 says an estimate is measured "until a window ruler signs it". `Measured::seal(window)` enforces that with a typed check: the ruler's address must be the window the measurement was taken under (`OtherWindow { expected, found }` otherwise), and the `Metrix` is sealed by that ruler's signer (`window.signer().by()`).
- **The open road stays open.** `measured.draft()?.seal(anyone)` is still allowed: `MetrixDraft::seal` takes any signer, by `metrix.md`'s design. The sealed `Metrix` names who sealed it, and its address is the same either way (the signer is beside it). Closing the open road would need the draft to carry its provenance; that is Alex's call, not taken here.
- **Never clipped.** A singular covariance handed to `MetrixDraft` directly is refused at its pivot (`NotDefinite`), never moved into the cone. `measure` refuses it earlier, as `NotInvertible`, and a fit that passes every refusal is positive definite (each [S_C⁻¹]⁰ − … sum is, for chordal support with every S_C definite; Lauritzen 1996, Prop. 5.9), so its draft always certifies.
- The `Metrix` address is `metrix.md`'s (frame ‖ G). It does not name the window or the method; the `Measured` does, at its own address.

## Not here

- **Δstock = in-flow − out-flow** (prism-023 §3, rule 5): a law between stock and flow streams. It is bullet-up's, on the stream side, not a property of the fit.
- **Non-chordal support** (above): refused, not approximated. Whether that refusal is the law is Mara's grade (prism-023 §1).
- **Floats.** An off-path reading in floats would be a different type, naming machine and method (`band::Measurement`); none is here.

## References

- A. P. Dempster, *Covariance selection*, Biometrics 28 (1972), 157–175.
- S. L. Lauritzen, *Graphical Models*, Oxford 1996, §5.3 (Prop. 5.9: the MLE in a decomposable model).
- R. E. Tarjan and M. Yannakakis, *Simple linear-time algorithms to test chordality of graphs, test acyclicity of hypergraphs, and selectively reduce acyclic hypergraphs*, SIAM J. Comput. 13 (1984), 566–579.
- J. R. S. Blair and B. Peyton, *An introduction to chordal graphs and clique trees*, in *Graph Theory and Sparse Matrix Computation*, IMA Vol. 56, Springer 1993, 1–29.
