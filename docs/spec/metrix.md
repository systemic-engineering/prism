# metrix: a frame's ruler for length, and the forms between rulers

*prism, 2026-11-17 (prism-021 D5, D6). Sources: Mara's frames note on mirror branch `mara/frames-hypothesis`, **54c88f72** (`docs/math/2026-11-17-mara-hypothesis-as-composition-of-frames.md`: §8 asks 6–7, `Metrix`, definite vs indefinite, relative eigenvalues; §9 one `Metrix`, two altitudes; §6.4 where `Metrix` enters kleinos), and its probe **c012e29f** (`frames_probe.py`, P11a–P11k). Decided by Alex as D5 and D6 (mirror STATE #77–#78). Names are provisional. Code: `prismqueer::metrix` (behind `lapack`, beside `gl` and `hypothesis`); tests: `prismqueer/tests/metrix.rs`.*

## The decisions

- **D5.** A frame's metric is **positive definite, certified (LDLᵀ over ℚ), signed, no default**. The Euclidean identity must be declared too. A separate **`Form`** of any signature holds metric differences and isometry defects. It lives in prism, as `prismqueer::metrix`. **Not over ℤ/n**: ℤ/n has no order, so "positive" means nothing. Sylvester's criterion certifies definite only. Relative eigenvalues are a complete pair invariant only over ℝ with one form definite. mirror's `rust/matrix` retires onto room + `Band`, not onto `Metrix`.
- **D6.** "Metrix is one thing" holds for the metric side: a precision matrix *is* a `Metrix`, and its zeros are declared conditional independence. It breaks for the relation side, which stands on the semiring floor. An estimate is **`Measured<Metrix>`** until a window ruler signs it.

## `Metrix`

A `Metrix` is the Gram matrix G of ⟨u, v⟩ = uᵀGv on a frame's stalk, in the frame's basis: a symmetric positive definite n×n matrix over exact ℚ.

- **Draft, certificate, seal.** `MetrixDraft::new(&Frame, rows)` checks squareness, the frame's dimension and symmetry, then runs an exact **LDLᵀ without pivoting**: dₖ = gₖₖ − Σⱼ₍ₖ lₖⱼ²dⱼ and lᵢₖ = (gᵢₖ − Σⱼ₍ₖ lᵢⱼlₖⱼdⱼ)/dₖ. G is positive definite iff every dₖ > 0 (d₁⋯dₖ is the k-th leading minor). The first dₖ ≤ 0 is the refusal, `NotDefinite { pivot: k, value: dₖ }`. The `Certificate` (L, D) is kept, and `Certificate::verifies(g)` checks it independently: L unit lower triangular, D > 0, L·D·Lᵀ = G exactly.
- **Never clipped.** An indefinite or semidefinite input is refused, never moved into the cone. Clipping eigenvalues is averaging toward the cone (Mara §8.2), and averaging is a choice someone signs.
- **Signed beside the address.** Only `MetrixDraft::seal(by)` makes a `Metrix`, minting its `Signer`. The address is

  ```
  metrix_oid = BLAKE3-keyed(K_public, "metrix\0" ‖ u64le(len) ‖ frame_oid ‖ u64le(n) ‖ rational(g₀₀) ‖ … ‖ rational(gₙ₋₁ₙ₋₁))
  ```

  with `rational` as `docs/spec/ruler.md` encodes it, row by row. The signer and the certificate (a function of G) are not in it. One judgment signed twice has one address and two signers.
