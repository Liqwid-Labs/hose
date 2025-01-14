{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, crane, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        craneLib = crane.mkLib pkgs;

        inherit (pkgs) lib;

        unfilteredRoot = ./.; # The original, unfiltered source
        src = lib.fileset.toSource {
          root = unfilteredRoot;
          fileset = lib.fileset.unions [
            # Default files from crane (Rust and cargo files)
            (craneLib.fileset.commonCargoSources unfilteredRoot)
          ];
        };

        commonArgs = {
          inherit src;
          strictDeps = true;
          # tests require live DB and SaaS blockfrost, so it requires network access
          doCheck = false;
        };
        hose = craneLib.buildPackage (commonArgs // {
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        });
      in {
        checks = { inherit hose; };

        packages.default = hose;
        packages.dockerImage = pkgs.dockerTools.buildImage {
          name = "ghcr.io/liqwid-labs/hose";
          tag = "latest";
          copyToRoot = [ hose ];
          config = {
            Cmd = [ "${hose}/bin/hose" ];
            User = "1000:1000";
          };
        };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          shellHook = ''
            export DOPPLER_PROJECT=betterfrost
            export DOPPLER_CONFIG=dev
            export DOPPLER_ENVIRONMENT=dev
          '';
          packages = with pkgs; [ doppler cargo rustc cargo-watch ];
        };
      });
}
