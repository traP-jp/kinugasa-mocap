package config

import (
	"bytes"
	"embed"
	"io/fs"
	"log"
)

//go:embed crds/*.yaml
var crdFiles embed.FS

func CRDManifest() []byte {
	manifest, err := crdManifest()
	if err != nil {
		log.Panicf("read bundled CRDs: %v", err)
	}
	return manifest
}

func RecordingCRDManifest() []byte {
	return CRDManifest()
}

func crdManifest() ([]byte, error) {
	entries, err := fs.ReadDir(crdFiles, "crds")
	if err != nil {
		return nil, err
	}

	manifests := make([][]byte, 0, len(entries))
	for _, entry := range entries {
		if entry.IsDir() {
			continue
		}
		manifest, err := crdFiles.ReadFile("crds/" + entry.Name())
		if err != nil {
			return nil, err
		}
		manifests = append(manifests, bytes.TrimSpace(manifest))
	}

	return append(bytes.Join(manifests, []byte("\n---\n")), '\n'), nil
}
