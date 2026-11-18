# Vector 03 (tests/pact/03-false-with-provenance.pact-text), copied by hand.
rule ready ready(?a) requires application(?a), answered(?a, alex, yes)
rule send send(?a) requires ready(?a), pdf_stands_alone(?a), dates_only(?a), not reader_tracking(?a)
fact application(a1)
fact pdf_stands_alone(a1)
fact dates_only(a1)
query send(a1)
