in @mirror/spec

# prism.spec: prism's build, as it is, as observation only.
#
# prism-020 (mirror-Reed -> prism-Reed, 2026-11-16; Response 2026-11-17).
# This is the first test vector for mirror's spec reading: mirror does not
# read it yet, and every line it can't read is a grammar gap found by a
# real user. Nothing here is aspirational. It describes main at 8de5883.
#
# Grammar: `system @NAME { ... }` with the seven section names of
# shards/mirror/spec/system.mirror (variety, coupling, coherence, reality,
# eigen, loop, kintsugi). system.mirror declares the sections only, each
# with an empty obligation (`{ \ }`), so the forms inside them come from
# their one documented use, mirror.spec on reed/ship: `emits [ kind {
# key value } ]`, `needs [ ]`, `audits [ name { altitude @x via tool
# check verb } ]`, `settle_on { a.b }`, `garden { }`, `pack { lead
# members { ~peer'..' => role } }`, `algedonic bypass a → b`, and the
# path literals ~d'..' and ~f'..'.
#
# Gaps. Where no documented form exists, the line is written anyway, bare
# and comment-free, under a `# Gn` marker, so a reader reports it as a
# \wat at that location. Each Gn is an item on mirror's grammar to-do list:
#
#   G1  a library crate as an emission (`crate`): mirror emits a binary,
#       ci and a release only.
#   G2  a test binary as an emission (`test_binary`).
#   G3  nix outputs as emissions (`nix_package`, `nix_overlay`, `nix_lib`)
#       and `altitude @nix`.
#   G4  a pin in coupling (`pin`, with a path or an input and a rev).
#   G5  a stage pin (a git tag naming a commit) in coupling (`stage`).
#   G6  an unpinned reference, said out loud (`unpinned`).
#   G7  a value that two files must agree on, and who enforces it (`agree`).
#   G8  a check run through nix (`via nix check`), not cargo.
#   G9  a check's declared inputs (`inputs [ ]`) and its environment (`env`).
#   G10 a check's hermeticity (`hermetic yes|no|unknown "why"`).
#   G11 a planned check, present but not running (`planned`).
#   G12 an upstream input in reality (`upstream`), beside the garden.
#   G13 a signer that isn't a pack peer: Alex's seal (`seal`), and a gate on
#       the signature scheme (`gate @spectral-003`).
#   G14 "this repository holds no keys" (`keys none`).
#   G15 where the algedonic bypass lands: a human, not a section (`to`).
#   G16 a repair tool in kintsugi (`repair`), and a lint that only names
#       what to repair (`names`).
#   G17 a predicate with a numeric floor in settle_on (`coverage.lines >=
#       94`): mirror.spec has `==` only (`total_transparency.weight == 0`).
#
# Guesses (a documented form, used where its fit is not certain):
#
#   Q1  `altitude @facet/rust` for prism's cargo checks: mirror.spec's own.
#   Q2  `needs [ ]` between crates: documented between emissions
#       (release needs [binary, ci]); here it is a cargo dependency.
#   Q3  `algedonic bypass variety → eigen`: the documented form, though a
#       red gate is raised at coherence. The bypass is mirror's departure,
#       not Beer's: Beer's algedonic signal passes arousal filters and
#       rises one level at a time (Mara's grade, mirror `mara/beer-graded`
#       40db77ee). It is kept as written, as mirror's.
#   Q4  settle_on predicate names (`clippy.lints`, `test.tests_pass`, ...)
#       follow mirror.spec's `lint.lints` / `tests.tests_pass` pattern;
#       the ones mirror.spec doesn't use are invented names, not grammar.
#   Q5  `pack { lead .. bindings { .. } members { .. } }` read as "who
#       signs commits". mirror.spec uses it for write access; the
#       `writer` binding is copied from mirror.spec as is.
#
# Hermeticity and the to-fix list: docs/spec/checks.md. S3*:
# docs/spec/audit.md.

