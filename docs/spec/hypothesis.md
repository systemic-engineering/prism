# Hypothesis: frames, their composite, and the unranked menu over it

*prism, 2026-11-17 (prism-021). Sources: Mara's frames note on mirror branch `mara/frames-hypothesis`, **54c88f72** (`docs/math/2026-11-17-mara-hypothesis-as-composition-of-frames.md`, §4.1–§4.3, §5, §11 "For prism"), and its probe **c012e29f** (`frames_probe.py`, 44/44, P5 for the averaging control). Decided by Alex as D1–D7 (mirror STATE #77–#78). Names are provisional and Mara's. Code: `prismqueer::gl`, `prismqueer::hypothesis`; tests: `prismqueer/tests/hypothesis.rs`.*

## The two decisions this implements

**D1, the hinge (PROVED, Theorems A–D).** An average is a *quotient* of the composition, taken after two choices the composition never makes: a **gauge** (the tree along which readings are carried to one place) and a **ruler** (weights or a metric). With a loop present, two honest gauges give two averages (Theorem C). Composition needs no metric. So nothing on this path names one.

**D2, `Hypothesis`.** A hypothesis is **the set of unranked closures proposed per located loop**, with the computed composite as its **ground**. Agreement, holonomy, curvature and provenance are all functions of the key (frames, relations, faces), so they are the *composite's interpretation*, not a hypothesis. The hypothesis is what no recompute establishes: a menu (ADO's Offer), never picked, never ranked.

## The types

- **`Gl<R>`**, `R ∈ {ℚ, ℤ}`: an invertible square exact matrix, a `field::Group` (path order: first `a`, then `b`, i.e. `b·a` on columns) and `field::Exact`. So `Field<Graph, Gl<R>>`, `check_flat`, `Curved::loops` and faces work unchanged.
  - Refused at construction, never later: not square; singular (det 0); over ℤ, det not ±1.
  - One identity at every size (as `Perm`): a trailing identity block is trimmed, and a transport acts as the identity past its size. A frame's dimension lives on the frame.
  - Compared exactly. Never by trace: the shear `[[1,1],[0,1]]` has the identity's trace *and* charpoly.
  - ℚ is `room::Q`, exact (`BigRational`). `QModOne` is ℚ/ℤ (no product) and `f64` rounds.
- **`Frame`**: a site. Address `"frame\0" ‖ bytes(reader) ‖ u64le(dim)` (public scope), stalk dimension `dim`. No metric slot yet (`Metrix` comes later, D5).
- **`Relation<R>`**: a `Gl<R>` from one frame's stalk to another's, by address. Address `"relation\0" ‖ bytes(from) ‖ bytes(to) ‖ u64le(n) ‖ n² rationals`. "Signed" means *attributed*: only `RelationDraft::seal(by)` makes one, and the signer sits beside the address (`ruler.md`). One relation signed twice is one edge with two signers.
- **`Face`**: a triangle of frames someone reports filled, `FaceDraft::seal(by)`. Address `"face\0"` then the three corners ascending.
- **`Interpretation<R>`**: a reading that carries its frame by address.
- **`Composite<R>`**, the composite's interpretation (address `"composite\0"` then frames, relations and faces, each counted and ascending; signers beside):
  - **agreement**: a basis of H⁰ = {s : s(to) = T·s(from) for every relation}, one `Interpretation` per frame. Over ℤ, a ℤ-basis of the kernel lattice;
  - **holonomy**: per located loop, its closed walk (frames and relations by address), its holonomy (an exact `Gl`) and its cycle certificate (a 1-chain z with ∂z = 0). The tree is named beside the loops: only the *set* is canonical, a basis is claimed with its tree. Never called H¹ (for matrix transports H¹ counts the fixed directions plus r(β₁ − 1), not the twist; over ℤ it also has torsion);
  - **curvature**: per reported face that doesn't close;
  - **provenance**: every signer.
- **`Hypothesis { ground: Oid, alternatives: BTreeMap<LoopId, BTreeSet<Closure>> }`**. No ranking field. A ranking is a separate record under a named ruler.
- **`Closure`** (Mara §4.3, after *Whose Metric?* §3; the set of moves is READING, their effects OPEN):
  - `Fill(Polygon)`: declare the loop closed, a face no one reported;
  - `Twist(relation)`: one frame changes a relation inside its own reach;
  - `Absorb(frame)`: one frame takes over the others' reach;
  - `Share(Bridge)`: the loop's frames sign one new relation together;
  - `Bias(relation)`: a systematic bias on a relation, indistinguishable from a twist without a second, independent route.

  Per loop: one `Fill`, a `Twist` and a `Bias` per relation on it, an `Absorb` per frame on it, one `Share`.

## `hypothesize(frames, relations, faces)`

`-> Imperfect<Hypothesis, CompositionError, BTreeSet<Obstruction>>` (terni's arms: the loss is what stays open).

- **Success** when flat: no open loop, no curved face. The alternatives are empty.
- **Partial** otherwise. The loss names each open loop (`Obstruction::Loop`, which has alternatives) and each curved reported face (`Obstruction::Face`, an answer, which has none).
- **Failure** only on a type error: a frame named twice at two dimensions, an unknown frame, related frames of different dimension, a transport larger than its frames, a face whose sides aren't single relations.
- **The empty input is the unit**: `Success`, never `Failure`.

## The tests (Mara's first red test and falsifiers)

- **Main**: A→B = I, B→C = I, C→A = R = [[3/5, −4/5], [4/5, 3/5]]. Agreement 0; one loop with holonomy R (or D·R·D⁻¹ = R⁻¹, by orientation), certificate ∂z = 0, tree of two; 11 alternatives; no metric (signature, plus a scan of `gl.rs` and `hypothesis.rs`).
- **F1** R → I: Success, agreement 2. **F2** gauge at B: same loop, charpoly bytes identical. **F3** shear: one loop at trace 2. **F4** singular / non-unit refused. **F5** a face reported by C: curvature on the face, no open loop. **F6** averaging control (P5a, a test, not an API): trees {AB, AC} and {AB, BC} give (4/15, 4/5) and (2/3, 2/3); flat, they agree.

## Not here yet

- `Metrix` / `Form` (D5), `kleinos` as a pushout along spans (D4): next, in that order.
- `Interpretation`'s crossing under a named ruler (Mara §11 lists `crossing: Membership`): it needs a ruler on the path, which composition doesn't.
