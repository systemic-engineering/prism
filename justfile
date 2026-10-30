# prism — focus | project | refract

# Static analysis only: formatting + clippy. Fast; runs no tests.
check: fmt-check lint

# mode=coverage runs the suite once, instrumented, gated on line coverage,
# then the doctests (llvm-cov skips them).
#
# Ratchet: 94.08% of lines measured 2026-10-29 under lapack,fate. Raise it as
# coverage rises; never lower it.
#
# Run the suite: mode=plain (cargo test) or mode=coverage (the push gate)
test mode="plain":
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{mode}}" in
      plain)
        nix develop -c cargo test --workspace --features lapack,fate ;;
      coverage)
        nix develop -c cargo llvm-cov --workspace --features lapack,fate --fail-under-lines 94
        nix develop -c cargo test --doc --workspace --features lapack,fate ;;
      *)
        echo "✖ unknown test mode '{{mode}}' (plain | coverage)" >&2
        exit 2 ;;
    esac

# What the push hook runs: the suite once, under coverage.
pre-push: (test "coverage")

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
