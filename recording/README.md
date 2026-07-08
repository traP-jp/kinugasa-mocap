# Recording

Cluster-wide manifests and k3d helpers for the recording components.

```sh
nix run .#recording:up
nix run .#recording:port-forward
```

In another terminal:

```sh
nix run .#recording:health
nix run .#recording:crd
```

`nix run .#recording:up` builds `recording-server:dev`, creates a
`kinugasa-mocap` k3d cluster if needed, imports the image, applies the
`Stream` and `Take` CRDs, and starts the server in the cluster.
`nix run .#recording:port-forward`
exposes it on
`localhost:8080`.

```sh
nix run .#recording:down
```

Set `IMAGE`, `CLUSTER`, or `HOST_PORT` to override the defaults.

Sample resources live in `recording/config/samples`. They are not applied by
`recording:up`; apply them explicitly after creating the required S3 Secrets.

`recording:up` also deploys one in-cluster LiveKit stack in the
`recording-system` namespace:

- `livekit-server`
- `livekit-ingress`
- `livekit-redis`

The local k3d cluster exposes LiveKit HTTP on `localhost:7880` and RTMP ingress
on `localhost:1935`.

The recording operator creates one LiveKit WHIP ingress per `Stream` by using
the LiveKit Go SDK. The generated WHIP URL is stored in a Kubernetes Secret and
mounted into the relay Pod through an environment variable.

## Test Inputs

The k3d helper exposes UDP host ports for local test sources:

- SRT: host `9000` -> nodePort `30900`
- RIST: host `9001` -> nodePort `30901`

If the k3d cluster already existed before these mappings were added, recreate it
with `nix run .#recording:down` and `nix run .#recording:up`.

Apply one of the sample streams, then run the matching sender from the host:

```sh
kubectl apply -f recording/config/samples/stream.yaml
nix run .#recording:send-srt
```

```sh
kubectl apply -f recording/config/samples/stream-rist.yaml
nix run .#recording:send-rist
```

The senders generate a test pattern with FFmpeg and send MPEG-TS over SRT or
RIST. Set `DURATION=30` to send for a fixed number of seconds, or override
`SRT_URI` / `RIST_URI` to point at a different target.
