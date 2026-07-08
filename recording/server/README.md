# Recording Server

Minimal Go application for the recording server.

## Structure

```text
domain/             Take and Stream CRD domain metadata
service/            application use cases
presentation/       Echo HTTP API
main.go             wiring and startup
```

The CRD manifests are embedded by the sibling `recording/config` Go module and
wired through the repository root `go.work`.

## Local Run

```sh
go run .
```

The server exposes:

- `GET /healthz`
- camera, take, and LiveKit endpoints generated from `recording/openapi.yaml`

The bundled Kubernetes CRDs can also be printed directly:

```sh
go run . --print-crd
```

The bundle currently contains:

- `Stream`: receives an RIST/SRT input, relays it to LiveKit continuously, and exposes a take endpoint for one-shot recorder Pods.
- `Take`: represents one start/stop capture attempt. The server usecase sets the S3 object key, and the operator reconciles it into a non-restarting Job.

## Operator Mode

```sh
go run . --enable-operator
```

Operator mode starts the HTTP API and reconciles:

- `Stream` into a LiveKit WHIP ingress, a Secret containing the generated WHIP URL, a single-replica relay Deployment, and a UDP Service. The relay container runs FFmpeg and forwards to LiveKit continuously.
- `Take` into a Job with `restartPolicy: Never` and `backoffLimit: 0`. The Pod has a recorder container and an uploader container. The recorder runs FFmpeg into a shared `emptyDir`; the uploader runs rclone and uploads that file to the server-selected S3 object key from `spec.output.s3.objectKey`.

If `Stream.spec.livekit.url` is set, the operator treats that URL as an
externally managed LiveKit output and skips LiveKit ingress creation.

Stop requests are written to `Take.spec.stopRequestedAt`. The operator patches the active recorder Pod annotation and exposes it through a Downward API file at `/var/run/kinugasa/take-control/stop-requested-at`, so the recorder can finalize and upload before exiting.

The relay image must provide `/bin/sh` and `ffmpeg`. The recorder image must
provide `/bin/sh` and `ffmpeg`. The uploader image must provide `/bin/sh` and
`rclone`. Missing binaries or unsupported protocols are treated as runtime
errors; the operator does not fall back to a custom uploader.

Images can be set with:

```sh
go run . --enable-operator \
  --stream-relay-image=linuxserver/ffmpeg:latest \
  --recording-recorder-image=linuxserver/ffmpeg:latest \
  --recording-uploader-image=rclone/rclone:latest \
  --livekit-url=http://livekit-server.recording-system.svc.cluster.local:7880 \
  --livekit-public-url=wss://livekit.example.com \
  --livekit-api-key=devkey \
  --livekit-api-secret=secret \
  --livekit-whip-base-url=http://livekit-ingress.recording-system.svc.cluster.local:8080/whip
```

`--livekit-public-url` is returned to frontend clients. LiveKit connection
tokens are issued on demand from the configured API key and secret.

## Cluster Run

Cluster-wide manifests and k3d helpers live one directory up in `recording/`.
