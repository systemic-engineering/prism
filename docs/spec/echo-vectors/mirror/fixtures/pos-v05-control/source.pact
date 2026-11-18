# Vector 05, control (tests/pact/05-functional-violation-control.pact-text), copied by hand.
rule  owner  owner(?d, ?p) requires claim(?d, ?p)
fact  claim(doc, alex)
functional owner(1) -> 2
