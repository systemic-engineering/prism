# Reads, but two engine errors: arity (q at 1 and 2) and safety (?x).
# @mirror/dark-order: arity before safety.
rule s p(?x) requires q(?y)
fact q(a, b)
