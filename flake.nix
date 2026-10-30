{
  description = "mirror";

  inputs = {
    nixpkgs.url     = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    flakes.url      = "path:/Users/alexwolf/.flakes";
    flakes.inputs.nixpkgs.follows = "nixpkgs";
    flakes.inputs.flake-utils.follows = "flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, flakes }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        rust = flakes.lib.${system}.rust;
      in {
        # Cache-boundary invariance (prism-009): README.md, docs/, the justfile
        # and the flake's own text are outside every package's source.
        # nix/invariance.sh checks the same claim end to end on drvPaths.
        checks.invariance =
          let
            fs = pkgs.lib.fileset;
            probes = fs.unions (map fs.maybeMissing [
              ./README.md ./docs ./justfile ./flake.nix ./flake.lock
              ./.github ./.envrc ./beam ./target
            ]);
            leaked = pkg: fs.toList (fs.intersection pkg.passthru.fileset probes);
            bad = pkgs.lib.concatMap leaked [
              self.packages.${system}.prismqueer
              self.packages.${system}.terni
            ];
          in
          if bad != [ ] then throw "invariance: package sources include ${toString bad}"
          else pkgs.runCommand "prism-invariance" { } "touch $out";

        devShells.default = pkgs.mkShell {
          buildInputs = [
            pkgs.git pkgs.just pkgs.jq
            pkgs.openssl pkgs.zlib
            pkgs.gfortran
            pkgs.lapack pkgs.blas
          ] ++ rust.rustTools
            ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
            pkgs.libiconv
          ];
          shellHook = ''
            export LANG=en_US.UTF-8
            export LAPACK_DIR=${pkgs.lapack}
            export BLAS_DIR=${pkgs.blas}
          '' + rust.rustHook;
        };
      });
}
