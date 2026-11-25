{
  description = "prism";

  # One flake, no household inputs: prism is the bottom of the stack and a
  # public crate, so it pins its own toolchain and nothing else of ours.
  # Consumers follow their own nixpkgs: `prism.inputs.nixpkgs.follows`.
  inputs = {
    # imperfect/ (terni) is a git submodule and a workspace member.
    self.submodules = true;
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      crane,
      rust-overlay,
    }:
    let
      inherit (nixpkgs) lib;
      fs = lib.fileset;

      # Same release the household pins in ~/.flakes/rust.nix. A nixpkgs bump
      # leaves the rustc store path alone, so cargo fingerprints survive it.
      rustVersion = "1.94.0";

      # The gate's feature set (justfile). One place, so `just` and nix agree.
      features = "lapack,fate";

      # Line-coverage ratchet; the justfile's `test coverage` uses the same.
      coverageFloor = "94";

      # ── Sources: the cache boundaries ─────────────────────────────────────────────
      # A derivation rebuilds when its source changes, so each one gets only
      # what it reads. `.f90` (prismqueer/native) and `.bf` (fate/brainfuck)
      # are build inputs; crane's cleanCargoSource would drop both.
      crates = [
        ./prismqueer
        ./imperfect
        ./projections
        ./chorus
        ./echo
        ./laws
        ./mcp
      ];
      sourceExts = [
        "rs"
        "toml"
        "f90"
        "bf"
      ];
      crateFiles =
        exts: dir:
        fs.difference (fs.fileFilter (f: lib.any f.hasExt exts) dir) (
          fs.unions [
            (fs.maybeMissing (dir + "/tests"))
            (fs.maybeMissing (dir + "/target"))
          ]
        );

      # What the packages are built from. Tests are not in it: a test edit
      # must not rebuild a package (nix/invariance.sh).
      packageFileset = fs.unions (
        [
          ./Cargo.toml
          ./Cargo.lock
        ]
        ++ map (crateFiles sourceExts) crates
      );

      # Every member's manifest: cargo loads the whole workspace to build any
      # one crate of it.
      manifestFileset = fs.unions (
        [
          ./Cargo.toml
          ./Cargo.lock
        ]
        ++ map (fs.fileFilter (f: f.name == "Cargo.toml")) crates
      );

      # What the checks read: the packages' sources, the tests, and the spec
      # vectors the tests load from docs/spec (gate_vectors, spec_vectors),
      # and the stock tools' raw output (docs/spec/seal-vectors: openssl and
      # ssh-keygen keys, messages and signatures, byte for byte), and mirror's
      # data that echo reads (docs/spec/echo-vectors).
      testFileset = fs.unions (
        [ packageFileset ]
        ++ map (d: fs.intersection (fs.maybeMissing (d + "/tests")) (fs.fileFilter (f: f.hasExt "rs") d)) crates
        ++ [
          (fs.fileFilter (f: f.hasExt "json") ./docs/spec)
          ./docs/spec/seal-vectors
          ./docs/spec/echo-vectors
        ]
      );

      toSource = fileset: fs.toSource { root = ./.; inherit fileset; };

      # The independence guards: the shared walk, its two wrappers, and their
      # fixtures, side by side (each wrapper calls the walk next to it).
      guardSrc = toSource (
        fs.unions [
          ./nix/independent.sh
          ./nix/echo-independent.sh
          ./nix/prism-mcp-independent.sh
          ./nix/prismqueer-exact.sh
          ./nix/laws-independent.sh
          ./nix/fixtures
        ]
      );

      # ── The native build environment ───────────────────────────────────────────────────
      # prismqueer/build.rs reads FC, LAPACK_DIR and BLAS_DIR (each under
      # rerun-if-env-changed). Here they come from the store, so the build is
      # pure. Exported as lib.nativeEnv for anything that compiles prismqueer
      # (mirror), so the recipe lives in one place.
      nativeEnv = pkgs: {
        nativeBuildInputs = [ pkgs.gfortran ];
        buildInputs = [
          pkgs.lapack
          pkgs.blas
        ]
        ++ lib.optionals pkgs.stdenv.hostPlatform.isDarwin [ pkgs.libiconv ];
        FC = "gfortran";
        LAPACK_DIR = "${pkgs.lapack}";
        BLAS_DIR = "${pkgs.blas}";
        # Tests run in parallel; OpenBLAS threads inside each dsyev on top of
        # that oversubscribe the cores (Taut profile, 2026-10-30).
        OPENBLAS_NUM_THREADS = "1";
      };

      # ── Everything prism builds, for one pkgs ─────────────────────────────────────────
      mkPrism =
        pkgs0:
        let
          # cargo-llvm-cov 0.9.0 in nixpkgs 58deea65 fails its own test suite
          # on aarch64-darwin (the instrumented test binaries SIGSEGV; sandbox
          # on or off), so Hydra has no build of it and a local one dies in
          # checkPhase. The tool is only run here, through checks.coverage,
          # which gates on the measured line coverage. Drop this override once
          # nixpkgs builds it.
          pkgs = pkgs0.extend (
            _: prev: {
              cargo-llvm-cov = prev.cargo-llvm-cov.overrideAttrs (_: {
                doCheck = false;
              });
            }
          );
          rustBin = rust-overlay.lib.mkRustBin { } pkgs;
          toolchain = rustBin.stable.${rustVersion}.default.override {
            extensions = [
              "rust-src"
              "llvm-tools-preview"
            ];
          };
          craneLib = (crane.mkLib pkgs).overrideToolchain (_: toolchain);

          # The same rustc with std for wasm32-unknown-unknown and core/alloc
          # for a bare-metal target, for chorus's portability check
          # only. A toolchain of its own, so the gate's toolchain (and every
          # cached build under it) stays as it is.
          portableTargets = [
            "wasm32-unknown-unknown"
            "thumbv7em-none-eabihf"
          ];
          craneLibPortable = (crane.mkLib pkgs).overrideToolchain (
            _: rustBin.stable.${rustVersion}.minimal.override { targets = portableTargets; }
          );
          inherit (craneLib.crateNameFromCargoToml { cargoToml = ./prismqueer/Cargo.toml; }) version;

          packageSrc = toSource packageFileset;
          testSrc = toSource testFileset;

          # One member's own files over stubs of the rest (crane's dummy
          # targets), so a crate's package moves only with its own sources,
          # its local dependencies' sources, and the manifests.
          memberSrc =
            name: dirs:
            let
              stubs = craneLib.mkDummySrc { src = toSource manifestFileset; };
              own = toSource (fs.unions ([ manifestFileset ] ++ map (crateFiles sourceExts) dirs));
            in
            pkgs.runCommand "${name}-src" { } ''
              cp -R ${stubs} $out
              chmod -R u+w $out
              cp -R ${own}/. $out/
            '';

          common = nativeEnv pkgs // {
            pname = "prism";
            inherit version;
            strictDeps = true;
          };

          # The checks use cargo's own profiles (dev, and test for the suite),
          # as `just` does: debug assertions and overflow checks stay on.
          gate = common // {
            src = testSrc;
            CARGO_PROFILE = "";
            cargoExtraArgs = "--locked --workspace --features ${features}";
            # A check's output is its verdict. Archiving its target dir
            # (~650 MiB before compression) would put 160 MiB in the store per
            # check per change.
            doInstallCargoArtifacts = false;
          };

          # Third-party dependencies, compiled once, out of the source's way:
          # built from a dummy of the workspace (Cargo.toml + Cargo.lock), so
          # only a dependency change rebuilds it.
          gateDeps = craneLib.buildDepsOnly (
            gate
            // {
              src = packageSrc;
              doInstallCargoArtifacts = true;
            }
          );

          # cargo-llvm-cov instruments every crate, dependencies included, so
          # the plain dependency build doesn't serve it. This one is the same
          # dependency build, instrumented, in cargo-llvm-cov's own target dir.
          coverageDeps = craneLib.buildDepsOnly (
            gate
            // {
              src = packageSrc;
              pnameSuffix = "-coverage-deps";
              nativeBuildInputs = gate.nativeBuildInputs ++ [ pkgs.cargo-llvm-cov ];
              buildPhaseCargoCommand = "cargo llvm-cov --no-report --locked --workspace --features ${features}";
              doCheck = false;
              doInstallCargoArtifacts = true;
            }
          );

          coverage = craneLib.cargoLlvmCov (
              gate
              // {
                cargoArtifacts = coverageDeps;
                # --remap-path-prefix: rustc maps the workspace root (the
                # per-build directory) to "" in the workspace crates only
                # (cargo-llvm-cov's wrapper; dependencies keep their flags, so
                # coverageDeps still serves), and the SF: paths come out
                # repo-relative. llvm-cov's -path-equivalence would not do:
                # it locates sources, the exported SF: names stay absolute.
                # Relative, the build script's out/ under target/ slips past
                # cargo-llvm-cov's own target-dir filter; ^target/ keeps it out.
                cargoLlvmCovExtraArgs = "--remap-path-prefix --ignore-filename-regex ^target/ --fail-under-lines ${coverageFloor} --lcov --output-path $out";
                # The output is checked, not just produced: every SF: path
                # repo-relative, so a rebuild gives the same bytes.
                doInstallCheck = true;
                installCheckPhase = "bash ${./nix/lcov-paths.sh} $out";
              }
            );

          # The floor's sources, as the lcov names them (laws/src/…), for
          # checks.laws-coverage's markers; and the filter's fixtures.
          lawsSrc = toSource (fs.fileFilter (f: f.hasExt "rs") ./laws/src);
          coverageFixtures = toSource ./nix/coverage-fixtures + "/nix/coverage-fixtures";

          release = common // {
            src = packageSrc;
            cargoExtraArgs = "--locked --workspace --features ${features}";
            # The packages run no tests (the checks do), so their dependency
            # build skips the release-mode test compile too.
            doCheck = false;
          };
          releaseDeps = craneLib.buildDepsOnly release;

          # prism is a library: its consumers link it through cargo. The
          # package is the release build of one crate, with its target dir
          # installed, so the build is cached and the boundary is testable.
          crate =
            name: dir: localDeps: extraArgs:
            craneLib.buildPackage (
              release
              // {
                pname = name;
                inherit (craneLib.crateNameFromCargoToml { cargoToml = dir + "/Cargo.toml"; }) version;
                src = memberSrc name ([ dir ] ++ localDeps);
                # memberSrc is a derivation; reading Cargo.lock out of it at
                # eval time would be import-from-derivation. Same lock, from
                # the tree.
                cargoLock = ./Cargo.lock;
                cargoVendorDir = craneLib.vendorCargoDeps { cargoLock = ./Cargo.lock; };
                cargoArtifacts = releaseDeps;
                cargoExtraArgs = "--locked -p ${name} ${extraArgs}";
                doInstallCargoArtifacts = true;
                passthru.fileset = packageFileset;
              }
            );

          packages = {
            prismqueer = crate "prismqueer" ./prismqueer [
              ./imperfect
              ./laws
              ./projections
            ] "--features ${features}";
            terni = crate "terni" ./imperfect [ ] "";
          };

          # A package's store path, or "" when the flake doesn't build it
          # (checks.packages-built reads "" as missing).
          built = name: if packages ? ${name} then "${packages.${name}}" else "";
        in
        {
          inherit toolchain craneLib pkgs packages;

          # Named after the justfile's gate: `just check` is fmt + clippy,
          # `just test` is test, `just test coverage` is coverage + doctest.
          checks = {
            # The binaries as packages: each bin/ holds its one executable, and
            # it runs (echo's usage exit 2; prism-mcp's initialize round-trip).
            packages-built = pkgs.runCommand "prism-packages-built" { } ''
              bash ${./nix/packages-built.sh} "${built "echo"}" "${built "prism-mcp"}"
              touch $out
            '';

            fmt = craneLib.cargoFmt { src = testSrc; inherit (common) pname version; };

            clippy = craneLib.cargoClippy (
              gate
              // {
                cargoArtifacts = gateDeps;
                # Same scope as `just lint`: every target, tests and benches too.
                cargoClippyExtraArgs = "--all-targets -- --deny warnings";
              }
            );

            test = craneLib.cargoTest (gate // { cargoArtifacts = gateDeps; });

            inherit coverage;

            # docs/spec/coverage.md: prism's floor at 100% line coverage, with
            # every gap an explicit, reasoned skip. nix/coverage-skips.sh reads
            # checks.coverage's lcov and laws/src, drops the lines under a
            # `// coverage: skip(<reason>)` or a `skip-start(<reason>)` …
            # `skip-end` region, and fails unless every other line ran, no
            # skip covers an executed line or none, every marker has a reason,
            # and the skipped lines number exactly nix/laws-skips-max (a
            # ratchet: it only goes down). The output is the ledger
            # (skips.txt, one line per skip: file:lines, count, reason) and
            # the verdict (summary.txt). The workspace keeps its own ratchet
            # (coverageFloor) in checks.coverage.
            laws-coverage = pkgs.runCommand "prism-laws-coverage" { } ''
              mkdir -p $out
              bash ${./nix/coverage-skips.sh} ${coverage} ${lawsSrc} laws/src ${./nix/laws-skips-max} \
                > $out/skips.txt 2> $out/summary.txt || { cat $out/summary.txt >&2; exit 1; }
              cat $out/summary.txt >&2
            '';

            # The filter itself, on tiny lcov + source pairs
            # (nix/coverage-fixtures/<case>/: cov.lcov, laws/src/*.rs, max,
            # expect, and for some accepted cases the exact ledger): the
            # accepted ones pass with their ledger, and each refused one fails
            # for its own reason (the text after `refuse: ` in its expect).
            coverage-skips = pkgs.runCommand "prism-coverage-skips" { } ''
              filter=${./nix/coverage-skips.sh}
              for d in ${coverageFixtures}/*/; do
                d=''${d%/}; n=''${d##*/}; expect=$(cat $d/expect)
                if said=$(bash $filter $d/cov.lcov $d laws/src $d/max 2> err); then rc=0; else rc=$?; fi
                case "$expect" in
                  ok)
                    if [ $rc -ne 0 ]; then echo "coverage-skips refused $n:" >&2; cat err >&2; exit 1; fi
                    if [ -f $d/ledger ] && [ "$said" != "$(cat $d/ledger)" ]; then
                      echo "coverage-skips: $n's ledger is [$said], not [$(cat $d/ledger)]" >&2; exit 1
                    fi ;;
                  refuse:*)
                    why=''${expect#refuse: }
                    if [ $rc -eq 0 ]; then echo "coverage-skips accepted $n" >&2; exit 1; fi
                    if ! grep -qF -- "$why" err; then echo "coverage-skips refused $n, but not for $why:" >&2; cat err >&2; exit 1; fi ;;
                  *) echo "coverage-skips: $n's expect is unreadable: $expect" >&2; exit 1 ;;
                esac
                echo "coverage-skips: $n as expected" >&2
              done
              touch $out
            '';

            # The guard itself: it must reject an absolute or build-directory
            # SF: path, a file under target/ (generated, not source), and an
            # lcov with none, and accept relative ones.
            lcov-paths = pkgs.runCommand "prism-lcov-paths" { } ''
              guard=${./nix/lcov-paths.sh}
              printf 'SF:prismqueer/src/lib.rs\nDA:1,1\nend_of_record\n' > good
              printf 'SF:/nix/var/nix/builds/nix-1-0/source/prismqueer/src/lib.rs\nend_of_record\n' > build
              printf 'SF:/build/source/prismqueer/src/lib.rs\nend_of_record\n' > absolute
              printf 'SF:prismqueer/src/lib.rs\nSF:x/nix/var/nix/builds/y.rs\n' > embedded
              printf 'SF:prismqueer/src/lib.rs\nSF:target/llvm-cov-target/debug/build/p-0/out/gen.rs\n' > generated
              printf 'DA:1,1\n' > empty
              bash $guard good
              for f in build absolute embedded generated empty; do
                if bash $guard $f 2>/dev/null; then echo "lcov-paths accepted $f" >&2; exit 1; fi
              done
              touch $out
            '';

            doctest = craneLib.cargoDocTest (gate // { cargoArtifacts = gateDeps; });

            # chorus builds where a reader's browser runs it
            # (wasm32-unknown-unknown) and where no std exists at all
            # (thumbv7em-none-eabihf: core + alloc only, so a dependency that
            # pulls in std fails here). Release, as a page would ship it.
            chorus-portable = craneLibPortable.mkCargoDerivation {
              pname = "chorus-portable";
              inherit version;
              src = packageSrc;
              strictDeps = true;
              cargoArtifacts = null;
              cargoVendorDir = craneLibPortable.vendorCargoDeps { cargoLock = ./Cargo.lock; };
              doInstallCargoArtifacts = false;
              buildPhaseCargoCommand = lib.concatMapStringsSep "\n" (
                t: "cargo build --locked --release -p chorus --target ${t}"
              ) portableTargets;
              installPhaseCommand = "touch $out";
            };

            # echo, the second reader (prism-022), builds for the same two
            # targets: no std, so no clock, no file and no process can reach it.
            echo-portable = craneLibPortable.mkCargoDerivation {
              pname = "echo-portable";
              inherit version;
              src = packageSrc;
              strictDeps = true;
              cargoArtifacts = null;
              cargoVendorDir = craneLibPortable.vendorCargoDeps { cargoLock = ./Cargo.lock; };
              doInstallCargoArtifacts = false;
              buildPhaseCargoCommand = lib.concatMapStringsSep "\n" (
                t: "cargo build --locked --release -p echo --lib --target ${t}"
              ) portableTargets;
              installPhaseCommand = "touch $out";
            };

            # prism-022 S5: echo's Cargo graph (every kind, every depth)
            # holds no prismqueer, no mirror crate and nothing LAPACK, BLAS or
            # Fortran. The guard runs on the real graph, and must reject the
            # three forbidden fixtures (a direct prismqueer, a transitive
            # openblas-src behind a dev cycle, an innocently named crate with
            # `links = "lapack"`) and accept the clean one
            # (prismqueer in the workspace, but not under echo).
            echo-independent = craneLib.mkCargoDerivation {
              pname = "echo-independent";
              inherit version;
              src = packageSrc;
              strictDeps = true;
              cargoArtifacts = null;
              cargoVendorDir = craneLib.vendorCargoDeps { cargoLock = ./Cargo.lock; };
              nativeBuildInputs = [ pkgs.jq ];
              doInstallCargoArtifacts = false;
              buildPhaseCargoCommand = ''
                guard=${guardSrc}/nix/echo-independent.sh
                fixtures=${guardSrc}/nix/fixtures
                cargo metadata --format-version 1 --locked --offline > metadata.json
                bash $guard metadata.json
                bash $guard $fixtures/echo-clean.json
                for f in $fixtures/echo-direct.json $fixtures/echo-transitive.json $fixtures/echo-links.json; do
                  if bash $guard $f 2>/dev/null; then echo "echo-independent accepted $f" >&2; exit 1; fi
                done
              '';
              installPhaseCommand = "touch $out";
            };

            # docs/spec/mcp.md §6, docs/spec/laws.md: prism-mcp's own build
            # graph holds no serde, no JSON crate, no mirror crate, no
            # prismqueer (its laws come from prism-laws) and nothing LAPACK,
            # BLAS or Fortran.
            # The graph is `cargo tree -p prism-mcp` (cargo's resolve for
            # prism-mcp alone: its own dev edges, never its dependencies',
            # features as that build turns them on), with `links` from the
            # metadata. The guard runs on the real graph; accepts mcp-merged
            # (prism-mcp → prism-laws, the real tree), whose workspace-union
            # metadata the old reading refused for terni → serde (asserted
            # here); and refuses eight, each for its own reason: prismqueer,
            # terni's serde really on, serde_json as a normal edge, a JSON
            # crate, a mirror crate, `links = "blas"`, a package the metadata
            # doesn't have, and a line that doesn't parse.
            prism-mcp-independent = craneLib.mkCargoDerivation {
              pname = "prism-mcp-independent";
              inherit version;
              src = packageSrc;
              strictDeps = true;
              cargoArtifacts = null;
              cargoVendorDir = craneLib.vendorCargoDeps { cargoLock = ./Cargo.lock; };
              nativeBuildInputs = [ pkgs.jq ];
              doInstallCargoArtifacts = false;
              buildPhaseCargoCommand = ''
                guard=${guardSrc}/nix/prism-mcp-independent.sh
                fixtures=${guardSrc}/nix/fixtures
                cargo metadata --format-version 1 --locked --offline > metadata.json
                cargo tree --locked --offline -p prism-mcp -e normal,build,dev --target all --prefix none -f '{p}|{f}' > tree.txt
                bash $guard tree.txt metadata.json
                bash $guard $fixtures/mcp-merged.tree $fixtures/mcp-merged.json
                # The old reading, the workspace union, refused that graph.
                union='^(mirror|mirror[-_].*|.*serde.*|.*json.*|.*lapack.*|.*blas.*|.*fortran.*|netlib-src|intel-mkl.*|accelerate-src)$'
                old=$(bash ${guardSrc}/nix/independent.sh $fixtures/mcp-merged.json prism-mcp "$union" root)
                if ! echo "$old" | grep -qx serde; then echo "the union reading no longer refuses mcp-merged: $old" >&2; exit 1; fi
                while read -r f why; do
                  if said=$(bash $guard $fixtures/mcp-$f.tree $fixtures/mcp-packages.json 2>&1); then echo "prism-mcp-independent accepted mcp-$f" >&2; exit 1; fi
                  case "$said" in *"$why"*) ;; *) echo "prism-mcp-independent refused mcp-$f, but not for $why: $said" >&2; exit 1 ;; esac
                done <<'EOF'
                prismqueer prismqueer
                terni-serde serde_derive
                serde-json serde_json
                json tinyjson
                mirror mirror-ast
                links quietly (links blas)
                missing unheard-of 9.9.9 (not in the metadata)
                unparsed unparsed: chorus v0.1.0
                EOF
              '';
              installPhaseCommand = "touch $out";
            };

            # docs/spec/features.md: prismqueer's certified path builds and
            # passes its tests with `--no-default-features --features exact`
            # and without the native environment (no gfortran, LAPACK or BLAS
            # here: a Fortran step would fail the build). Its library closure
            # (normal and build edges) holds no cc but blake3's own, nothing
            # LAPACK, BLAS or Fortran, by name or `links`. The guard runs on
            # the real graph under `exact`, refuses it under `lapack` (cc),
            # accepts the clean fixture (blake3's cc, dev-only cc and
            # openblas-src) and refuses four: prismqueer's own cc, cc through
            # another crate, `links = "lapack"`, a Fortran runtime by name.
            prismqueer-exact = craneLib.mkCargoDerivation {
              pname = "prismqueer-exact";
              inherit version;
              src = testSrc;
              strictDeps = true;
              cargoArtifacts = null;
              cargoVendorDir = craneLib.vendorCargoDeps { cargoLock = ./Cargo.lock; };
              nativeBuildInputs = [ pkgs.jq ];
              doInstallCargoArtifacts = false;
              buildPhaseCargoCommand = ''
                guard=${guardSrc}/nix/prismqueer-exact.sh
                fixtures=${guardSrc}/nix/fixtures
                exact="--no-default-features --features prismqueer/exact"
                cargo metadata --format-version 1 --locked --offline $exact > exact.json
                bash $guard exact.json
                cargo metadata --format-version 1 --locked --offline --no-default-features --features prismqueer/lapack > lapack.json
                if bash $guard lapack.json 2>/dev/null; then echo "prismqueer-exact accepted lapack" >&2; exit 1; fi
                bash $guard $fixtures/exact-clean.json
                for f in cc transitive links fortran; do
                  if bash $guard $fixtures/exact-$f.json 2>/dev/null; then echo "prismqueer-exact accepted exact-$f" >&2; exit 1; fi
                done
                cargo build --locked --offline -p prismqueer $exact
                cargo test --locked --offline -p prismqueer $exact
              '';
              installPhaseCommand = "touch $out";
            };

            # docs/spec/laws.md: prism's floor, prism-laws, builds alone
            # without the native environment (no gfortran, LAPACK or BLAS
            # here), passes its own tests (the float scan among them), and
            # its own resolve (`cargo tree -p prism-laws`, `links` and
            # proc-macro targets from the metadata) holds no proc-macro, no
            # serde or JSON crate, no mirror crate, no prismqueer, nothing
            # LAPACK, BLAS or Fortran, and no cc but blake3's. The guard runs
            # on the real graph, accepts the clean fixture, and refuses
            # fourteen, each for its own reason.
            laws-independent = craneLib.mkCargoDerivation {
              pname = "laws-independent";
              inherit version;
              src = testSrc;
              strictDeps = true;
              cargoArtifacts = null;
              cargoVendorDir = craneLib.vendorCargoDeps { cargoLock = ./Cargo.lock; };
              nativeBuildInputs = [ pkgs.jq ];
              doInstallCargoArtifacts = false;
              buildPhaseCargoCommand = ''
                guard=${guardSrc}/nix/laws-independent.sh
                fixtures=${guardSrc}/nix/fixtures
                cargo metadata --format-version 1 --locked --offline > metadata.json
                cargo tree --locked --offline -p prism-laws -e normal,build,dev --target all --prefix depth -f '{p}|{f}' > tree.txt
                bash $guard tree.txt metadata.json
                bash $guard $fixtures/laws-clean.tree $fixtures/laws-packages.json
                while read -r f why; do
                  if said=$(bash $guard $fixtures/laws-$f.tree $fixtures/laws-packages.json 2>&1); then echo "laws-independent accepted laws-$f" >&2; exit 1; fi
                  case "$said" in *"$why"*) ;; *) echo "laws-independent refused laws-$f, but not for $why: $said" >&2; exit 1 ;; esac
                done <<'EOF'
                terni-macros terni-macros (proc-macro)
                projections prismqueer-projections
                serde serde_core
                json tinyjson
                mirror mirror-ast
                prismqueer prismqueer
                lapack netlib-src
                links quietly (links blas)
                cc cc (through psm)
                missing unheard-of 9.9.9 (not in the metadata)
                unparsed unparsed: 1chorus 0.1.0|
                depth unparsed: depth 3 after 1: num-traits
                empty does not start with prism-laws
                wrong-root does not start with prism-laws
                EOF
                cargo build --locked --offline -p prism-laws
                cargo test --locked --offline -p prism-laws
              '';
              installPhaseCommand = "touch $out";
            };

            # One ratchet: the CI workflow repeats coverageFloor as its own
            # `coverage_threshold`. The two must be the same number, or CI
            # fails a push the local gate passed (it did: '99' against 94).
            ratchet =
              let
                m = builtins.match ".*coverage_threshold: '([0-9]+)'.*" (builtins.readFile ./.github/workflows/ci.yml);
              in
              if m != [ coverageFloor ] then
                throw "ratchet: .github/workflows/ci.yml coverage_threshold is ${toString m}, flake.nix coverageFloor is ${coverageFloor}"
              else
                pkgs.runCommand "prism-ratchet" { } "touch $out";

            # The package fileset excludes what the push of a doc or a test
            # must not rebuild. nix/invariance.sh checks it end to end.
            invariance =
              let
                probes = fs.unions (
                  map fs.maybeMissing [
                    ./README.md
                    ./docs
                    ./justfile
                    ./flake.nix
                    ./flake.lock
                    ./.github
                    ./.envrc
                    ./beam
                    ./target
                    ./prismqueer/tests
                    ./imperfect/tests
                    ./chorus/tests
                    ./echo/tests
                    ./mcp/tests
                  ]
                );
                leaked = fs.toList (fs.intersection packageFileset probes);
              in
              if leaked != [ ] then
                throw "invariance: package sources include ${toString leaked}"
              else
                pkgs.runCommand "prism-invariance" { } "touch $out";
          };
        };
    in
    {
      # For anything that compiles prismqueer itself (mirror, through cargo).
      lib = { inherit nativeEnv features rustVersion; };

      # pkgs.prism.{prismqueer,terni}, built against the consumer's nixpkgs.
      overlays.default = final: _prev: { prism = (mkPrism final).packages; };
    }
    # nixpkgs dropped x86_64-darwin (26.11), so evaluating it throws; these
    # are the systems that evaluate (nix/invariance.sh checks all of them).
    // flake-utils.lib.eachSystem
      [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ]
      (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        prism = mkPrism pkgs;
      in
      {
        packages = prism.packages // {
          default = prism.packages.prismqueer;
        };

        inherit (prism) checks;

        devShells.default = prism.craneLib.devShell (
          builtins.removeAttrs (nativeEnv pkgs) [
            "nativeBuildInputs"
            "buildInputs"
          ]
          // {
            # The checks' inputs (toolchain, gfortran, lapack, blas) come with.
            checks = self.checks.${system};
            packages = [
              pkgs.just
              pkgs.git
              pkgs.jq
              prism.pkgs.cargo-llvm-cov
              pkgs.rust-analyzer
            ];
          }
        );

        formatter = pkgs.nixfmt;
      }
    );
}
