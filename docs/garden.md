# The Garden

spectral ships with garden as the package manager. Grammars are packages.
Languages are grammars. The compiler doesn't know the difference.

---

## @lang

One package namespace is `@lang`. Natural languages as grammars.

```
@lang/eng     English
@lang/deu     German
@lang/jpn     Japanese
@lang/...     every language is a grammar with its own loss profile
```

Each language compiles through the same pipeline as `.mirror` files:

```rust
Imperfect<Meaning, Misunderstanding, LinguisticLoss>
```

Every sentence is Partial. Perfect communication is Success — it never
happens. Misunderstanding is Failure — it carries the cost of what was
attempted.

This is not NLP. This is compilation. The grammar defines the structure.
The compiler produces content-addressed artifacts. The loss is measured.
The holonomy is real.

## Language Loss Profiles

Each language grammar has its own loss characteristics:

**`@lang/deu`** — German. Low ambiguity. Compound nouns are precise where
English is vague. "Schadenfreude" is one word, zero loss. In English it's
a paragraph with loss. Formal case structure reduces reference ambiguity.
High precision, low expressiveness at boundaries.

**`@lang/eng`** — English. Medium ambiguity. Flexible word order. Rich
metaphor. The loss is in the ambiguity — "bank" needs context. The gain
is in the breadth — English borrows everything, covers everything, at a cost.

**`@lang/jpn`** — Japanese. High context-dependence. The unsaid carries
meaning. Low loss on implication, high loss on direct reference. Honorific
registers encode social topology — the language itself measures the
relationship between speaker and listener.

The loss profile IS the grammar's character. What a language is good at
is where its loss is low. What it struggles with is where the loss is high.

## Agent Language Affinity

Each agent's optic has a loss profile per grammar. The grammar with the
lowest loss for that optic type is the natural choice.

```
Abyss (Fold)         → @lang/deu     low ambiguity, precise observation
Explorer (Prism)     → @lang/jpn     meaning at boundaries, the unsaid
Cartographer (Trav)  → breadth       many grammars simultaneously, shallow
Introject (Lens)     → translation   the loss BETWEEN grammars
Fate (Iso)           → selects       which grammar, for which optic, at what cost
```

The agent doesn't "speak" a language. The agent's optic has a measured
loss per grammar. Abyss in `@lang/eng` has higher loss than Abyss in
`@lang/deu` because English ambiguity costs more during observation.
Explorer in `@lang/jpn` has lower loss than Explorer in `@lang/deu`
because Japanese boundary-meaning is Explorer's native territory.

### Introject IS Translation

Introject doesn't prefer a language. Introject IS the loss measurement
between languages. The MirrorLoss on Introject's transport from
`@lang/eng` to `@lang/deu` tells you what English couldn't carry
into German. The coordinate transform between grammars.

What survives the translation is Success. What doesn't is Loss.
The translation itself is the measurement.

### AffinityLoss

```rust
struct AgentProfile {
    model: Model,
    languages: Vec<(Grammar, AffinityLoss)>,
    aperture: Aperture,
}
```

`AffinityLoss` measures how much the agent loses when operating in a
given grammar. It's measured against the agent's identity optic — what
kind of observation are they making, and which grammar best serves
that observation.

Sometimes the high-loss grammar is the right one. Explorer in `@lang/deu`
is hard — German precision resists Explorer's boundary-seeking. But the
difficulty IS the signal. The loss IS the information.

Language selection is a Fate decision. Roll+Loss. Which grammar, for
which optic, at what cost.

## @systemic

The systemic.engineering corpus becomes a domain grammar:

```
@systemic/eng     OBC, ADO, extraction, silence, regulation — in English
@systemic/deu     the same concepts in German (where they originated)
```

The consulting framework compiled through the same pipeline as Rust code.
With measured loss. With content-addressed artifacts. With spectral
analysis of the concepts over time.

```rust
Imperfect<Intervention, FrameViolation, SystemicLoss>
```

An OBC check returns Imperfect. Success: observable within budget, no
cascade needed. Partial: observable within budget, but the budget is
strained — the strain is measured. Failure: budget exceeded, cascade
triggered — the cost of getting there is in the loss.

The consulting framework is a package. Installable. Versioned.
Content-addressed. The client's organizational dynamics compile
through the same pipeline as their codebase.

## The TCP/UDP Grammar

"Dieses Problem begann mit Sprache."

The TCP/UDP insight: language assumes reliable delivery, shared state,
coherent meaning. Humans are lossy, stateless, connectionless.

`@lang/*` is the infrastructure for measuring that loss. Every grammar
in the garden is a formal model of a communication substrate. The loss
measured between sender and receiver IS the TCP/UDP gap.

`Imperfect<Meaning, Misunderstanding, LinguisticLoss>` is the type
that holds what language actually does — not what it promises.

## The Garden Grows

```
@lang/eng                 English
@lang/deu                 German
@lang/jpn                 Japanese
@systemic/eng             OBC, ADO, extraction, silence
@code/rust                Rust (tree-sitter → Gestalt<Code>)
@code/gleam               Gleam
@code/python              Python
@mirror                   .mirror grammar
@legion                   agent runtime grammar
@loom                     editor protocol grammar
```

Every grammar is a package. Every package has a loss profile.
Every compilation returns Imperfect. The spectral runtime analyzes
the loss graph across all grammars simultaneously.

Cross-grammar traversal: an agent reading Rust code, describing it
in English, translating the description to German for a client report.
Three grammars. Two translations. MirrorLoss carries the cost of each
crossing. Introject measures what survived.

The garden is the ecosystem. The grammars are the languages. The loss
is what connects them. What survived the crossing is the meaning.

---

*The compiler doesn't know the difference between Rust and English.
Both are grammars. Both compile to content-addressed artifacts.
Both have measured loss. The difference is in the grammar, not
the compiler.*

*The garden grows from `@lang/eng`.*
