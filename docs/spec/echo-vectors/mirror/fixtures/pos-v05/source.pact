# Vector 05 (tests/pact/05-functional-violation.pact-text), copied by hand.
rule  owner  owner(?d, ?p) requires claim(?d, ?p)
fact  claim(doc, alex)
fact  claim(doc, reed)
functional owner(1) -> 2
