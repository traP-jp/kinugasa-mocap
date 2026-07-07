package presentation

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/comavius/kinugasa-mocap/recording/config"
	"github.com/comavius/kinugasa-mocap/recording/server/service"
)

func TestHealthz(t *testing.T) {
	recorder := httptest.NewRecorder()
	request := httptest.NewRequest(http.MethodGet, "/healthz", nil)

	newTestServer().Router().ServeHTTP(recorder, request)

	if recorder.Code != http.StatusOK {
		t.Fatalf("status = %d, want %d", recorder.Code, http.StatusOK)
	}
	if recorder.Body.String() != "ok\n" {
		t.Fatalf("body = %q, want %q", recorder.Body.String(), "ok\n")
	}
}

func TestCRD(t *testing.T) {
	recorder := httptest.NewRecorder()
	request := httptest.NewRequest(http.MethodGet, "/crd", nil)

	newTestServer().Router().ServeHTTP(recorder, request)

	if recorder.Code != http.StatusOK {
		t.Fatalf("status = %d, want %d", recorder.Code, http.StatusOK)
	}
	if contentType := recorder.Header().Get("Content-Type"); !strings.HasPrefix(contentType, "application/yaml") {
		t.Fatalf("content type = %q, want application/yaml", contentType)
	}

	body := recorder.Body.String()
	for _, want := range []string{
		"kind: CustomResourceDefinition",
		"name: recordings.recording.kinugasa.dev",
		"kind: Recording",
	} {
		if !strings.Contains(body, want) {
			t.Fatalf("CRD body does not contain %q:\n%s", want, body)
		}
	}
}

func newTestServer() *APIServer {
	return NewAPIServer(":0", service.NewCRDService(config.RecordingCRDManifest()))
}
