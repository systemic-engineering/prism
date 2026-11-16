#!/usr/bin/env bash
# Time each phase of the push gate, the way the hook runs it, and print a table.
#
# What it measures: wall seconds per phase, run through the same recipes the
# household pre-push hook calls (`just check`, `just pre-push`), so every
# per-recipe overhead (nix develop, nix eval) is counted where the hook pays it.
#
#   devshell   `nix develop -c true`: what each `nix develop -c` recipe pays
#              before cargo starts (flake eval + shell realisation)
#   eval       evaluating the coverage + doctest drvPaths (no build)
#   check      `just check` (fmt + clippy, one dev shell): what the hook runs
#              on a push to main
#   fmt        `just fmt-check` alone
#   clippy     `just lint` alone
#   test       `just test` (plain cargo; the dev loop, not the push gate)
#   coverage   `just test coverage` (= `just pre-push`): nix checks.coverage + doctest
#
# State, so a number is never read without its context:
#   cargo phases: the target dir's size before the phase, and how many units
#     cargo compiled or checked during it (0 = fully warm).
#   coverage: "hit" if nix had nothing to build (`--dry-run` lists no
#     derivation), else the count of derivations it would build.
#   load: the 1-minute load average when the phase started, and how many
#     other cargo/rustc/nix-build processes were running (contention).
#
# Usage: nix/bench-gate.sh [warm|cold] [phase...]
#   warm (default): the current CARGO_TARGET_DIR, as a push would find it.
#   cold: cargo phases run in a fresh, empty target dir (created under
#     target/, removed afterwards), so the existing target/ is not touched.
#     Nix's store can't be made cold without a GC; coverage reports hit/miss.
#   phases default to: devshell eval check coverage (the push gate, as the
#     hook runs it on main); add `test` to time the plain suite too.
#
# Timing lives here, in tooling. The library reads no clock.
set -uo pipefail

# Everything runs inside main, which bash parses whole before running it, so
# editing this file while a long run is in flight can't derail the run.
main() {
root=$(git rev-parse --show-toplevel)
cd "$root" || exit 2
mode=${1:-warm}
case "$mode" in warm | cold) shift || true ;; *) mode=warm ;; esac
phases=("$@")
((${#phases[@]})) || phases=(devshell eval check coverage)

# The justfile reads PRISM_TARGET_DIR (default <repo>/target).
export PRISM_TARGET_DIR=${PRISM_TARGET_DIR:-$root/target}
if [[ $mode == cold ]]; then
  PRISM_TARGET_DIR=$root/target/bench-cold-$$
  trap 'rm -rf "$PRISM_TARGET_DIR"' EXIT
fi
CARGO_TARGET_DIR=$PRISM_TARGET_DIR

sys=$(nix eval --impure --raw --expr builtins.currentSystem)
log=$(mktemp -d)/bench-gate
mkdir -p "$log"

now() { echo "${EPOCHREALTIME:-$(date +%s)}"; }
load() { sysctl -n vm.loadavg 2>/dev/null | awk '{print $2}' || uptime | sed 's/.*load averages*: //;s/[, ].*//'; }
others() { # cargo/rustc/clippy/gfortran processes running when a phase starts
  ps -axo comm= | awk '{c=$1; sub(".*/","",c)} c ~ /^(cargo|rustc|clippy-driver|gfortran|f951)$/' | wc -l | tr -d ' '
}
size() { [[ -d $CARGO_TARGET_DIR ]] && du -sm "$CARGO_TARGET_DIR" 2>/dev/null | cut -f1 | sed 's/$/M/' || echo absent; }
units() { grep -cE '^ *(Compiling|Checking) ' "$1" || true; }
# cargo's own "Finished ... in 1m 05s" and libtest's doctest "finished in 4.2s",
# so a test phase splits into compile / doctests / the rest (binaries + launch).
cargo_secs() { grep -E '^ *Finished ' "$1" | tail -1 | sed 's/.* in //' |
  awk '{s=0; for(i=1;i<=NF;i++){v=$i; if(v~/m$/){sub(/m$/,"",v); s+=v*60} else {sub(/s$/,"",v); s+=v}} printf "%.0f", s}'; }
doc_secs() { awk '/Doc-tests/{d=1} d && /finished in/{v=$NF; sub(/s$/,"",v); s+=v} END{printf "%.0f", s}' "$1"; }

run() { # run <phase>: prints "<seconds>\t<state>"
  local p=$1 out="$log/$1.log" t0 t1 rc state
  case $p in
    devshell) state=- ;;
    eval) state=- ;;
    check | fmt | clippy | test) state="target $(size)" ;;
    coverage)
      local n
      n=$(nix build --dry-run ".#checks.$sys.coverage" ".#checks.$sys.doctest" 2>&1 |
        awk '/will be built/{f=1;next} /will be fetched/{f=0} f && /\.drv/' | wc -l | tr -d ' ')
      if ((n == 0)); then state=hit; else state="miss ($n drv)"; fi ;;
  esac
  local l o
  l=$(load) o=$(others)
  # Live, on stderr: a long phase shows what it is waiting on, not silence.
  printf '[%s] %s (%s) ... log: %s\n' "$(date +%T)" "$p" "$state" "$out" >&2
  t0=$(now)
  case $p in
    devshell) nix develop -c true >"$out" 2>&1 ;;
    eval) nix eval --raw ".#checks.$sys.coverage.drvPath" >"$out" 2>&1 &&
      nix eval --raw ".#checks.$sys.doctest.drvPath" >>"$out" 2>&1 ;;
    check) just check >"$out" 2>&1 ;;
    fmt) just fmt-check >"$out" 2>&1 ;;
    clippy) just lint >"$out" 2>&1 ;;
    test) just test >"$out" 2>&1 ;;
    coverage) just test coverage >"$out" 2>&1 ;;
    *) echo "unknown phase $p" >"$out"; false ;;
  esac
  rc=$?
  t1=$(now)
  case $p in check | fmt | clippy | test) state="$state, $(units "$out") units" ;; esac
  case $p in check | clippy | test) state="$state, compile $(cargo_secs "$out")s" ;; esac
  [[ $p == test ]] && state="$state, doctests $(doc_secs "$out")s"
  ((rc == 0)) || state="$state, FAILED rc=$rc"
  printf '%s\t%.1f\t%s\t%s / %s\n' "$p" "$(echo "$t1 - $t0" | bc)" "$state" "$l" "$o"
}

printf 'bench-gate: %s, %s, %s @ %s\n' "$mode" "$root" "$sys" "$(git rev-parse --short HEAD)"
printf 'PRISM_TARGET_DIR=%s\n' "$PRISM_TARGET_DIR"
printf 'logs: %s\n\n' "$log"
{
  printf 'phase\tseconds\tstate\tload / other builds\n'
  total=0
  for p in "${phases[@]}"; do
    line=$(run "$p")
    echo "$line"
    total=$(echo "$total + $(cut -f2 <<<"$line")" | bc)
  done
  printf 'total\t%s\t\t\n' "$total"
} | column -t -s $'\t'
}

main "$@"
exit $?
