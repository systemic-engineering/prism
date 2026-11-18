# Vector 01 (tests/pact/01-determinism.pact-text), copied by hand.
rule path-base path(?x, ?y) requires edge(?x, ?y)
rule path-step path(?x, ?z) requires path(?x, ?y), edge(?y, ?z)
fact edge(a, b)
fact edge(b, c)
fact edge(c, b)
fact edge(c, d)
