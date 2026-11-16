# prism — focus | project | refract

# Build into prism's own target/ (gitignored), not the shared per-user target
# dir. Launching test binaries out of a deps/ dir holding ~600k files from
# every project cost 17-27s per binary (measured 2026-10-30); here it's <1s.
# PRISM_TARGET_DIR overrides it (nix/bench-gate.sh's cold runs use an empty one).
export CARGO_TARGET_DIR := env("PRISM_TARGET_DIR", justfile_directory() / "target")

# Static analysis only: formatting + clippy. Fast; runs no tests.
#
# The same two commands as `fmt-check` and `lint`, in one dev shell: each
# `nix develop -c` costs 3-6s of evaluation before cargo starts (Taut,
# 2026-11-16), and the hook runs `check` on every push to main.
check:
    @echo "[$(date +%T)] check: fmt + clippy (warm: seconds; cold target/: minutes)"
    nix develop -c bash -c 'cargo fmt --check && cargo clippy --workspace --all-targets --features lapack,fate -- --deny warnings'
    @echo "[$(date +%T)] check: done"

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
        # Say which it is before it starts: a hit takes seconds, a miss builds
        # and runs the instrumented suite (minutes), and the two must not look
        # alike. -L streams the build log.
        echo "[$(date +%T)] coverage: $(nix build --dry-run ".#checks.$sys.coverage" ".#checks.$sys.doctest" 2>&1 |
          grep -q 'will be built' && echo 'cache miss, building' || echo 'cache hit')"
        nix build --no-link -L ".#checks.$sys.coverage" ".#checks.$sys.doctest"
        echo "[$(date +%T)] coverage: done" ;;
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

# Clippy lint: every target (lib, tests, benches, examples), as the flake's
# checks.clippy does.
lint:
    nix develop -c cargo clippy --workspace --all-targets --features lapack,fate -- --deny warnings

# Format check
fmt-check:
    nix develop -c cargo fmt --check

# Format
fmt:
    nix develop -c cargo fmt

# ── Benchmarks: where the minutes go ─────────────────────────────────────────
# Recipes time things; the library reads no clock.

# Time each gate phase, the way the push hook runs it, and print a table with
# its state (target size, units compiled, nix cache hit or miss, load and
# other builds running). `just bench-gate` = devshell eval check coverage;
# `just bench-gate cold check test` runs the cargo phases in an empty target
# dir (existing target/ untouched). See nix/bench-gate.sh.
bench-gate *args:
    nix/bench-gate.sh {{args}}

# The slowest tests by libtest's own exec_time, and every test binary's wall
# time, so a slow test shows up when it lands. threads=1 for clean per-test
# numbers; filter as for cargo test. See nix/bench-suite.sh.
bench-suite top="25" threads="" filter="":
    nix develop -c nix/bench-suite.sh "{{top}}" "{{threads}}" "{{filter}}"

# The criterion benches (prismqueer beam_pipeline, terni imperfect).
bench filter="":
    OPENBLAS_NUM_THREADS=1 nix develop -c cargo bench --workspace --features lapack,fate -- {{filter}}

# ── Worktrees ────────────────────────────────────────────────────────────────

# A builder's worktree, warm: <main checkout>/.worktrees/<name> on a new
# branch, terni checked out (the submodule's local clone needs file transport),
# and target/ seeded from this checkout's (seed-target).
worktree name branch=name base="HEAD":
    #!/usr/bin/env bash
    set -euo pipefail
    main=$(git worktree list --porcelain | sed -n '1s/^worktree //p')
    dir="$main/.worktrees/{{name}}"
    git worktree add -b "{{branch}}" "$dir" "{{base}}"
    git -C "$dir" -c protocol.file.allow=always submodule update --init
    just --justfile "$dir/justfile" seed-target "{{justfile_directory()}}/target"
    echo "✔ $dir on {{branch}}"

# Seed an empty target/ with another checkout's compiled dependencies (default:
# the main checkout's), by copy-on-write clone: no disk until a file changes.
# Cargo keys third-party crates by name, version and flags, not by checkout
# path, so they come over fresh; the workspace's own crates recompile. A fresh
# worktree's first clippy went from 100-138s to 39s, its test build from
# 166-187s to 138s (Taut, 2026-11-16). Never touches an existing target/debug.
seed-target from="":
    #!/usr/bin/env bash
    set -euo pipefail
    from="{{from}}"
    [ -n "$from" ] || from="$(git worktree list --porcelain | sed -n '1s/^worktree //p')/target"
    to="{{justfile_directory()}}/target"
    if [ -e "$to/debug" ]; then echo "target/debug exists; not seeding"; exit 0; fi
    if [ "$(cd "$from/.." && pwd -P)" = "$(cd "{{justfile_directory()}}" && pwd -P)" ]; then echo "nothing to seed from"; exit 0; fi
    mkdir -p "$to/debug"
    for d in deps build .fingerprint; do
      [ -d "$from/debug/$d" ] || continue
      /bin/cp -cRp "$from/debug/$d" "$to/debug/" 2>/dev/null || cp -Rp --reflink=auto "$from/debug/$d" "$to/debug/"
    done
    echo "✔ seeded $to from $from"

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
