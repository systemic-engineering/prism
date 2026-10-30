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
      # vectors the tests load from docs/spec (gate_vectors, spec_vectors).
      testFileset = fs.unions (
        [ packageFileset ]
        ++ map (d: fs.intersection (fs.maybeMissing (d + "/tests")) (fs.fileFilter (f: f.hasExt "rs") d)) crates
        ++ [ (fs.fileFilter (f: f.hasExt "json") ./docs/spec) ]
      );

      toSource = fileset: fs.toSource { root = ./.; inherit fileset; };

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
        pkgs:
        let
          rustBin = rust-overlay.lib.mkRustBin { } pkgs;
          toolchain = rustBin.stable.${rustVersion}.default.override {
            extensions = [
              "rust-src"
              "llvm-tools-preview"
            ];
          };
          craneLib = (crane.mkLib pkgs).overrideToolchain (_: toolchain);
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
        in
        {
          inherit toolchain craneLib;
          packages = {
            prismqueer = crate "prismqueer" ./prismqueer [
              ./imperfect
              ./projections
            ] "--features ${features}";
            terni = crate "terni" ./imperfect [ ] "";
          };

          # Named after the justfile's gate: `just check` is fmt + clippy,
          # `just test` is test, `just test coverage` is coverage + doctest.
          checks = {
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

            coverage = craneLib.cargoLlvmCov (
              gate
              // {
                cargoArtifacts = coverageDeps;
                cargoLlvmCovExtraArgs = "--fail-under-lines ${coverageFloor} --lcov --output-path $out";
              }
            );

            doctest = craneLib.cargoDocTest (gate // { cargoArtifacts = gateDeps; });

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
    // flake-utils.lib.eachDefaultSystem (
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
              pkgs.cargo-llvm-cov
              pkgs.rust-analyzer
            ];
          }
        );

        formatter = pkgs.nixfmt;
      }
    );
}
