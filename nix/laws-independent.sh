#!/usr/bin/env bash
# Fail if the `prism-laws` package's own build depends on anything forbidden
# (docs/spec/laws.md, "Independence"; mirror-034).
#
# prism-laws is prism's floor: what a verdict rests on. It may not carry a
# proc-macro, a serialisation or JSON crate, mirror, the lab, or a native
# numeric backend. Forbidden:
#   - any proc-macro package: a package with a target of kind `proc-macro`
#     in the metadata (syn and quote come only under one; prismqueer-
#     projections and terni-macros are ones);
#   - anything serde (*serde*) and any JSON crate (*json*);
#   - any mirror crate (`mirror`, `mirror-*`, `mirror_*`) and prismqueer
#     (`prismqueer`, `prismqueer-*`): the lab;
#   - anything LAPACK, BLAS or Fortran (*lapack*, *blas*, *fortran*,
#     netlib-src, intel-mkl*, accelerate-src), and any package whose `links`
#     key names lapack, blas or fortran;
#   - `cc` reached through any package but blake3: blake3's build-dependency
#     compiles blake3's own SIMD, as in checks.prismqueer-exact; every other
#     path to a C compiler is refused.
#
# The graph is prism-laws's own resolve, `cargo tree -p prism-laws`, as
# prism-mcp's guard reads prism-mcp's (docs/spec/mcp.md §6.1): prism-laws's
# own dev edges, never its dependencies', each package with the features
# that build turns on, every platform. `--prefix depth` gives each line its
# depth, so each package's parent is the nearest line above it one level up:
# that is how cc's edge is told from blake3's. `links` and proc-macro
# targets are manifest facts, the same in any resolve, so they come from the
# metadata, joined by name and version.
#
# The tree is read strictly: the first line must be prism-laws at depth 0,
# every line must parse, each depth at most one below the line before, and
# every package must be in the metadata. Anything else is refused, so an
# empty or truncated tree never passes.
#
# Inputs:
#   <tree>      cargo tree --locked --offline -p prism-laws -e normal,build,dev
#               --target all --prefix depth -f '{p}|{f}'
#   <metadata>  cargo metadata --format-version 1 (only .packages is read)
#
# Usage: laws-independent.sh <tree> <metadata.json>
set -euo pipefail

tree=${1:?usage: laws-independent.sh <tree> <metadata.json>}
meta=${2:?usage: laws-independent.sh <tree> <metadata.json>}
forbidden='^(prismqueer|prismqueer[-_].*|mirror|mirror[-_].*|.*serde.*|.*json.*|.*lapack.*|.*blas.*|.*fortran.*|netlib-src|intel-mkl.*|accelerate-src)$'

# One line per package node: `DEPTHname vVERSION[ (proc-macro)][ (source)]|features`,
# then ` (*)` when cargo has printed that node already.
found=$(jq -r -R -s --slurpfile meta "$meta" --arg re "$forbidden" --arg native 'lapack|blas|fortran' '
  ($meta[0].packages
    | map({key: "\(.name) \(.version)",
           value: {links: (.links // ""),
                   macro: any(.targets[]?; any(.kind[]?; . == "proc-macro"))}})
    | from_entries) as $pkg
  | split("\n") | map(select(length > 0))
  | map(. as $line
        | (capture("^(?<depth>[0-9]+)(?<name>[A-Za-z0-9_-]+) v(?<version>[0-9A-Za-z.+-]+)( \\(proc-macro\\))?( \\([^()]*\\))?\\|(?<features>[^ ]*)( \\(\\*\\))?$")
           | .depth |= tonumber)
          // {bad: $line}) as $nodes
  | if ($nodes | length) == 0 or $nodes[0].bad or $nodes[0].name != "prism-laws" or $nodes[0].depth != 0
    then "!no-root"
    else
      # Each node with its parent: the stack of names by depth, walked down
      # the tree in order.
      (reduce $nodes[1:][] as $n ({stack: [$nodes[0].name], out: []};
        if $n.bad then .out += [$n]
        elif $n.depth < 1 or $n.depth > (.stack | length) then .out += [{bad: "depth \($n.depth) after \(.stack | length - 1): \($n.name)"}]
        else .stack = (.stack[0:$n.depth] + [$n.name])
          | .out += [$n + {parent: .stack[$n.depth - 1]}]
        end) | .out)[]
      | "\(.name) \(.version)" as $key
      | if .bad then "unparsed: \(.bad)"
        elif ($pkg | has($key) | not) then "\($key) (not in the metadata)"
        elif (.name | test($re)) then .name
        elif ($pkg[$key].links | test($native)) then "\(.name) (links \($pkg[$key].links))"
        elif $pkg[$key].macro then "\(.name) (proc-macro)"
        elif .name == "cc" and .parent != "blake3" then "cc (through \(.parent))"
        else empty end
    end
' "$tree" | sort -u)

if [ "$found" = "!no-root" ]; then
  echo "✖ laws-independent: $tree does not start with prism-laws" >&2
  exit 1
fi
if [ -n "$found" ]; then
  echo "✖ laws-independent: prism-laws's build depends on forbidden packages:" >&2
  echo "$found" | sed 's/^/    /' >&2
  exit 1
fi
echo "✔ laws-independent: no proc-macro, serde, JSON crate, mirror, prismqueer, LAPACK, BLAS or Fortran in prism-laws's build; cc only under blake3"
