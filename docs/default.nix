{ ... }:

{
  perSystem =
    { pkgs, ... }:
    let
      create-design-note = pkgs.writeShellApplication {
        name = "create-design-note";
        runtimeInputs = [
          pkgs.coreutils
        ];
        text = ''
          if [ "$#" -ne 1 ]; then
            echo "usage: nix run .#dev:create-design-note <short-description>" >&2
            exit 2
          fi

          short_description="$1"

          case "$short_description" in
            "" | */* | *\\*)
              echo "short-description must be a single path segment" >&2
              exit 2
              ;;
          esac

          template="docs/templates/design-note.md"
          design_notes_dir="docs/design-notes"
          ongoing_dir="docs/design-notes-ongoing"

          if [ ! -f "$template" ]; then
            echo "template not found: $template" >&2
            exit 1
          fi

          yymmdd="$(date +%y%m%d)"
          note_name="''${yymmdd}_''${short_description}.md"
          note_path="''${design_notes_dir}/''${note_name}"
          ongoing_path="''${ongoing_dir}/''${short_description}.md"

          mkdir -p "$design_notes_dir" "$ongoing_dir"

          if [ -e "$note_path" ]; then
            echo "design note already exists: $note_path" >&2
            exit 1
          fi

          if [ -e "$ongoing_path" ] || [ -L "$ongoing_path" ]; then
            echo "ongoing symlink already exists: $ongoing_path" >&2
            exit 1
          fi

          cp "$template" "$note_path"
          ln -s "../design-notes/$note_name" "$ongoing_path"

          echo "$note_path"
          echo "$ongoing_path"
        '';
      };

      complete-design-note = pkgs.writeShellApplication {
        name = "complete-design-note";
        runtimeInputs = [
          pkgs.coreutils
        ];
        text = ''
          if [ "$#" -ne 1 ]; then
            echo "usage: nix run .#dev:complete-design-note <short-description>" >&2
            exit 2
          fi

          short_description="$1"

          case "$short_description" in
            "" | */* | *\\*)
              echo "short-description must be a single path segment" >&2
              exit 2
              ;;
          esac

          ongoing_path="docs/design-notes-ongoing/''${short_description}.md"

          if [ -L "$ongoing_path" ]; then
            rm "$ongoing_path"
            echo "removed $ongoing_path"
          elif [ -e "$ongoing_path" ]; then
            echo "refusing to remove non-symlink: $ongoing_path" >&2
            exit 1
          else
            echo "ongoing symlink not found: $ongoing_path" >&2
            exit 1
          fi
        '';
      };
    in
    {
      apps = {
        "dev:create-design-note" = {
          type = "app";
          program = "${create-design-note}/bin/create-design-note";
          meta.description = "Create a design note from docs/templates/design-note.md and add an ongoing symlink";
        };

        "dev:complete-design-note" = {
          type = "app";
          program = "${complete-design-note}/bin/complete-design-note";
          meta.description = "Remove the ongoing symlink for a completed design note";
        };
      };
    };
}
