# prism — focus | project | refract

# Build into prism's own target/ (gitignored), not the shared per-user target
# dir. Launching test binaries out of a deps/ dir holding ~600k files from
# every project cost 17-27s per binary (measured 2026-10-30); here it's <1s.
export CARGO_TARGET_DIR := justfile_directory() / "target"

# Static analysis only: formatting + clippy. Fast; runs no tests.
check: fmt-check lint

# mode=plain is the dev loop: cargo, incremental, in target/.
#
# mode=coverage is the push gate: the flake's checks.coverage (the suite once,
# instrumented, gated on line coverage) and checks.doctest (llvm-cov skips
# doctests). They are nix derivations over the tracked sources, so a push whose
# crate sources were already gated (a docs edit, a merge of a tested branch)
# is a cache hit: about 5s instead of a full run. The ratchet and the feature
# set live in flake.nix (coverageFloor, features). Raise the ratchet as
# coverage rises; never lower it.
#
# OPENBLAS_NUM_THREADS=1: tests already run in parallel; OpenBLAS threading
# inside each dsyev oversubscribes the cores and made the suite slower. The
# flake sets the same for its checks.
#
# Run the suite: mode=plain (cargo test) or mode=coverage (the push gate)
test mode="plain":
    #!/usr/bin/env bash
    set -euo pipefail
    export OPENBLAS_NUM_THREADS=1
    case "{{mode}}" in
      plain)
        nix develop -c cargo test --workspace --features lapack,fate ;;
      coverage)
        sys=$(nix eval --impure --raw --expr builtins.currentSystem)
        nix build --no-link -L ".#checks.$sys.coverage" ".#checks.$sys.doctest" ;;
      *)
        echo "✖ unknown test mode '{{mode}}' (plain | coverage)" >&2
        exit 2 ;;
    esac

# What the push hook runs: the suite once, under coverage.
pre-push: (test "coverage")

# Every check the flake defines (fmt, clippy, test, coverage, doctest,
# invariance): what CI runs.
flake-check:
    nix flake check -L

# Docs, the justfile and tests stay outside the packages' sources (prism-009).
invariance:
    nix/invariance.sh

# Build workspace
build:
    nix develop -c cargo build --workspace --features lapack,fate

# Clippy lint
lint:
    nix develop -c cargo clippy --workspace --features lapack,fate -- -D warnings

# Format check
fmt-check:
    nix develop -c cargo fmt --check

# Format
fmt:
    nix develop -c cargo fmt

# Merge the current branch into main.
#
# - Refuses if on main, or if working tree is dirty.
# - Fast-forwards if possible; falls back to --no-ff merge commit.
# - Runs the test suite after the merge.
# - prism is a library; no binary to reinstall. Downstream crates pick up
#   changes via path deps on next compile.
# - Push stays explicit — run `git push origin main` when ready.
merge:
    #!/usr/bin/env bash
    set -euo pipefail
    branch=$(git rev-parse --abbrev-ref HEAD)
    if [ "$branch" = "main" ]; then
        echo "✖ error: already on main" >&2
        exit 1
    fi
    dirty=$(git status --porcelain --ignore-submodules=all | grep -v '^?? ' || true)
    if [ -n "$dirty" ]; then
        echo "✖ error: working tree dirty. Commit or stash first." >&2
        git status --short >&2
        exit 1
    fi
    echo "→ merging $branch into main"
    git checkout main
    git pull --ff-only origin main
    if ! git merge --ff-only "$branch" 2>/dev/null; then
        echo "→ ff-only failed; creating merge commit"
        git merge --no-ff --no-gpg-sign "$branch" -m "🔀 merge $branch into main"
    fi
    echo "→ running tests"
    cargo test --workspace --features lapack,fate,pq
    echo "✔ merged $branch into main"
    echo "  next: \`git push origin main\` when ready"
