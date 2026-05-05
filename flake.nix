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
    librist-src = {
      url = "git+https://code.videolan.org/rist/librist.git";
      flake = false;
    };
    systems.url = "github:nix-systems/default";
  };

  outputs =
    inputs@{
      flake-parts,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = import inputs.systems;

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

              libclang.lib
              meson
              pkg-config
              ninja
              # PlantUML
              graphviz
              plantuml
              jdk21

              config.packages."ci:treefmt:sync"
              config.packages.rust-toolchain
            ];

            shellHook = ''
              treefmt-sync
              export JAVA_HOME="${pkgs.jdk21}/lib/openjdk"
              export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"
              export LIBRIST_SRC="${inputs.librist-src}"
            '';
          };
        };
    };
}
