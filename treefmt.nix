{ inputs, ... }:
{
  imports = [
    inputs.treefmt-nix.flakeModule
  ];

  perSystem =
    {
      pkgs,
      config,
      ...
    }:
    {
      treefmt = {
        projectRootFile = "flake.nix";

        programs.biome = {
          enable = true;
          formatCommand = "format";
        };
        programs.nixfmt.enable = true;
        programs.rustfmt = {
          enable = true;
          package = config.packages.rust-toolchain;
        };
        settings.formatter.biome.includes = [
          "*.ts"
          "*.tsx"
          "*.md"
        ];
      };

      packages."ci:treefmt:sync" = pkgs.writeShellApplication {
        name = "treefmt-sync";
        text = ''
          echo "Updating treefmt.toml ..." >&2
          GIT_ROOT=$(git rev-parse --show-toplevel)
            cp -f "$TREEFMT_CONFIG_FILE" "$GIT_ROOT"/treefmt.toml
            echo "Updated treefmt.toml ..." >&2
        '';
        runtimeInputs = with pkgs; [ git ];
        runtimeEnv = {
          TREEFMT_CONFIG_FILE = config.treefmt.build.configFile;
        };
      };
      packages."ci:treefmt:check" = pkgs.writeShellApplication {
        name = "treefmt-check";
        text = ''
          echo "Checking formatting ..." >&2
          treefmt --ci --config-file "$TREEFMT_CONFIG_FILE"
        '';
        runtimeInputs = with pkgs; [ treefmt ];
        runtimeEnv = {
          TREEFMT_CONFIG_FILE = config.treefmt.build.configFile;
        };
      };
    };
}
