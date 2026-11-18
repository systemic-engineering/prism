#!/usr/bin/env bash
# Fail if the `listen` package depends on anything forbidden (prism-022 S5).
#
# listen is the second reader of mirror's output; it may not share code with
# what it checks, nor carry prismqueer's LAPACK/BLAS/Fortran backend into the
# verifier path. Forbidden, by package name:
#   - prismqueer;
#   - any mirror crate (`mirror`, `mirror-*`, `mirror_*`);
#   - anything LAPACK, BLAS or Fortran (by name: *lapack*, *blas*,
#     *fortran*, netlib-src, intel-mkl*, accelerate-src).
#
# Input: `cargo metadata --format-version 1` JSON. The graph is resolve.nodes:
# every dependency kind (normal, build, dev) and every target counts.
#
# Usage: listen-independent.sh <metadata.json>
set -euo pipefail

meta=${1:?usage: listen-independent.sh <metadata.json>}
forbidden='^(prismqueer|mirror|mirror[-_].*|.*lapack.*|.*blas.*|.*fortran.*|netlib-src|intel-mkl.*|accelerate-src)$'

found=$(jq -r --arg re "$forbidden" '
  (.packages | map({key: .id, value: .name}) | from_entries) as $name
  | (.resolve.nodes | map({key: .id, value: [.deps[].pkg]}) | from_entries) as $deps
  | [.packages[] | select(.name == "listen") | .id] as $roots
  | if ($roots | length) != 1 then "!no-listen"
    else ($deps[$roots[0]] // [])
      | map($name[.]) | unique | .[] | select(test($re))
    end
' "$meta")

if [ "$found" = "!no-listen" ]; then
  echo "✖ listen-independent: no single package named listen in $meta" >&2
  exit 1
fi
if [ -n "$found" ]; then
  echo "✖ listen-independent: listen depends on forbidden packages:" >&2
  echo "$found" | sed 's/^/    /' >&2
  exit 1
fi
echo "✔ listen-independent: no prismqueer, mirror, LAPACK, BLAS or Fortran under listen"
