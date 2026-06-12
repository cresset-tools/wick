{
  description = "wick dev environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        # `nix develop` gives contributors the few host tools wick's tests
        # and scripts assume. The Rust toolchain itself is NOT provided here
        # — rustup / your host toolchain own that (rust-toolchain.toml pins
        # the channel); this shell only bridges the gap for people who don't
        # have a PHP on the host to eyeball formatter output against.
        devShells.default = pkgs.mkShell {
          packages = [
            pkgs.php       # sanity-check formatted output actually runs
            pkgs.bash      # scripts use `set -euo pipefail`
            pkgs.coreutils # `mktemp -d`, consistent across macOS/Linux
          ];
        };
      });
}
