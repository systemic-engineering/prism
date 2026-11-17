#!/usr/bin/env bash
# Fail unless every SF: record in an lcov file names a repo-relative path.
#
# checks.coverage's output must be the same bytes on every rebuild. An
# absolute SF: path carries the build directory (on darwin,
# /nix/var/nix/builds/nix-<pid>-<n>/source/...), which moves per build, and
# nix then reports the coverage output as non-deterministic (checks.md, to-fix 5).
#
# A path under target/ is generated code (a build script's out/), not a
# source: absolute, cargo-llvm-cov's own filter drops it; made relative, it
# would slip past that filter and move the measured line count.
#
# Usage: lcov-paths.sh <file.lcov>
set -euo pipefail

lcov=${1:?usage: lcov-paths.sh <file.lcov>}

sf=$(grep '^SF:' "$lcov" || true)
if [ -z "$sf" ]; then
  echo "✖ lcov-paths: no SF: records in $lcov" >&2
  exit 1
fi

bad=$(printf '%s\n' "$sf" | grep -E '^SF:/|/nix/var/nix/builds|^SF:target/' || true)
if [ -n "$bad" ]; then
  echo "✖ lcov-paths: $(printf '%s\n' "$bad" | wc -l | tr -d ' ') of $(printf '%s\n' "$sf" | wc -l | tr -d ' ') SF: paths are absolute, name the build directory, or are under target/:" >&2
  printf '%s\n' "$bad" | head -5 >&2
  exit 1
fi

echo "✔ lcov-paths: $(printf '%s\n' "$sf" | wc -l | tr -d ' ') SF: paths, all repo-relative"
