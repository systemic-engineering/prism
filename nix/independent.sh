#!/usr/bin/env bash
# The independence walk (prism-022 S5; docs/spec/echo.md §2, docs/spec/mcp.md
# §6): fail if the closure of one package in a Cargo graph holds a forbidden
# package, by name or by `links`.
#
# Input: `cargo metadata --format-version 1` JSON. The graph is resolve.nodes.
#
# Usage: independent.sh <metadata.json> <root> <forbidden-name-regex> <dev>
#   <dev> = all:  every dependency kind (normal, build, dev) at every depth.
#   <dev> = root: every kind from the root; below it, normal and build only.
#                 A dependency's dev-dependencies build only that
#                 dependency's own tests and are never linked into a
#                 dependent. An edge with no dep_kinds counts as normal.
# A package whose `links` names lapack, blas or fortran is always refused:
# a native library is linked under the `links` name, whatever the crate is
# called.
set -euo pipefail

meta=${1:?usage: independent.sh <metadata.json> <root> <forbidden> <all|root>}
root=${2:?root package name}
forbidden=${3:?forbidden name regex}
dev=${4:?all or root}
native='lapack|blas|fortran'
case "$dev" in all | root) ;; *)
  echo "✖ independent: <dev> is all or root, not $dev" >&2
  exit 2
  ;;
esac

# The closure is a fixpoint over a frontier, not jq's recurse: a dev-dependency
# can close a cycle back to the root, and recurse would not terminate.
jq -r --arg root "$root" --arg re "$forbidden" --arg native "$native" --arg dev "$dev" '
  (.packages | map({key: .id, value: .name}) | from_entries) as $name
  | (.packages | map({key: .id, value: (.links // "")}) | from_entries) as $links
  | (.resolve.nodes | map({key: .id, value: .deps}) | from_entries) as $deps
  | [.packages[] | select(.name == $root) | .id] as $roots
  # The edges out of a package: all of them, or only normal and build
  # ones below the root.
  | def out($from):
      ($deps[$from] // [])[]
      | select($dev == "all" or ($roots | index($from)) != null
               or any((.dep_kinds // [{kind: null}])[]; .kind != "dev"))
      | .pkg;
    if ($roots | length) != 1 then "!no-root"
    else {seen: $roots, frontier: $roots}
      | until(.frontier | length == 0;
          ([.frontier[] | out(.)] | unique) as $next
          | ($next - .seen) as $new
          | {seen: (.seen + $new), frontier: $new})
      | .seen - $roots
      | map(if ($name[.] | test($re)) then $name[.]
            elif ($links[.] | test($native)) then "\($name[.]) (links \($links[.]))"
            else empty end)
      | unique | .[]
    end
' "$meta"
