{
  perSystem =
    {
      config,
      pkgs,
      ...
    }:
    {
      packages."recording:image" = pkgs.writeShellApplication {
        name = "recording-image";
        runtimeInputs = with pkgs; [
          docker
          git
        ];
        text = ''
          GIT_ROOT=$(git rev-parse --show-toplevel)
          IMAGE=''${IMAGE:-recording-server:dev}

          docker build -f "$GIT_ROOT/recording/server/Dockerfile" -t "$IMAGE" "$GIT_ROOT"
        '';
      };

      packages."recording:cluster" = pkgs.writeShellApplication {
        name = "recording-cluster";
        runtimeInputs = with pkgs; [ k3d ];
        text = ''
          CLUSTER=''${CLUSTER:-kinugasa-mocap}

          if k3d cluster list "$CLUSTER" >/dev/null 2>&1; then
            echo "k3d cluster $CLUSTER already exists"
          else
            k3d cluster create "$CLUSTER"
          fi
        '';
      };

      packages."recording:load" = pkgs.writeShellApplication {
        name = "recording-load";
        runtimeInputs = with pkgs; [ k3d ];
        text = ''
          CLUSTER=''${CLUSTER:-kinugasa-mocap}
          IMAGE=''${IMAGE:-recording-server:dev}

          k3d image import "$IMAGE" --cluster "$CLUSTER"
        '';
      };

      packages."recording:deploy" = pkgs.writeShellApplication {
        name = "recording-deploy";
        runtimeInputs = with pkgs; [
          git
          kubectl
        ];
        text = ''
          GIT_ROOT=$(git rev-parse --show-toplevel)
          IMAGE=''${IMAGE:-recording-server:dev}

          kubectl apply -f "$GIT_ROOT/recording/config/crd.yaml"
          kubectl wait --for condition=Established crd/recordings.recording.kinugasa.dev --timeout=60s
          kubectl apply -f "$GIT_ROOT/recording/config/server.yaml"
          kubectl set image deployment/recording-server "server=$IMAGE" -n recording-system
          kubectl rollout status deployment/recording-server -n recording-system --timeout=120s
        '';
      };

      packages."recording:up" = pkgs.writeShellApplication {
        name = "recording-up";
        text = ''
          ${config.packages."recording:image"}/bin/recording-image
          ${config.packages."recording:cluster"}/bin/recording-cluster
          ${config.packages."recording:load"}/bin/recording-load
          ${config.packages."recording:deploy"}/bin/recording-deploy
        '';
      };

      packages."recording:status" = pkgs.writeShellApplication {
        name = "recording-status";
        runtimeInputs = with pkgs; [ kubectl ];
        text = ''
          kubectl rollout status deployment/recording-server -n recording-system --timeout=120s
        '';
      };

      packages."recording:port-forward" = pkgs.writeShellApplication {
        name = "recording-port-forward";
        runtimeInputs = with pkgs; [ kubectl ];
        text = ''
          HOST_PORT=''${HOST_PORT:-8080}

          kubectl port-forward -n recording-system svc/recording-server "$HOST_PORT":8080
        '';
      };

      packages."recording:health" = pkgs.writeShellApplication {
        name = "recording-health";
        runtimeInputs = with pkgs; [ curl ];
        text = ''
          HOST_PORT=''${HOST_PORT:-8080}

          curl -s "http://localhost:$HOST_PORT/healthz"
        '';
      };

      packages."recording:crd" = pkgs.writeShellApplication {
        name = "recording-crd";
        runtimeInputs = with pkgs; [ curl ];
        text = ''
          HOST_PORT=''${HOST_PORT:-8080}

          curl -s "http://localhost:$HOST_PORT/crd"
        '';
      };

      packages."recording:logs" = pkgs.writeShellApplication {
        name = "recording-logs";
        runtimeInputs = with pkgs; [ kubectl ];
        text = ''
          kubectl logs -n recording-system deployment/recording-server -f
        '';
      };

      packages."recording:down" = pkgs.writeShellApplication {
        name = "recording-down";
        runtimeInputs = with pkgs; [ k3d ];
        text = ''
          CLUSTER=''${CLUSTER:-kinugasa-mocap}

          k3d cluster delete "$CLUSTER"
        '';
      };

      apps."recording:image" = {
        type = "app";
        program = "${config.packages."recording:image"}/bin/recording-image";
      };
      apps."recording:cluster" = {
        type = "app";
        program = "${config.packages."recording:cluster"}/bin/recording-cluster";
      };
      apps."recording:load" = {
        type = "app";
        program = "${config.packages."recording:load"}/bin/recording-load";
      };
      apps."recording:deploy" = {
        type = "app";
        program = "${config.packages."recording:deploy"}/bin/recording-deploy";
      };
      apps."recording:up" = {
        type = "app";
        program = "${config.packages."recording:up"}/bin/recording-up";
      };
      apps."recording:status" = {
        type = "app";
        program = "${config.packages."recording:status"}/bin/recording-status";
      };
      apps."recording:port-forward" = {
        type = "app";
        program = "${config.packages."recording:port-forward"}/bin/recording-port-forward";
      };
      apps."recording:health" = {
        type = "app";
        program = "${config.packages."recording:health"}/bin/recording-health";
      };
      apps."recording:crd" = {
        type = "app";
        program = "${config.packages."recording:crd"}/bin/recording-crd";
      };
      apps."recording:logs" = {
        type = "app";
        program = "${config.packages."recording:logs"}/bin/recording-logs";
      };
      apps."recording:down" = {
        type = "app";
        program = "${config.packages."recording:down"}/bin/recording-down";
      };
    };
}
