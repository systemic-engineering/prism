#!/usr/bin/env bash
# Fail if the `prism-mcp` package's own build depends on anything forbidden
# (docs/spec/mcp.md §6; prism-025 §1; Alex: "No serde.").
#
# prism-mcp serves prism's verifiers to mirror. It may not share code with
# what it serves, carry a JSON crate (the wire is hand-written), or carry
# prismqueer's LAPACK/BLAS/Fortran backend. Forbidden, by package name:
#   - any mirror crate (`mirror`, `mirror-*`, `mirror_*`);
#   - anything serde (*serde*: serde, serde_json, serde_core, serde_derive,
#     miniserde, nanoserde) and any JSON crate (*json*);
#   - anything LAPACK, BLAS or Fortran (*lapack*, *blas*, *fortran*,
#     netlib-src, intel-mkl*, accelerate-src), and any package whose `links`
#     key names lapack, blas or fortran;
#   - prismqueer with any feature set but exactly `exact` (no `default`, no
#     `lapack`): its certified path, held by checks.prismqueer-exact.
#
# The graph is prism-mcp's own resolve, not the workspace's. `cargo
# metadata` resolves features once for the whole workspace, dev-dependencies
# included: prismqueer's dev-dependency on terni turns terni's `serde` on in
# that union, and the union shows terni → serde under prism-mcp although
# `cargo build -p prism-mcp` never builds it. `cargo tree -p prism-mcp`
# resolves for prism-mcp alone, as cargo build and cargo test -p prism-mcp
# do: prism-mcp's own dev edges, never its dependencies', and each package
# with the features that build turns on. `--target all` keeps every
# platform's edges. `links` is a manifest key, the same in any resolve, so it
# comes from the metadata, joined by name and version.
#
# The tree is read strictly: the first line must be prism-mcp itself, every
# line must parse, and every package must be in the metadata. Anything else
# is refused, so an empty or truncated tree never passes.
#
# Inputs:
#   <tree>      cargo tree --locked --offline -p prism-mcp -e normal,build,dev
#               --target all --prefix none -f '{p}|{f}'
#   <metadata>  cargo metadata --format-version 1 (only .packages is read)
#
# Usage: prism-mcp-independent.sh <tree> <metadata.json>
set -euo pipefail

tree=${1:?usage: prism-mcp-independent.sh <tree> <metadata.json>}
meta=${2:?usage: prism-mcp-independent.sh <tree> <metadata.json>}
forbidden='^(mirror|mirror[-_].*|.*serde.*|.*json.*|.*lapack.*|.*blas.*|.*fortran.*|netlib-src|intel-mkl.*|accelerate-src)$'

# One line per package node: `name vVERSION[ (proc-macro)][ (source)]|features`,
# then ` (*)` when cargo has printed that node already.
found=$(jq -r -R -s --slurpfile meta "$meta" --arg re "$forbidden" --arg native 'lapack|blas|fortran' '
  ($meta[0].packages | map({key: "\(.name) \(.version)", value: (.links // "")}) | from_entries) as $links
  | split("\n") | map(select(length > 0))
  | map(. as $line
        | (capture("^(?<name>[A-Za-z0-9_-]+) v(?<version>[0-9A-Za-z.+-]+)( \\(proc-macro\\))?( \\([^()]*\\))?\\|(?<features>[^ ]*)( \\(\\*\\))?$")
           // {bad: $line})) as $nodes
  | if ($nodes | length) == 0 or $nodes[0].name != "prism-mcp" then "!no-root"
    else $nodes[1:][]
      | "\(.name) \(.version)" as $key
      | if .bad then "unparsed: \(.bad)"
        elif ($links | has($key) | not) then "\($key) (not in the metadata)"
        elif (.name | test($re)) then .name
        elif ($links[$key] | test($native)) then "\(.name) (links \($links[$key]))"
        elif .name == "prismqueer" and .features != "exact" then "prismqueer (features \(.features))"
        else empty end
    end
' "$tree" | sort -u)

if [ "$found" = "!no-root" ]; then
  echo "✖ prism-mcp-independent: $tree does not start with prism-mcp" >&2
  exit 1
fi
if [ -n "$found" ]; then
  echo "✖ prism-mcp-independent: prism-mcp's build depends on forbidden packages:" >&2
  echo "$found" | sed 's/^/    /' >&2
  exit 1
fi
echo "✔ prism-mcp-independent: no serde, JSON crate, mirror, LAPACK, BLAS or Fortran in prism-mcp's build; prismqueer only under exact"
