#!/usr/bin/env bash
# Fail if prismqueer built with `--no-default-features --features exact` links
# anything native beyond blake3's own C (docs/spec/features.md; prism-025 §1;
# contract rule 7: the certified path is exact).
#
# The closure is the library build's: normal and build edges, at every depth
# (nix/independent.sh, <dev> = none). prismqueer's own tests may use what the
# library never links. Forbidden, by package name:
#   - `cc`: a C compiler step. prismqueer's own (an optional build-dependency)
#     is `lapack`'s, never `exact`'s;
#   - anything LAPACK, BLAS or Fortran (*lapack*, *blas*, *fortran*,
#     netlib-src, intel-mkl*, accelerate-src), and any package whose `links`
#     key names lapack, blas or fortran.
#
# The one `cc` allowed: blake3's build-dependency, which compiles blake3's
# own SIMD (C intrinsics, or assembly on x86) unless its `pure` feature is
# on. It is BLAKE3, the hash every Oid is, not a numeric backend; its edge
# to cc is set aside before the walk, and every other path to cc still
# counts. blake3 is pinned (=1.8.7), as echo's is.
#
# Input: `cargo metadata --format-version 1 --no-default-features --features
# prismqueer/exact` JSON (the resolve follows the features asked for).
#
# Usage: prismqueer-exact.sh <metadata.json>
set -euo pipefail

meta=${1:?usage: prismqueer-exact.sh <metadata.json>}
forbidden='^(cc|.*lapack.*|.*blas.*|.*fortran.*|netlib-src|intel-mkl.*|accelerate-src)$'

pruned=$(mktemp)
trap 'rm -f "$pruned"' EXIT
jq '
  ([.packages[] | select(.name == "cc") | .id]) as $cc
  | ([.packages[] | select(.name == "blake3") | .id]) as $blake3
  | .resolve.nodes |= map(
      if (.id as $id | $blake3 | index($id)) != null
      then .deps |= map(select(.pkg as $p | ($cc | index($p)) == null))
      else . end)
' "$meta" > "$pruned"

found=$(bash "$(dirname "$0")/independent.sh" "$pruned" prismqueer "$forbidden" none)

if [ "$found" = "!no-root" ]; then
  echo "✖ prismqueer-exact: no single package named prismqueer in $meta" >&2
  exit 1
fi
if [ -n "$found" ]; then
  echo "✖ prismqueer-exact: prismqueer under \`exact\` links forbidden packages:" >&2
  echo "$found" | sed 's/^/    /' >&2
  exit 1
fi
echo "✔ prismqueer-exact: no cc (but blake3's), LAPACK, BLAS or Fortran under prismqueer/exact"
