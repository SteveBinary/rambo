{
  description = "RAMBO - Rename All Media By Order - rename media files based on their date/time of creation";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { ... }@inputs:
    inputs.flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [ (import inputs.rust-overlay) ];
        };

        craneLib = (inputs.crane.mkLib pkgs).overrideToolchain (
          p: p.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml
        );

        src = craneLib.cleanCargoSource ./.;

        commonArgs = {
          inherit src;
          strictDeps = true;

          buildInputs = [
            # add additional build inputs here
          ];
        };

        # all dependencies, without our code -> make caching effective
        cargoArtifacts = craneLib.buildDepsOnly (
          commonArgs
          // {
            cargoBuildCommand = "cargo build --profile release-with-lto";
          }
        );

        rambo = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            cargoBuildCommand = "cargo build --profile release-with-lto";
          }
        );
      in
      {
        packages = {
          inherit rambo;
          default = rambo;
        };

        devShells.default = craneLib.devShell {
          packages = with pkgs; [
            # additional packages for the dev shell
          ];
        };

        formatter = pkgs.nixfmt-rfc-style;
      }
    );
}
