# Recording Server

Minimal Go application for the recording server.

## Structure

```text
domain/             Recording CRD domain metadata
service/            application use cases
presentation/       Echo HTTP API
main.go             wiring and startup
```

The CRD manifest is embedded by the sibling `recording/config` Go module and
wired through the repository root `go.work`.

## Local Run

```sh
go run .
```

The server exposes:

- `GET /healthz`
- `GET /crd`

The bundled empty Kubernetes CRD can also be printed directly:

```sh
go run . --print-crd
```

## Cluster Run

Cluster-wide manifests and k3d helpers live one directory up in `recording/`.
