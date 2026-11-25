#!/usr/bin/env bash
# The floor's coverage gate (docs/spec/coverage.md): the lines under <prefix>
# in an lcov file, minus the lines a source marker skips, must all be
# executed, and the skipped lines must number exactly the ratchet.
#
# Markers, each a comment on a line of its own:
#   // coverage: skip(<reason>)        skips the next line
#   // coverage: skip-start(<reason>)  skips every line up to
#   // coverage: skip-end
# The reason is required and non-empty.
#
# Refused, each a failure:
#   - a marker without a reason, a marker that doesn't parse, a marker after
#     code on the same line;
#   - a skip-start inside a region, a skip inside a region, a skip-end with
#     no region open, a region still open at the end of the file, a skip
#     whose next line is another marker;
#   - a skip with no instrumented line under it ("no lines under it");
#   - a skip over a line that ran ("skip covers executed code"): a skip
#     names code no test reaches, so an executed line inside one makes its
#     reason a lie, wherever the region's edges fall;
#   - an instrumented line under <prefix> that didn't run and isn't skipped;
#   - more skipped lines than the ratchet holds, or fewer (lower it).
#
# The ledger, one line per skip (file:first-last, lines skipped, reason),
# goes to stdout; the verdict and every failure to stderr.
#
# Usage: coverage-skips.sh <file.lcov> <root> <prefix> <max-file>
#   <root>     the directory the lcov's SF: paths are relative to
#   <prefix>   the source directory gated, relative to <root> (laws/src)
#   <max-file> the ratchet: the number of skipped lines, and nothing else
set -euo pipefail

lcov=${1:?usage: coverage-skips.sh <file.lcov> <root> <prefix> <max-file>}
root=${2:?usage: coverage-skips.sh <file.lcov> <root> <prefix> <max-file>}
prefix=${3:?usage: coverage-skips.sh <file.lcov> <root> <prefix> <max-file>}
maxfile=${4:?usage: coverage-skips.sh <file.lcov> <root> <prefix> <max-file>}
prefix=${prefix%/}

max=$(tr -d ' \t\n' < "$maxfile")
case "$max" in
  '' | *[!0-9]*)
    echo "✖ coverage-skips: the ratchet $maxfile holds '$max', not a count of lines" >&2
    exit 1
    ;;
esac

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# file<TAB>line<TAB>count, for every DA: record under the prefix.
awk -v p="$prefix/" '
  /^SF:/ { f = substr($0, 4); keep = (index(f, p) == 1); next }
  keep && /^DA:/ { split(substr($0, 4), a, ","); print f "\t" a[1] "\t" a[2] }
' "$lcov" > "$work/da"
if [ ! -s "$work/da" ]; then
  echo "✖ coverage-skips: no DA: records under $prefix/ in $lcov" >&2
  exit 1
fi

# Every source under the prefix, measured or not: a marker in a file the
# lcov doesn't name has no lines under it.
{
  cut -f1 "$work/da"
  (cd "$root" && find "$prefix" -name '*.rs' -type f)
} | sort -u > "$work/files"
while read -r f; do
  if [ ! -f "$root/$f" ]; then
    echo "✖ coverage-skips: $f is in the lcov but not under $root" >&2
    exit 1
  fi
done < "$work/files"

