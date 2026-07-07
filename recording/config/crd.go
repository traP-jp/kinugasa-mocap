package config

import _ "embed"

//go:embed crd.yaml
var recordingCRD []byte

func RecordingCRDManifest() []byte {
	return append([]byte(nil), recordingCRD...)
}
