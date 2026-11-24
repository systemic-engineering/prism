#!/usr/bin/env bash
# Fail if the `echo` package depends on anything forbidden (prism-022 S5).
#
# echo is the second reader of mirror's output; it may not share code with
# what it checks, nor carry prismqueer's LAPACK/BLAS/Fortran backend into the
# verifier path. Forbidden, by package name:
#   - prismqueer;
#   - any mirror crate (`mirror`, `mirror-*`, `mirror_*`);
#   - anything LAPACK, BLAS or Fortran (by name: *lapack*, *blas*,
#     *fortran*, netlib-src, intel-mkl*, accelerate-src), and any package
#     whose `links` key names one (*lapack*, *blas*, *fortran*): a native
#     library is linked under the `links` name, whatever the crate is called.
#
# Input: `cargo metadata --format-version 1` JSON. The graph is resolve.nodes:
# every dependency kind (normal, build, dev) and every target counts, at every
# depth (the shared walk, nix/independent.sh, with <dev> = all).
#
# Usage: echo-independent.sh <metadata.json>
set -euo pipefail

meta=${1:?usage: echo-independent.sh <metadata.json>}
forbidden='^(prismqueer|mirror|mirror[-_].*|.*lapack.*|.*blas.*|.*fortran.*|netlib-src|intel-mkl.*|accelerate-src)$'

found=$(bash "$(dirname "$0")/independent.sh" "$meta" echo "$forbidden" all)

if [ "$found" = "!no-root" ]; then
  echo "✖ echo-independent: no single package named echo in $meta" >&2
  exit 1
fi
if [ -n "$found" ]; then
  echo "✖ echo-independent: echo depends on forbidden packages:" >&2
  echo "$found" | sed 's/^/    /' >&2
  exit 1
fi
echo "✔ echo-independent: no prismqueer, mirror, LAPACK, BLAS or Fortran under echo"