# Word-split on purpose: the paths are repo-relative and hold no spaces.
# shellcheck disable=SC2046
set -- $(cat "$work/files")
status=0
(cd "$root" && awk -v da="$work/da" -v missfile="$work/missed" -v max="$max" -v prefix="$prefix" '
  function fail(msg) { print "✖ coverage-skips: " msg > "/dev/stderr"; failed = 1 }
  function reason(s,   r) {
    r = s
    sub(/^[^(]*\(/, "", r)
    sub(/\)[ \t]*$/, "", r)
    gsub(/^[ \t]+|[ \t]+$/, "", r)
    return r
  }
  # Close a skip over lines [a, b] of file f.
  function skip(f, a, b, why, at,   l, k, n, ran) {
    n = 0; ran = ""
    for (l = a; l <= b; l++) {
      k = f SUBSEP l
      if (!(k in count)) continue
      if (count[k] > 0) ran = ran " " l
      else { n++; skipped[k] = 1 }
    }
    if (ran != "") fail("skip covers executed code: " f ":" at " (" why "): line" ran " ran")
    else if (n == 0) fail("skip with no lines under it: " f ":" at " (" why ")")
    else { ledger[++entries] = f ":" (a == b ? a : a "-" b) "\t" n "\t" why; total += n }
  }
  function finish() {
    if (file == "") return
    if (open) fail("skip-start without a skip-end: " file ":" ostart " (" oreason ")")
    if (pending) fail("skip at the end of the file, no line under it: " file ":" (pending - 1))
    open = 0; pending = 0
  }
  BEGIN {
    while ((getline row < da) > 0) {
      split(row, c, "\t")
      k = c[1] SUBSEP c[2]
      if (!(k in count) || c[3] + 0 > count[k]) count[k] = c[3] + 0
    }
    close(da)
  }
  FNR == 1 { finish(); file = FILENAME }
  {
    line = $0
    marker = (line ~ /^[ \t]*\/\/[ \t]*coverage:/)
    if (!marker && line ~ /\/\/[ \t]*coverage:/) {
      fail("a coverage marker after code, not on its own line: " file ":" FNR)
      next
    }
    if (pending && pending == FNR) {
      if (marker) fail("skip followed by another marker: " file ":" (FNR - 1))
      else skip(file, FNR, FNR, preason, FNR - 1)
      pending = 0
    }
    if (!marker) next
    s = line
    sub(/^[ \t]*\/\/[ \t]*coverage:[ \t]*/, "", s)
    if (s ~ /^skip-end[ \t]*$/) {
      if (!open) fail("skip-end without a skip-start: " file ":" FNR)
      else { skip(file, ostart + 1, FNR - 1, oreason, ostart); open = 0 }
    } else if (s ~ /^skip-start\(.*\)[ \t]*$/ || s ~ /^skip\(.*\)[ \t]*$/) {
      r = reason(s)
      region = (s ~ /^skip-start/)
      if (r == "") { fail("a coverage marker without a reason: " file ":" FNR); next }
      if (open) { fail("a skip inside the skip region from " file ":" ostart ": " file ":" FNR); next }
      if (region) { open = 1; ostart = FNR; oreason = r }
      else { pending = FNR + 1; preason = r }
    } else if (s ~ /^skip(-start)?[ \t]*(\([ \t]*\))?[ \t]*$/) {
      fail("a coverage marker without a reason: " file ":" FNR)
    } else {
      fail("a coverage marker that does not parse: " file ":" FNR ": " line)
    }
  }
  END {
    finish()
    found = 0; hit = 0; missed = 0
    for (k in count) {
      if (k in skipped) continue
      found++
      if (count[k] > 0) hit++
      else { split(k, fl, SUBSEP); miss[++missed] = fl[1] ":" fl[2] }
    }
    for (i = 1; i <= entries; i++) print ledger[i]
    if (missed > 0) {
      fail(prefix ": " hit "/" found " lines executed; " missed " neither executed nor skipped:")
      for (i = 1; i <= missed; i++) print miss[i] > missfile
    }
    if (total > max) fail(total " skipped lines exceed the ratchet of " max)
    if (total < max) fail(total " skipped lines, under the ratchet of " max ": lower it to " total)
    if (failed) exit 1
    printf "✔ coverage-skips: %s: %d/%d lines executed (100%%), %d skipped in %d skips (ratchet %d)\n", prefix, hit, found, total, entries, max > "/dev/stderr"
  }
' "$@") > "$work/ledger" || status=$?

sort "$work/ledger"
if [ -s "$work/missed" ]; then
  sort -t: -k1,1 -k2,2n "$work/missed" | head -40 | sed 's/^/    /' >&2
  n=$(wc -l < "$work/missed" | tr -d ' ')
  if [ "$n" -gt 40 ]; then echo "    ... and $((n - 40)) more" >&2; fi
fi
exit "$status"
