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
`kinugasa-mocap` k3d cluster if needed, imports the image, applies the empty
CRD, and starts the server in the cluster. `nix run .#recording:port-forward`
exposes it on
`localhost:8080`.

```sh
nix run .#recording:down
```

Set `IMAGE`, `CLUSTER`, or `HOST_PORT` to override the defaults.
