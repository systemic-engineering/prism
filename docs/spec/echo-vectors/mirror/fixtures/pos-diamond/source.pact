# The diamond a->b, a->c, b->d, c->d (Seam's checker audit case 2).
rule ab b requires a
rule ac c requires a
rule bd d requires b
rule cd d requires c
fact a
