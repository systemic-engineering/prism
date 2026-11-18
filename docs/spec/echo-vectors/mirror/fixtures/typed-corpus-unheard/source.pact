# trace.pact: the relations a trace derives from its span facts.
#
# Scope: docs/audits/2026-11-17-seam-trace-layer-and-first-metrix-scope.md
# (Seam ee20c0b9) section 2. Design: Mara 34f70d6d (kleinos is a pushout
# along spans). Beside shards/trace.mirror, which declares the wire form.
#
# A span is an edge between two events: span(emit, receive). Its identity
# is the full pair of addresses; the 8 + 8 hex form is a live-telemetry
# label, computed at render and never joined on here.
#
# Event layer only. Readings (transports, inverses, holonomy) are the
# reading layer's, a slot awaiting Mara's design verdict: no rule here
# reads a span backward, and reading back is never an event.
#
# Record types, prose until the v1 reader reads `type`:
#   span    = { emit: addr, receive: addr }             (an @edge)
#   measure = { span, name, value: nat, source }        source: atoms | ticks | wall_driver
#   content = { span, reads: addr, returns: addr }      a step's content projection
# A measurement sits beside a span and never enters its address.
#
# candidate(e, r, pe): the span (e, r) starts where the span (pe, e) ends.
# It is parent and links in one relation. The pact derives every candidate
# and never chooses: one candidate is the parent by derivation; at a
# fan-in, picking one is a spanning-tree gauge (Mara 34f70d6d s2.3), done
# at render, labelled presentation, never signed, never in an address.
# There is no `parent` relation.
#
# Names are placeholders: "span" is OTel's word; the driver says "step"
# (span.rs's span is a parse interval). Alex and Mara pin them.

rule cand candidate(?e, ?r, ?pe) requires span(?e, ?r), span(?pe, ?e)
rule recv received(?r) requires span(?e, ?r)
rule root root(?e, ?r) requires span(?e, ?r), not received(?e)

# bullet-up-011's six spans over nine commits of the systemic.engineering
# corpus (full 40-hex commit ids, resolved with git rev-parse from the
# prefixes in Mara's probe, mara/frames-hypothesis df9cccc3).
# Typed: a git commit id is 20 bytes of SHA-1, so `bytes`, not `addr`
# (a BLAKE3 address, 64 hex).
fact span(bytes "552e3785a76a48197bc0844c82cff397a4bdf906", bytes "0e97587c4125a87a85a70589ff3e15860acb1b6e")
fact span(bytes "0e97587c4125a87a85a70589ff3e15860acb1b6e", bytes "457086e9b40a06aff9158e2ef9f751eb00309ebb")
fact span(bytes "457086e9b40a06aff9158e2ef9f751eb00309ebb", bytes "a15849844b0e694aaf7b1a43da4aa59cda25bfb7")
fact span(bytes "878304ec46b54c37e29f08b05e15fcc754e2e0f0", bytes "6ff3f452cebd0aa039423d595b4c3af785081a02")
fact span(bytes "6ff3f452cebd0aa039423d595b4c3af785081a02", bytes "af20d9cad27c0ed15c96a14e312242a468861b7d")
fact span(bytes "0a261ff8316b0156a71050625d4f9f10277f7364", bytes "ab55253bd5de935b84e24e3cc4a6911f11d8732e")
