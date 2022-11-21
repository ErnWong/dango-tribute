{
  inputs = {
    cargo2nix.url = "github:cargo2nix/cargo2nix/release-0.11.0";
    flake-utils.follows = "cargo2nix/flake-utils";
    nixpkgs.follows = "cargo2nix/nixpkgs";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = inputs: with inputs;
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            cargo2nix.overlays.default
            rust-overlay.overlays.default
          ];
        };

        rustPkgs = pkgs.rustBuilder.makePackageSet {
            rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain;
          packageFun = import ./Cargo.nix;
        };

      in rec {
        packages = {
          signalling-server = (rustPkgs.workspace.signalling-server {}).bin;
          default = packages.signalling-server;
        };
      }
    );
}
