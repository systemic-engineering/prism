#!/usr/bin/env bash
# The slowest tests, by wall time, so a slow test is seen when it lands.
#
# What it measures: libtest's own per-test exec_time (`--report-time`) for
# every test in every test binary of `cargo test --workspace --features
# lapack,fate` (the gate's feature set, unit and integration tests; doctests
# are not timed), plus each binary's wall time. Binaries run one after
# another, as `cargo test` runs them; tests inside a binary run on
# `threads` threads (default: RUST_TEST_THREADS, else libtest's default), so
# with threads > 1 a test's time includes contention from its neighbours.
# Use threads=1 for clean per-test numbers.
#
# The binaries are built once (`--no-run`), in the justfile's target dir, then
# run directly, so nothing is rebuilt to get the timings. RUSTC_BOOTSTRAP is
# set only on the test binaries' environment (libtest's --report-time is
# unstable); it never reaches rustc, so no fingerprint moves.
#
# Usage: nix/bench-suite.sh [top=25] [threads] [filter]
#   Runs inside the dev shell (`just bench-suite` puts it there).
# Timing lives here, in tooling. The library reads no clock.
set -euo pipefail

# Parsed whole before it runs, so an edit mid-run can't derail it.
main() {
top=${1:-25}
threads=${2:-${RUST_TEST_THREADS:-}}
filter=${3:-}
root=$(git rev-parse --show-toplevel)
cd "$root"
export OPENBLAS_NUM_THREADS=1
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

echo "→ building test binaries (cargo test --no-run)" >&2
cargo test --workspace --features lapack,fate --no-run --message-format=json 2>/dev/null |
  jq -r 'select(.reason == "compiler-artifact" and .profile.test and .executable != null)
         | [.executable, (.manifest_path | sub("/Cargo.toml$"; "")), .target.name, (.target.kind | join(","))] | @tsv' \
    >"$work/bins"

args=(-Z unstable-options --format json --report-time)
[[ -n $threads ]] && args+=(--test-threads "$threads")
[[ -n $filter ]] && args+=("$filter")

now() { echo "${EPOCHREALTIME:-$(date +%s)}"; }
# As cargo runs them: proc-macro crates' test binaries load the toolchain's
# dylibs from the sysroot.
sysroot_lib="$(rustc --print sysroot)/lib"
export DYLD_FALLBACK_LIBRARY_PATH="$sysroot_lib${DYLD_FALLBACK_LIBRARY_PATH:+:$DYLD_FALLBACK_LIBRARY_PATH}"
export LD_LIBRARY_PATH="$sysroot_lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
printf 'binary\tkind\ttests\tseconds\n' >"$work/walls"
while IFS=$'\t' read -r exe dir name kind; do
  t0=$(now)
  (cd "$dir" && CARGO_MANIFEST_DIR=$dir RUSTC_BOOTSTRAP=1 "$exe" "${args[@]}" 2>/dev/null) \
    >"$work/$name-$kind.json" || echo "✗ $name ($kind) had failures" >&2
  t1=$(now)
  n=$(jq -r 'select(.type == "test" and .event != "started") | .name' "$work/$name-$kind.json" 2>/dev/null | wc -l | tr -d ' ')
  printf '%s\t%s\t%s\t%.2f\n' "$name" "$kind" "$n" "$(echo "$t1 - $t0" | bc)" >>"$work/walls"
  jq -r --arg b "$name" 'select(.type == "test" and .exec_time != null)
        | [.exec_time, $b, .name, .event] | @tsv' "$work/$name-$kind.json" 2>/dev/null >>"$work/tests" || true
done <"$work/bins"

echo
echo "slowest $top tests (libtest exec_time, threads=${threads:-default})"
{
  printf 'seconds\tbinary\ttest\tresult\n'
  sort -t$'\t' -k1,1 -rg "$work/tests" | awk -F'\t' -v n="$top" 'NR <= n {printf "%.3f\t%s\t%s\t%s\n", $1, $2, $3, $4}'
} | column -t -s $'\t'

echo
echo "test binaries by wall time"
{
  head -1 "$work/walls"
  tail -n +2 "$work/walls" | sort -t$'\t' -k4,4 -rg
  awk -F'\t' 'NR > 1 {s += $4; n += $3} END {printf "total\t\t%d\t%.2f\n", n, s}' "$work/walls"
} | column -t -s $'\t'
}

main "$@"
exit $?
