# CT-A4 (Seam's checker audit case 4): vector 01.
rule path-base path(?x, ?y) requires edge(?x, ?y)
rule path-step path(?x, ?z) requires path(?x, ?y), edge(?y, ?z)
fact edge(a, b)
fact edge(b, c)
fact edge(c, b)
fact edge(c, d)
