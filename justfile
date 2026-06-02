# prism — focus | project | refract

# Run all tests
test:
    nix develop -c cargo test --workspace --features optics

# Run tests with coverage report
coverage:
    nix develop -c cargo llvm-cov --workspace --features optics

# Coverage gate — fails if below threshold
pre-push:
    nix develop -c cargo llvm-cov --workspace --features optics --fail-under-lines 99

# Build workspace
build:
    nix develop -c cargo build --workspace --features optics

# Clippy lint
lint:
    nix develop -c cargo clippy --workspace --features optics -- -D warnings

# Format check
fmt-check:
    nix develop -c cargo fmt --check

# Format
fmt:
    nix develop -c cargo fmt

# Full check (what CI and pre-push hook run)
check: test lint pre-push

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
    cargo test --workspace --features optics,pq
    echo "✔ merged $branch into main"
    echo "  next: \`git push origin main\` when ready"
