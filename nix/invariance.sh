#!/usr/bin/env bash
# Cache-boundary invariance, end to end.
#
# The claim: prism's packages are a function of the crate sources only. A
# change to README.md, docs/, the justfile, the flake's own text or a test
# must leave every package's derivation unchanged, so nix (and any binary
# cache) serves the old build. A change to crate source must change it, or
# the claim is vacuous. Tests feed the checks, not the packages, so a test
# edit must move checks.test and leave the packages alone.
#
# Method: copy the tracked tree (submodules included) to a temp dir,
# evaluate drvPaths, mutate one file, evaluate again, compare. Evaluation
# only; nothing is built.
#
# Usage: nix/invariance.sh [repo-root]    (default: the git toplevel)
set -euo pipefail

root="${1:-$(git rev-parse --show-toplevel)}"
system=$(nix eval --impure --raw --expr builtins.currentSystem)
work=$(cd "$(mktemp -d)" && pwd -P)
trap 'rm -rf "$work"' EXIT
mkdir "$work/tree"

(cd "$root" && git ls-files -z --recurse-submodules | tar --null -T - -cf -) |
  tar -xf - -C "$work/tree"
[[ -f $work/tree/Cargo.lock && -f $work/tree/imperfect/Cargo.toml ]] ||
  { echo "✗ copy of the tree is incomplete"; exit 2; }

fail=0
ev() { # ev <attr> -> drvPath, or the eval error marker
  nix eval --raw "path:$work/tree#$1.drvPath" 2>/dev/null || echo "EVAL-FAILED:$1"
}
snapshot() {
  for a in packages.$system.prismqueer packages.$system.terni checks.$system.test; do
    printf '%s %s\n' "$a" "$(ev "$a")"
  done
}
expect() { # expect <what> <attr> <same|moved> <before> <after>
  local b a
  b=$(grep "^$2 " <<<"$4" | cut -d' ' -f2)
  a=$(grep "^$2 " <<<"$5" | cut -d' ' -f2)
  if [[ $b == EVAL-FAILED* || $a == EVAL-FAILED* ]]; then
    echo "✗ $1: $2 does not evaluate"; fail=1; return
  fi
  if [[ $3 == same && $b != "$a" ]]; then
    echo "✗ $1: $2 moved ($b -> $a)"; fail=1
  elif [[ $3 == moved && $b == "$a" ]]; then
    echo "✗ $1: $2 did not move (the check is vacuous)"; fail=1
  else
    echo "✓ $1: $2 $3"
  fi
}
mutate() { # mutate <label> <file> <expectations...>
  local before after label=$1 file=$2; shift 2
  before=$(snapshot)
  mkdir -p "$(dirname "$work/tree/$file")"
  printf '\n// invariance probe\n' >>"$work/tree/$file"
  after=$(snapshot)
  while (($#)); do expect "$label" "$1" "$2" "$before" "$after"; shift 2; done
}

P=packages.$system.prismqueer T=packages.$system.terni C=checks.$system.test
mutate "README.md"        README.md                 $P same  $T same  $C same
mutate "docs/"            docs/architecture.md      $P same  $T same  $C same
mutate "justfile"         justfile                  $P same  $T same  $C same
mutate "a test"           prismqueer/tests/music.rs $P same  $T same  $C moved
mutate "crate source"     prismqueer/src/lib.rs     $P moved $T same  $C moved
mutate "terni source"     imperfect/src/lib.rs      $P moved $T moved $C moved

# Every system evaluates without building anything first: no import from
# derivation. IFD builds silently on the host and fails for any other system
# (CI, fly, a Linux builder), and it serialises evaluation behind builds.
if nix flake check --no-build --all-systems --option allow-import-from-derivation false \
  "path:$work/tree" >/dev/null 2>&1; then
  echo "✓ all systems evaluate without import-from-derivation"
else
  echo "✗ evaluation needs import-from-derivation (or fails) on some system"; fail=1
fi

exit $fail
