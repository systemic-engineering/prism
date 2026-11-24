#!/usr/bin/env bash
# Fail if the `prism-mcp` package depends on anything forbidden
# (docs/spec/mcp.md §6; prism-025 §1; Alex: "No serde.").
#
# prism-mcp serves prism's verifiers to mirror. It may not share code with
# what it serves, carry a JSON crate (the wire is hand-written), or carry
# prismqueer's LAPACK/BLAS/Fortran backend. Forbidden, by package name:
#   - prismqueer;
#   - any mirror crate (`mirror`, `mirror-*`, `mirror_*`);
#   - anything serde (*serde*: serde, serde_json, serde_core, serde_derive,
#     miniserde, nanoserde) and any JSON crate (*json*);
#   - anything LAPACK, BLAS or Fortran (*lapack*, *blas*, *fortran*,
#     netlib-src, intel-mkl*, accelerate-src), and any package whose `links`
#     key names lapack, blas or fortran.
#
# Every dependency kind counts from prism-mcp itself; below it, dev edges
# don't (nix/independent.sh, <dev> = root): chorus's own tests use
# serde_json, and that is never linked into prism-mcp.
#
# Usage: prism-mcp-independent.sh <metadata.json>
set -euo pipefail

meta=${1:?usage: prism-mcp-independent.sh <metadata.json>}
forbidden='^(prismqueer|mirror|mirror[-_].*|.*serde.*|.*json.*|.*lapack.*|.*blas.*|.*fortran.*|netlib-src|intel-mkl.*|accelerate-src)$'

found=$(bash "$(dirname "$0")/independent.sh" "$meta" prism-mcp "$forbidden" root)

if [ "$found" = "!no-root" ]; then
  echo "✖ prism-mcp-independent: no single package named prism-mcp in $meta" >&2
  exit 1
fi
if [ -n "$found" ]; then
  echo "✖ prism-mcp-independent: prism-mcp depends on forbidden packages:" >&2
  echo "$found" | sed 's/^/    /' >&2
  exit 1
fi
echo "✔ prism-mcp-independent: no serde, JSON crate, prismqueer, mirror, LAPACK, BLAS or Fortran under prism-mcp"
