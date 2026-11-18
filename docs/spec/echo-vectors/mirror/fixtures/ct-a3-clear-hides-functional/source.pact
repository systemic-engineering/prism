# CT-A3 (Seam's checker audit case 3): vector 05.
rule  owner  owner(?d, ?p) requires claim(?d, ?p)
fact  claim(doc, alex)
fact  claim(doc, reed)
functional owner(1) -> 2