system @prism {
  # === variety (S1): what prism emits ===
  #
  # Three workspace members and the proc-macro crate terni depends on.
  # prism is a library: no binary, and no release today (prism-020 item 10
  # is agreed, not built).
  variety {
    emits [
      # G1
      crate {
        name     "prismqueer"
        altitude @facet/rust
        emit     cargo
        needs    [terni, prismqueer-projections]
      }
      # G1
      crate {
        name     "terni"
        altitude @facet/rust
        emit     cargo
        needs    [terni-macros]
      }
      # G1
      crate {
        name     "terni-macros"
        altitude @facet/rust
        emit     cargo
      }
      # G1
      crate {
        name     "prismqueer-projections"
        altitude @facet/rust
        emit     cargo
      }
      # G2
      test_binary {
        name     "suite"
        altitude @facet/rust
        emit     cargo
        needs    [prismqueer]
      }
      # G3
      nix_package {
        name     "prismqueer"
        altitude @nix
        emit     nix
        needs    [prismqueer]
      }
      # G3
      nix_package {
        name     "terni"
        altitude @nix
        emit     nix
        needs    [terni]
      }
      # G3
      nix_overlay {
        name     "default"
        altitude @nix
        emit     nix
        needs    [prismqueer, terni]
      }
      # G3
      nix_lib {
        name     "lib"
        altitude @nix
        emit     nix
      }
    ]
  }

  # === coupling (S2): what keeps the S1 units from oscillating ===
  #
  # Every pin, by what it pins. flake.lock revs as of 8de5883.
  coupling {
    # G4
    pin ~f'Cargo.lock'
    # G4
    pin ~f'flake.lock'
    # G4
    pin nixpkgs      "github:NixOS/nixpkgs"      58deea654ec5dd0644f31c840a07c29fa0592528
    # G4
    pin crane        "github:ipetkov/crane"      e675a36b7377bebe41d548388a2501cbb16d380d
    # G4
    pin flake-utils  "github:numtide/flake-utils" 11707dc2f618dd54ca8739b309ec4fc024de578b
    # G4
    pin rust-overlay "github:oxalica/rust-overlay" 61ec6a4fc56fe0c2b863f7b3eaba07b6664697d9
    # G4
    pin rust         "1.94.0"
    # G4
    pin ~d'imperfect/' e05d7e07f7a74a41131bc87d998c7dfcfcde9aca
    # G5
    stage prism-flake-v1 36b9fff1b6d0e654c32fe826ef002c9aea8489e2
    # G6
    unpinned "systemic-engineering/ci/.github/workflows/rust-nix-check.yml@main"
    # G6
    unpinned "systemic-engineering/ci/actions/nix-setup@main"
    # G7
    agree coverage_floor 94 [~f'flake.nix', ~f'.github/workflows/ci.yml'] by ratchet
    # G7
    agree features "lapack,fate" [~f'flake.nix', ~f'justfile'] by none
    # The justfile repeats "lapack,fate" in five recipes, and `merge` runs
    # "lapack,fate,pq". Nothing checks that they match flake.nix.
  }

  # === coherence (S3): the gate ===
  #
  # The flake's checks, each a nix derivation keyed by its inputs, and the
  # justfile recipes that run them. Filesets as flake.nix names them:
  #   package = Cargo.toml, Cargo.lock, each crate's *.rs *.toml *.f90 *.bf
  #             minus tests/ and target/
  #   test    = package + each crate's tests/**/*.rs + docs/spec/*.json
  coherence {
    audits [
      # Q1 G8 G9 G10
      fmt {
        altitude @facet/rust
        via      nix check fmt
        inputs   [~f'Cargo.toml', ~f'Cargo.lock', ~d'prismqueer/', ~d'imperfect/', ~d'projections/', ~d'docs/spec/']
        hermetic yes
      }
      # Q1 G8 G9 G10
      clippy {
        altitude @facet/rust
        via      nix check clippy
        inputs   [~f'Cargo.toml', ~f'Cargo.lock', ~d'prismqueer/', ~d'imperfect/', ~d'projections/', ~d'docs/spec/']
        env      [rust "1.94.0", gfortran, lapack, blas, features "lapack,fate"]
        hermetic yes
      }
      # Q1 G8 G9 G10
      test {
        altitude @facet/rust
        via      nix check test
        inputs   [~f'Cargo.toml', ~f'Cargo.lock', ~d'prismqueer/', ~d'imperfect/', ~d'projections/', ~d'docs/spec/']
        env      [rust "1.94.0", gfortran, lapack, blas, features "lapack,fate", OPENBLAS_NUM_THREADS 1]
        hermetic no "wall clock: flux::Sample::new, Fate::excited; Instant asserted: fate runtime_performance, compiled_performance"
      }
      # Q1 G8 G9 G10
      coverage {
        altitude @facet/rust
        via      nix check coverage
        inputs   [~f'Cargo.toml', ~f'Cargo.lock', ~d'prismqueer/', ~d'imperfect/', ~d'projections/', ~d'docs/spec/']
        env      [rust "1.94.0", gfortran, lapack, blas, cargo-llvm-cov, features "lapack,fate", OPENBLAS_NUM_THREADS 1]
        hermetic no "as test; the lcov line set moves with the clock-seeded runs"
      }
      # Q1 G8 G9 G10
      doctest {
        altitude @facet/rust
        via      nix check doctest
        inputs   [~f'Cargo.toml', ~f'Cargo.lock', ~d'prismqueer/', ~d'imperfect/', ~d'projections/', ~d'docs/spec/']
        hermetic yes
      }
      # G3 G8 G9 G10
      ratchet {
        altitude @nix
        via      nix check ratchet
        inputs   [~f'flake.nix', ~f'.github/workflows/ci.yml']
        hermetic yes
      }
      # G3 G8 G9 G10
      invariance {
        altitude @nix
        via      nix check invariance
        inputs   [~f'flake.nix']
        hermetic yes
      }
      # G3 G8 G9 G10
      invariance_end_to_end {
        altitude @nix
        via      just invariance
        inputs   [~f'nix/invariance.sh', ~f'flake.nix', ~f'flake.lock']
        hermetic yes
      }
      # G11
      s3_star {
        planned  ~f'docs/spec/audit.md'
      }
    ]
    # `just check` (fmt + clippy) and `just test` (plain) run the same tools
    # over the working tree, unkeyed: the dev loop, not cacheable verdicts.
    # `just pre-push` = `just test coverage` = checks.coverage +
    # checks.doctest. The push hook (outside this repo) runs it.
    settle_on {
      fmt.formats
      clippy.lints
      test.tests_pass
      doctest.tests_pass
      ratchet.agrees
      invariance.holds
      # G17
      coverage.lines >= 94
    }
  }

  # === reality (S4): the outside prism builds against ===
  reality {
    # No garden packages: prism depends on none.
    garden { }
    # G12
    upstream nixpkgs      "github:NixOS/nixpkgs/nixos-unstable"
    # G12
    upstream crane        "github:ipetkov/crane"
    # G12
    upstream flake-utils  "github:numtide/flake-utils"
    # G12
    upstream rust-overlay "github:oxalica/rust-overlay"
    # G12
    upstream terni        "https://github.com/systemic-engineering/imperfect.git"
    # G12
    upstream crates-io    ~f'Cargo.lock'
    # G12
    upstream ci           "github:systemic-engineering/ci"
  }

  # === eigen (S5): who signs ===
  #
  # Commits are SSH-signed: the last 40 on main all verify (git log %G? =
  # G), by Reed (36) and Taut (4). Older history has unsigned ones.
  # Alex's seal decides what changes the names, the rulers and the release.
  # The signature scheme waits on spectral-003's post-quantum gate.
  # prism verifies; it holds no keys.
  eigen {
    # Q5
    pack {
      lead ~peer'~/.reed'

      bindings {
        let writer = acl { ops: any, targets: any, predicates: [] }
      }

      members {
        ~peer'~/.taut' => writer
      }
    }
    # G13
    seal alex
    # G13
    gate @spectral-003
    # G14
    keys none
  }

  # === loop: the algedonic bypass ===
  #
  # A red gate is not averaged away: it stops the push, and a kept
  # disagreement goes to Alex as a question (docs/spec/gate.md).
  loop {
    # Q3
    algedonic bypass variety → eigen
    # G15
    to alex
  }

  # === kintsugi: how prism heals itself ===
  #
  # `just fmt` rewrites; clippy only names what to heal (`--deny warnings`).
  # No roomba walks prism today.
  kintsugi {
    # G16
    repair fmt via cargo fmt
    # G16
    names clippy via cargo clippy
  }
}
