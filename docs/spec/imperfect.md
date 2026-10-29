# Imperfect: a portable spec

*prism-001, 2026-10-27. Written from terni 0.7.0 (`imperfect/src/lib.rs`), so that a port in another language (first: Gleam, for bullet-up) can be checked against the real crate. Test vectors: `imperfect-vectors.json`, generated from the crate by `prismqueer/tests/spec_vectors.rs`.*

## What it is

A result with three cases instead of two:

| Case | Means | Carries |
|---|---|---|
| `Success(value)` | It worked, nothing was lost. | the value |
| `Partial(value, loss)` | Here is what I have. Something was lost on the way, or something isn't settled yet. | the value and the loss |
| `Failure(error, loss)` | It didn't work. Here is what it cost to get here. | the error and the loss so far |

`Partial` is the honest middle: "I don't know the rest yet." It promises nothing about the future.

Wire format (terni's serde, used by the vectors):

```json
{"status": "success", "value": 2}
{"status": "partial", "value": 2, "loss": 1}
{"status": "failure", "error": "boom", "loss": 1}
```

## Loss

A loss type has four things: `zero` (nothing lost), `total` (everything lost), `is_zero`, and `combine` (put two losses together, in order).

Laws:

1. **Identity.** `combine(zero, x) = x = combine(x, zero)`.
2. **Associativity.** `combine(combine(a, b), c) = combine(a, combine(b, c))`.
3. **Absorbing total.** `combine(total, x) = total = combine(x, total)`.

The vectors use two loss types:

- **count**: a non-negative integer. zero = 0, combine = add (saturating), total = 2⁶⁴−1. All three laws hold.
- **list**: a list of strings saying *what* was lost. zero = `[]`, combine = append. Laws 1 and 2 hold. **Law 3 does not**: terni returns `[]` for `total`, so `total` is the same as `zero` and absorbs nothing. A list has no natural "everything is gone" value. This is measured in prism (`prismqueer/tests/loss_laws.rs`) and visible in the vectors (`combine` with `total`).

**If your port needs a lost-everything value for a list loss, add it as its own case:** `Known(list) | Lost`, with `combine` returning `Lost` if either side is `Lost`, and appending otherwise. All three laws then hold; prism audits exactly this shape (`Progression<u8, Monophony>`, `prismqueer/src/time/progression.rs`). Don't encode `Lost` as a special list.

**Don't use floating-point numbers for a loss you need to compare exactly.** Float addition isn't associative at rounding (also measured in `loss_laws.rs`).

**JavaScript targets:** count's `total` is 18446744073709551615, which a JavaScript number can't hold exactly. On a JS target, give count a separate total case too.

## Operations

Rules, case by case. `f`, `g` are the caller's functions.

| Operation | Success(v) | Partial(v, l) | Failure(e, l) |
|---|---|---|---|
| `map(f)` | Success(f(v)) | Partial(f(v), l) | Failure(e, l) |
| `map_err(f)` | Success(v) | Partial(v, l) | Failure(f(e), l) |
| `eh(f)` (bind; aliases `imp`, `tri`) | f(v) | see below | Failure(e, l), f not called |
| `recover(f)` | Success(v) | Partial(v, l) | see below |
| `unwrap_or(d)` | Success(v) | Partial(v, l) | Partial(d, l) |
| `loss()` | zero | l | l |
| `compose(next)` (older name for sequencing) | next | see below | Failure(e, l) |

**`eh(f)` on `Partial(v, l)`:** run `f(v)` and carry `l` into its result:

- f gives `Success(u)` → `Partial(u, l)`
- f gives `Partial(u, l2)` → `Partial(u, combine(l, l2))`
- f gives `Failure(e2, l2)` → `Failure(e2, combine(l, l2))`

`compose(next)` on `Partial(_, l)` follows the same three rules with `next` in place of `f(v)`.

**`recover(f)` on `Failure(e, l)`:** run `f(e)` and carry `l`:

- f gives `Success(u)` → `Partial(u, l)`. Recovering never makes a `Success`: the failure happened.
- f gives `Partial(u, l2)` → `Partial(u, combine(l, l2))`
- f gives `Failure(e2, l2)` → `Failure(e2, combine(l, l2))`

**Constructors:** `success(v)`, `partial(v, l)`, `failure(e)` = `Failure(e, zero)`, `failure_with_loss(e, l)`.

**The sequencing context (`Eh`)** lets a caller run several steps and collect their loss:

- each step gives back its value (or its error), and its loss is added to the context, **including a zero loss from a `Partial`**;
- a `Failure` step's loss is added too, before its error is returned;
- `finish(v)` gives `Success(v)` if **no** step recorded anything, and `Partial(v, accumulated)` otherwise, even if the accumulated loss is zero.

## Laws a port must keep

1. **Map is a functor.** `map(id) = id`; `map(f).map(g) = map(g ∘ f)`.
2. **Bind is a monad, given an associative loss.**
   - left identity: `Success(a).eh(f) = f(a)`
   - right identity: `m.eh(Success) = m`
   - associativity: `m.eh(f).eh(g) = m.eh(x ↦ f(x).eh(g))`. This holds exactly when `combine` is associative, so it fails for float losses.
3. **Failure short-circuits and keeps its cost.** After a `Failure`, `eh`, `map` and `compose` don't run anything new, and the loss stays.
4. **Recovery never yields `Success`.** `recover` and `unwrap_or` on a `Failure` give `Partial` (or another `Failure`).
5. **`Partial` is sticky.** No composition turns a `Partial` back into `Success`, not even with zero loss. Only building a fresh `Success` does.

## What `Partial` with zero loss means

"Here is the value, nothing has been lost so far, and it isn't settled." It's a real state, not a rounding of `Success`: loss looks back (what was lost), `Partial` also looks forward (what's still open). That's why it's sticky (law 5) and why the `Eh` context reports it as `Partial`. In the vectors: every case whose input is `{"status": "partial", "loss": 0}` or `[]`.

## What must never happen

- **Composition dropping loss.** `map`, `map_err`, `eh`, `recover`, `unwrap_or`, `compose` and the `Eh` context all keep it.
- **A `Failure` becoming a `Success`.**
- **A `Partial` becoming a `Success` through binding.**
- **A failure's earlier cost being thrown away.** `Failure` carries the loss accumulated before it.
- **Silent loss.** Loss may only leave through exits that say so by name: `ok()` (the value, or nothing), `err()`, `unwrap`/`expect` (value or crash), and conversion to a two-case result (`Result`, which keeps the value of `Success` and `Partial` and drops the loss). A port should give these exits names that make the drop visible.

## Conversions

| From | Gives |
|---|---|
| `Ok(v)` | `Success(v)` |
| `Err(e)` | `Failure(e, zero)` |
| `Some(v)` | `Success(v)` |
| `None` | `Failure((), zero)`: absence is a failure with no further detail |
| `Imperfect` → `Result` | `Success(v)` and `Partial(v, _)` → `Ok(v)`; `Failure(e, _)` → `Err(e)`. Drops loss: an exit. |

## Checking a port

Run every case in `imperfect-vectors.json`. Each case has `loss` (`count` or `list`), `op`, `input`, `args` and `expected`, all in the wire format above. Values are integers, errors are strings. The functions the operations use are listed in the file's `functions` field (e.g. `map` adds 1, `eh` returns `args.next` with its value increased by the input value). A mismatch is a bug on one side. If you think it's prism's, write it up as a handoff.

The vectors are regenerated from the crate with `PRISM_SPEC_UPDATE=1 cargo test -p prismqueer --test spec_vectors`, and that test fails if the committed file and the crate ever disagree.
