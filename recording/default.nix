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
          SRT_HOST_PORT=''${SRT_HOST_PORT:-9000}
          RIST_HOST_PORT=''${RIST_HOST_PORT:-9001}
          LIVEKIT_HOST_PORT=''${LIVEKIT_HOST_PORT:-7880}
          LIVEKIT_RTMP_HOST_PORT=''${LIVEKIT_RTMP_HOST_PORT:-1935}
          SRT_NODE_PORT=''${SRT_NODE_PORT:-30900}
          RIST_NODE_PORT=''${RIST_NODE_PORT:-30901}
          LIVEKIT_NODE_PORT=''${LIVEKIT_NODE_PORT:-30880}
          LIVEKIT_RTMP_NODE_PORT=''${LIVEKIT_RTMP_NODE_PORT:-31935}

          if k3d cluster list "$CLUSTER" >/dev/null 2>&1; then
            echo "k3d cluster $CLUSTER already exists"
            echo "test UDP and LiveKit port mappings are only added when the cluster is created; run recording:down and recording:up if they are missing"
          else
            k3d cluster create "$CLUSTER" \
              --port "$SRT_HOST_PORT:$SRT_NODE_PORT/udp@server:0" \
              --port "$RIST_HOST_PORT:$RIST_NODE_PORT/udp@server:0" \
              --port "$LIVEKIT_HOST_PORT:$LIVEKIT_NODE_PORT/tcp@server:0" \
              --port "$LIVEKIT_RTMP_HOST_PORT:$LIVEKIT_RTMP_NODE_PORT/tcp@server:0"
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

          kubectl apply -f "$GIT_ROOT/recording/config/crds"
          kubectl wait --for condition=Established crd/takes.recording.kinugasa.dev --timeout=60s
          kubectl wait --for condition=Established crd/streams.recording.kinugasa.dev --timeout=60s
          kubectl apply -f "$GIT_ROOT/recording/config/livekit.yaml"
          kubectl rollout status deployment/livekit-redis -n recording-system --timeout=120s
          kubectl rollout status deployment/livekit-server -n recording-system --timeout=180s
          kubectl rollout status deployment/livekit-ingress -n recording-system --timeout=180s
          kubectl apply -f "$GIT_ROOT/recording/config/server.yaml"
          kubectl set image deployment/recording-server "server=$IMAGE" -n recording-system
          kubectl rollout restart deployment/recording-server -n recording-system
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

      packages."recording:livekit-logs" = pkgs.writeShellApplication {
        name = "recording-livekit-logs";
        runtimeInputs = with pkgs; [ kubectl ];
        text = ''
          kubectl logs -n recording-system deployment/livekit-server -f
        '';
      };

      packages."recording:send-srt" = pkgs.writeShellApplication {
        name = "recording-send-srt";
        runtimeInputs = with pkgs; [ ffmpeg ];
        text = ''
          SRT_HOST=''${SRT_HOST:-127.0.0.1}
          SRT_HOST_PORT=''${SRT_HOST_PORT:-9000}
          SRT_URI=''${SRT_URI:-srt://$SRT_HOST:$SRT_HOST_PORT?mode=caller&latency=200000}
          VIDEO_SIZE=''${VIDEO_SIZE:-1280x720}
          VIDEO_RATE=''${VIDEO_RATE:-30}
          AUDIO_FREQUENCY=''${AUDIO_FREQUENCY:-1000}

          duration_args=()
          if [ -n "''${DURATION:-}" ]; then
            duration_args=(-t "$DURATION")
          fi

          ffmpeg -hide_banner -loglevel info -re \
            -f lavfi -i "testsrc2=size=$VIDEO_SIZE:rate=$VIDEO_RATE" \
            -f lavfi -i "sine=frequency=$AUDIO_FREQUENCY:sample_rate=48000" \
            "''${duration_args[@]}" \
            -map 0:v:0 -map 1:a:0 \
            -c:v libx264 -preset veryfast -tune zerolatency -pix_fmt yuv420p -g "$((VIDEO_RATE * 2))" \
            -c:a aac -b:a 128k \
            -f mpegts "$SRT_URI"
        '';
      };

      packages."recording:send-rist" = pkgs.writeShellApplication {
        name = "recording-send-rist";
        runtimeInputs = with pkgs; [ ffmpeg ];
        text = ''
          RIST_HOST=''${RIST_HOST:-127.0.0.1}
          RIST_HOST_PORT=''${RIST_HOST_PORT:-9001}
          RIST_URI=''${RIST_URI:-rist://$RIST_HOST:$RIST_HOST_PORT}
          VIDEO_SIZE=''${VIDEO_SIZE:-1280x720}
          VIDEO_RATE=''${VIDEO_RATE:-30}
          AUDIO_FREQUENCY=''${AUDIO_FREQUENCY:-1000}

          duration_args=()
          if [ -n "''${DURATION:-}" ]; then
            duration_args=(-t "$DURATION")
          fi

          ffmpeg -hide_banner -loglevel info -re \
            -f lavfi -i "testsrc2=size=$VIDEO_SIZE:rate=$VIDEO_RATE" \
            -f lavfi -i "sine=frequency=$AUDIO_FREQUENCY:sample_rate=48000" \
            "''${duration_args[@]}" \
            -map 0:v:0 -map 1:a:0 \
            -c:v libx264 -preset veryfast -tune zerolatency -pix_fmt yuv420p -g "$((VIDEO_RATE * 2))" \
            -c:a aac -b:a 128k \
            -f mpegts "$RIST_URI"
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
      apps."recording:livekit-logs" = {
        type = "app";
        program = "${config.packages."recording:livekit-logs"}/bin/recording-livekit-logs";
      };
      apps."recording:send-srt" = {
        type = "app";
        program = "${config.packages."recording:send-srt"}/bin/recording-send-srt";
      };
      apps."recording:send-rist" = {
        type = "app";
        program = "${config.packages."recording:send-rist"}/bin/recording-send-rist";
      };
      apps."recording:down" = {
        type = "app";
        program = "${config.packages."recording:down"}/bin/recording-down";
      };
    };
}
