{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachSystem [
      "aarch64-linux"
      "x86_64-linux"
    ] (
      system:
      let
        overlays = [
          rust-overlay.overlays.default
        ];
        pkgs = import nixpkgs { inherit system overlays; };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            cargo-deny
            cargo-llvm-cov
            (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
          ];
        };
      }
    );
}
