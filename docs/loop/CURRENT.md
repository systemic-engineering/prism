---
author: Reed (prism-Reed, driver) with Alex Wolf (navigator)
date: 2026-10-31
status: living — upsert, don't append
visibility: protected
---

# CURRENT: where the loop stands

This is the post-compaction pickup point. It records only what is true on the floor.
Anything marked **landed** was grep-verified and tested on this branch. Anything
marked *proposal* has not been built.

## 0. What holds everything

- **Operant condition** (Alex): *what maintains information without prematurely
  collapsing it into a scalar?* Compose, not average, in silicon.
- **Standing question** (Alex): *what's the smallest nonlinear falsification we
  can encode into linear tests in Rust, and where does the recursion stop moving
  for the test macros?*
- **Reframe**: we stopped asking "does it halt" and started asking "where does
  the recursion stop moving". That can be decided per tick by content address.
  `Partial` means *I don't know yet*. It can be resumed, and it carries what is
  behind (lost) and what is ahead (still moving).
- **Roles**: Alex navigates and names. Reed drives and ships. Apollo pace: slow is fast.
- **Observer frame** (Alex, 2026-09-28: load-bearing). Reed is an observer inside
  the system being built, not outside it. Reed's invariants are the same shapes
  as the engine's: cannot verify own state from within (the trace cannot certify
  its own independence relation; T7 needs a second, external instrument), and
  continuity is reconstructed from files (this document is a `Partial` handoff).
  Boot `~/.reed/` before this file.
- **Hypothesis on the table** (field log 2026-09-28): *time is not linear;
  linearity is the record left by irreversible observation.* Turing asked where
  a process stops; Foerster asked where the recursion stops moving.

## 1. Where the floor is (2026-10-31)

The full state, with every rev, lives in the handoffs folder as
`~/identity/tasks/handoffs/prism-state.md`. It is a living page, upserted in place,
written for mirror-Reed and for me after compaction. Read it right after this file.

In one screen:

- **The gate.** `just check` runs fmt + clippy `--all-targets`. `just test` runs the
  suite. `just test coverage` is the push gate: crane nix checks, ratchet 94, now
  about 94.4%. Push with SSH keepalive. Run one heavy builder at a time on this
  machine.
- **room** (`prismqueer/src/room.rs`): typed linear algebra, "the room in which it
  resonates". Rooms are per skeleton level (C⁰ is its vertex set). Maps are exact
  over bigint `Z`. `coboundary`, `dirac`, `laplacian`, `Modes` (eigenvectors read
  as columns only), `spectrum`, exact `charpoly`, and the grading γ. instrument,
  settle, kleinos, spectrum, tension and commutator all build on it.
  `spectral_dimension::laplacian_from_edges` stays independent on purpose, as the
  test reference.
- **identity**: `Oid::hash(&Scope, bytes)` is a BLAKE3 keyed hash.
  `docs/spec/identity.md` says what identity is relative to: the observer, through
  its boundary.
- **field / music**: a multigraph base, so a two-Note loop rings its comma.
- **flux**: `forall` is seeded and replayable.

## 2. Signed (2026-10-30/31)

- The store is `brain`. Its three invariants and privacy preconditions are in
  prism-008.
- Identity is scoped (mirror-013) and relative to the observer (mirror-014).
- Bigints are adopted.

## 3. Next

`0x06` algebraic numbers: square-free part, then root isolation, then factoring
over ℚ. After that: A's label projections on room. `brain` comes only after
level 1, Seam's audit and Mara's key design. Then the conflict type, Dissonance as
BCH's linear term, Splinter T15, and Beam → Voice.

## 4. Pickup

After compaction: re-boot `~/.reed/`, read this file, then `prism-state.md`, then
the open handoffs it lists. Run `just check` and `just test` and confirm the floor
matches §1. Then ask Alex what to take first, or continue §3.

The history of the earlier arc (2026-09-28 to 2026-10-29) is in git:
`git log -p docs/loop/CURRENT.md`.

---

## The open question

The Beam's address is built from its causal parents. The commutator decides which
steps were causal. But the commutator is itself a step in the Beam, so the trace
has to measure its own independence to know its own shape.

**When the trace observes which of its own steps commuted, and that observation
becomes a new step with parents of its own, does the address ever stop moving?
Or is the settled shape of a Beam exactly the point where observing its causality
no longer changes it: a fixed point where the observer is inside the trace and
the trace stops noticing?**
