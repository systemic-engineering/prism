# coverage: the floor at 100%, every gap a reasoned skip

*prism, 2026-11-25 (the step laws.md's "Coverage" named next). Checks: `checks.laws-coverage` (the gate), `checks.coverage-skips` (the filter on its fixtures). Filter: `nix/coverage-skips.sh`. Ratchet: `nix/laws-skips-max`. Fixtures: `nix/coverage-fixtures/`.*

## The rule

`prism-laws` (`laws/src`) is covered at **100% of its lines**. A line no test reaches is either reached by a new test, made unrepresentable, or skipped by a marker in the source that says why. Leaving it uncovered is never the answer.

The rest of the workspace keeps its ratchet (`coverageFloor`, 94, in `checks.coverage`). The floor is measured on its own, beside it.

## The mechanism

### Markers, in the source

```rust
// coverage: skip(<reason>)
the_one_line_skipped();

// coverage: skip-start(<reason>)
fn every_line_up_to_the_end_marker() {}
// coverage: skip-end
```

- `// coverage: skip(<reason>)` excludes the next line.
- `// coverage: skip-start(<reason>)` … `// coverage: skip-end` excludes every line between them.
- `<reason>` is required and non-empty: it names **why** no test reaches the code, not what the code is.
- A marker is a comment on a line of its own. Nothing else in the source changes: no attribute, no `cfg`, no build flag.

### The filter: `nix/coverage-skips.sh <lcov> <root> <prefix> <max-file>`

It reads the lcov that `checks.coverage` writes (the whole workspace, one instrumented run; `SF:` paths are repo-relative, `nix/lcov-paths.sh`), keeps the `DA:` records under `<prefix>` (`laws/src`), reads every source under `<prefix>`, measured or not, and drops the lines inside skips. Then it fails unless:

1. **Every marker parses and has a reason.** `skip()`, `skip(   )`, a bare `skip`, a `skip-start()` and a marker it can't read all fail. So does a marker after code on the same line.
2. **Every region is well formed.** No `skip-start` or `skip` inside a region, no `skip-end` without a region, no region open at the end of a file, no `skip` followed by another marker, no `skip` on a file's last line.
3. **Every skip has lines under it.** A skip over a comment, a blank line or a file the lcov doesn't name skips nothing: "skip with no lines under it".
4. **No skip covers a line that ran.** A skip says no test reaches this code; an executed line inside it makes the reason a lie: "skip covers executed code". This holds for every skip, so a region that crosses a `fn` boundary into covered code is refused for the covered lines, and allowed only when every line in it is uncovered.
5. **Every other line under `<prefix>` ran.** Lines executed / lines instrumented is 100% once the skipped lines are dropped. The uncovered ones are listed by file and line.
6. **The skipped lines number exactly the ratchet.** `nix/laws-skips-max` holds the current count. More fails ("exceed the ratchet"); fewer fails too ("lower it to N"), so the file always holds the real number and can only go down.

A line counts as instrumented when the lcov has a `DA:` record for it; a skipped line counts once however many regions llvm-cov puts on it.

### Its outputs

`checks.laws-coverage`'s `$out` is a directory:

- `skips.txt`: **the ledger**, one line per skip, sorted: `file:first-last`, the number of lines skipped, the reason, tab-separated. Empty when there are no skips.
- `summary.txt`: the verdict, `laws/src: H/F lines executed (100%), S skipped in K skips (ratchet M)`.

### The filter's own tests: `checks.coverage-skips`

`nix/coverage-fixtures/<case>/` is a tiny lcov (`cov.lcov`), its sources (`laws/src/*.rs`), a ratchet (`max`), and `expect`: `ok`, or `refuse: <text>`, the reason the filter must give. An accepted case may carry its exact `ledger`. The check runs the filter on each: accepted cases pass with their ledger; each refused case fails, and for its own reason.

- Accepted: no skips (`ok-none`), one line (`ok-skip`), a region with a blank line inside (`ok-region`); each beside an uncovered line in `prismqueer/src`, outside the prefix.
- Refused, a reason missing or unreadable: `empty-reason`, `blank-reason`, `bare`, `empty-region-reason`, `unparsed`, `trailing`.
- Refused, a dishonest skip: `covered` (one executed line), `region-covered` (a region over two `fn`s, one of them run), `nothing-under` (a blank line), `unmeasured-file` (a file the lcov doesn't name).
- Refused, shape: `unclosed`, `stray-end`, `nested`, `doubled`, `at-eof`.
- Refused, the gate: `uncovered`, `ok-skip-partial` (a one-line skip leaves the rest of its `fn` uncovered), `over-ratchet`, `under-ratchet`, `bad-max`, `no-data` (no records under the prefix), `missing-source` (the lcov names a file that isn't there).

## The policy: a test, then a refactor, then a skip

1. **Ordinary code and accessors: a test.** Code that can simply be exercised is exercised. No skips here.
2. **Unreached branches and error paths:** construct the input and test it; or, when the branch can't happen by construction, **make the state unrepresentable** (Alex's rule) so the branch is gone. A skip only when neither is possible, and its reason names *why*.
3. **`Debug` and `Display`: a test that pins the text.** A stable printing is part of the interface; never skipped.

## The ledger

**None.** `nix/laws-skips-max` is 0: every line of `laws/src` is reached by a test.

At `f98de6d` the floor stood at 98.95% (5725/5786; 61 lines uncovered). The 61, and what answered each:

- **Accessors, tested:** `algedonic::Enclosure::hi`, `Evidence::site`, `Evidence::oids`; `hypothesis::LoopId::oid`, `Located::frames`; `measured::Measured::frame`, `Measured::method`; `metrix::Form::dim`, `RelativeSpectrum::unresolved`; `room::Q`'s `Coefficient::ring` and `Coefficient::times`.
- **`Debug` and `Display`, pinned:** `crystal::Crystal`'s `Debug`, `kleinos::Label`'s `Display`.
- **Branches and error paths, tested by constructing the input:** `ZPoly::square_free` and `QPoly::monic` of 0; the real roots of 0 (an empty Sturm sequence); `ZPoly::div_exact` of 0, and with a leading coefficient that doesn't divide; 𝔽ₚ's `gcd(0, 0) = 0` (the monic of nothing); Berlekamp on a linear polynomial; `Factorization::identify` on a constant's factorization (no factors: `Absent`), with a negative radius (`Unmeasured`), and with two roots of one factor in the window (`Ambiguous`); `room::quietest` of the empty graph (its minimal polynomial is 1: no kernel to divide out, `Silent`); `Form::congruent` by a P larger than the form (`WrongSize`); `relative_spectrum` with no budget (`Uncertified`, the rest `unresolved`); `Ruler::read` of an empty neighbourhood under `Max` (the unit, 0).
- **States made unrepresentable:**
  - `gl`'s echelon form asked a nonzero pivot for its inverse and carried an `if let` for a nonzero rational with none. The pivot search now finds the row whose entry *has* an inverse (`find_map` over `Q::recip`), which is exactly the nonzero test, so there is no second branch.
  - `metrix::relative_spectrum` factored the square-free part of its charpoly and kept an `Err` arm for `NotSquareFree`. The charpoly is monic, so nonzero, and the square-free part of a nonzero polynomial is square-free: the arm is the invariant `room::quietest` already states for the same call, `expect`ed with that reason.

## Why not `#[coverage(off)]`

The attribute would say the same thing on the item itself, and llvm-cov would honour it. But it is unstable: rustc 1.94 (the pinned toolchain) rejects it with E0658, and `#![feature(coverage_attribute)]` would put the floor on nightly. A `cfg_attr` behind a coverage-only cfg still needs the feature gate on the build that sets it, so the instrumented build would be nightly and the measured code would not be the code the gate builds. Markers in comments change nothing the compiler sees, so the code measured is the code shipped, and the filter can hold them to rules the attribute has no way to: a reason, honesty against the lcov, and a ratchet on their number.