- **In a named basis.** The frame's address is in the preimage (Mara §8.1: "the address must be of the matrix *in a named basis*"). One matrix on two frames is two addresses.
- **No default.** `MetrixDraft::identity(&Frame)` *declares* the Euclidean identity; it becomes a `Metrix` only when sealed. There is no `Default` (a `compile_fail` doctest, and an autoref probe in the tests).
- **Not over ℤ/n.** Draft entries are `Ordered`, a sealed trait for `Q` and `Z` (a lattice's Gram matrix, read in ℚ). `ZMod` doesn't implement it: a `compile_fail,E0277` doctest, with the ℤ control beside it.

## `Form`

A `Form` is a symmetric matrix over ℚ of any signature, unsigned (a computed value, not a judgment): isometry defects, differences of metrics (`Form::difference`), `Map::grading`'s Krein form. Its **inertia (n₊, n₋, n₀)** is data, computed by Sylvester's law through an **LDLᵀ with symmetric pivoting**:

- if some remaining diagonal entry is nonzero, it is a 1×1 pivot (a symmetric permutation brings it to the front), and its sign counts once;
- if the whole remaining diagonal is zero but some aᵢⱼ = b isn't, rows and columns i, j form the 2×2 pivot E = [[0, b], [b, 0]], with det −b² < 0, so it counts one positive and one negative;
- if everything left is zero, it is n₀.

Each step replaces the form by the Schur complement A/E, and A is congruent to E ⊕ A/E, so inertias add (Haynsworth) and Sylvester's law makes the count the form's. 1×1: a′ᵣₛ = aᵣₛ − aᵣₖaₖₛ/d. 2×2: a′ᵣₛ = aᵣₛ − (aᵣᵢaⱼₛ + aᵣⱼaᵢₛ)/b.

**Not leading minors.** Sylvester's *criterion* (leading minors > 0) certifies positive definite only (Mara §8.1). [[0, 1], [1, 0]] has leading minor 0 and inertia (1, 1, 0). [[0, 0], [0, −1]] has leading minors ≥ 0 and isn't semidefinite (P11a). `Form::congruent(&Gl<Q>)` gives PᵀFP.

A `Form` becomes a `Metrix` only through a draft (certificate) and a seal (signature).

## Isometry defect

`isometry_defect(T: &Gl<Q>, g_a: &Metrix, g_b: &Metrix) -> Result<Form, MetrixError>` = **Tᵀ·G_B·T − G_A**, for T reading frame A into frame B: B's ruler pulled back along T, minus A's (Mara §8.1, the orientation of `RelationDraft::new(from, to, T)`). Zero iff T is an isometry; indefinite in general (P11j: diag(3, −3/4)). The two rulers must have one size and T must act within it (`WrongSize` otherwise). It returns a `Result` because sizes can disagree; the brief's signature returned a bare `Form`.

Per Mara §6.4, this is where a `Metrix` meets kleinos: beside each edge, as its defect, never in the law.

## Relative spectrum

`relative_spectrum(g1, g2, budget) -> Result<RelativeSpectrum, MetrixError>`: the eigenvalues of G₁⁻¹G₂, the roots of det(G₂ − λG₁), as an **exact algebraic multiset**.

- G₁⁻¹G₂ is solved column by column through G₁'s certificate (forward by L, scale by D, back by Lᵀ). Nothing can fail: the certificate is total.
- Its characteristic polynomial over ℚ is Faddeev–LeVerrier (`gl::Charpoly::of`, shared with `Gl::charpoly`).
- Its primitive part's square-free part is factored by `poly` (Zassenhaus, against `budget`). Each irreducible factor f gets its multiplicity by exact division, and each of its real roots is an `Algebraic(f, index)`: mirror's `0x06` tag. All roots are real (G₁ is definite), so `Algebraic` expresses every eigenvalue.
- If the budget runs out, `is_certified()` is false and `unresolved()` holds the cofactor not yet split, with its multiplicities. Never a guess.
- Both rulers must be on **one frame** (`OtherFrame` otherwise): the eigenvalues mean something in one basis.
- **Basis-invariant:** a simultaneous congruence acts on G₁⁻¹G₂ by similarity (P11e). Per-axis unit changes drop out, so "did the coupling change between windows" needs no correlation (Mara §9.1).

**Caveat (Mara §8.3).** The relative eigenvalues are a **complete** invariant of the pair **only over ℝ, with one form definite** (simultaneous diagonalisation).
- **Over ℚ, they aren't complete,** even with both definite: (1) and (2) on ℚ¹ share relative eigenvalue 1, and aren't simultaneously congruent to (1), (1) over ℚ (p² = 2, P11f). Rational classes carry Hasse–Minkowski data.
- **For indefinite pairs, they aren't complete either:** P11g (one G₁⁻¹G₂, two signatures), P11h (a Jordan block). The complete invariant there is the pencil's Kronecker structure plus a sign characteristic (Lancaster & Rodman 2005).
- Here both are `Metrix`, so the spectrum is complete under GL_n(ℝ), and **necessary, not complete,** under GL_n(ℚ). `Form` pairs have no relative spectrum here.

## Not built, and why

- **`Measured<Metrix>` (D6).** prism has no generic measured wrapper: `band::Measurement` is one `f64` with its machine and method, and `Band` judges scalars near zero. A `Measured<Metrix>` would be a new type, not a fit. **Next:** a measured matrix (floats, machine, method, the window as signing context) that crosses into a `Metrix` only through `MetrixDraft::new` plus a seal, and the fit `Metrix::fit(sample, support) -> Result<Metrix, NotPD>` (Mara §9.1: never `mask`; zeros come from a covariance-selection fit, Dempster 1972).
- **A metric slot on `hypothesis::Frame`.** Not added. Composition needs no metric (Mara §4.2), and the frames test requires no metric on the composition path. A `Metrix` names its frame by address instead, so `Frame`, its address and `compose`/`hypothesize` are untouched, and nothing on that path can reference a metric. "Absent" stays "no lengths in this frame", not Euclidean (Mara §8.1).
- **Leading minors over ℤ** (`room::determinant` on submatrices, Mara §8.1): not needed, since LDLᵀ certifies.
- **Kronecker/sign-characteristic invariants for `Form` pairs:** OPEN (Mara §8.3), whether prism needs them.

## Tests (`prismqueer/tests/metrix.rs`)

25 tests, and 4 doctests in the module (two `compile_fail` with their controls). The full suite passes under coverage at 95.84% lines (ratchet 94), with `metrix.rs` at 96.77%.

- **Certification.** [[2,1],[1,2]] gives L = [[1, 0], [1/2, 1]], D = (2, 3/2), and `verifies` re-multiplies it (and rejects [[2,1],[1,3]]).
- **Refusals.**
  - [[1,2],[2,1]] is `NotDefinite { pivot: 2, value: −3 }`, and diag(−1, 1) is refused at pivot 1.
  - Zero pivots are refused: [[1,1],[1,1]] at pivot 2, P11a's [[0,0],[0,−1]] at pivot 1, diag(1,1,0) at pivot 3.
  - [[2,1],[0,2]] is `NotSymmetric { row: 1, col: 0 }`. A 1×1 on a 2-frame is `WrongSize`, and ragged rows are `NotSquare`.
  - The identity exists only as `MetrixDraft::identity(..).seal(..)`, and has the same address as the sealed literal I₃. There's no `Default` (autoref probe).
  - One draft sealed by two signers has one address and two signers. One matrix on two frames has two addresses.
- **Inertia.**
  - 2×2: diag(1,−1) → (1,1,0); [[0,1],[1,0]] → (1,1,0), by a 2×2 pivot; [[1,1],[1,1]] → (1,0,1); [[0,0],[0,−1]] → (0,1,1); 0 → (0,0,2).
  - diag(3,−2,0,5) → (2,1,1), and the empty form is (0,0,0).
  - 3×3: [[0,1,1],[1,0,1],[1,1,0]] → (1,2,0); [[0,1,0],[1,0,0],[0,0,−3]] → (1,2,0); vvᵀ with v = (1,2,3) → (1,0,2); the Krein grading diag(1,1,−1) → (2,1,0); [[0,1,0],[1,2,1],[0,1,0]] → (1,1,1).
- **Sylvester's law**, seeded over 128 draws: inertia(PᵀFP) = inertia(F), with n₊ + n₋ + n₀ = n. F is 1×1 to 4×4 with small entries (zero one time in three, so often singular, often with a zero diagonal). P is invertible over ℚ with denominators up to 3.
- **Isometry defect.**
  - The 3-4-5 rotation with I, I is the zero `Form`, (0,0,2).
  - The shear [[1,1],[0,1]] gives [[0,1],[1,1]], (1,1,0).
  - P11j: T = diag(2, 1/2) gives diag(3, −3/4), (1,1,0).
  - Orientation: T = diag(2,1), G_A = diag(4,1), G_B = I gives zero; swapping the rulers gives diag(15, 0), (1,0,1).
  - Mismatched sizes are refused.
- **Relative spectrum.**
  - I vs diag(2,3): charpoly (1, −5, 6), {x−2 #0, x−3 #0}.
  - diag(1,2) vs [[2,1],[1,1]]: charpoly (1, −5/2, 1/2), {2x²−5x+1 #0, #1}, irreducible (disc 17).
  - I vs diag(1,1,100): {x−1 ×2, x−100 ×1}.
  - Invariant under the simultaneous congruence P = [[1,2],[0,3]].
  - Across frames: `OtherFrame`.

**Mutations.** Each was applied, run against `metrix::` (and the doctests) and reverted:
- **Accepting a zero pivot as definite** (`dk < 0` for `dk ≤ 0`): fails `a_zero_pivot_is_not_definite`.
- **Inertia by leading minors** (unpivoted pivots; at the first zero, the rest counted as n₀): fails 5 tests, the 2×2, diagonal and 3×3 inertia tests, the shear's defect and the Sylvester property.
- **`impl Default for Metrix`:** fails `there_is_no_default_metrix` and the `compile_fail,E0599` doctest.
