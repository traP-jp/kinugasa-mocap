{
  description = "kinugasa-mocap development flake";

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{
      flake-parts,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      imports = [
        ./docs
        ./treefmt.nix
        ./rust-toolchain.nix
      ];

      perSystem =
        {
          pkgs,
          config,
          ...
        }:
        {
          devShells.default = pkgs.mkShell {
            name = "devshell";

            packages = with pkgs; [
              cargo-hakari
              nodejs
              pnpm
              treefmt
              cargo-tarpaulin
              config.packages."ci:treefmt:sync"
              config.packages.rust-toolchain
            ];

            shellHook = ''
              treefmt-sync
            '';
          };
        };
    };
}
