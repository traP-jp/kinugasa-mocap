package presentation

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/comavius/kinugasa-mocap/recording/server/domain"
	"github.com/comavius/kinugasa-mocap/recording/server/infra/k8s"
	"github.com/comavius/kinugasa-mocap/recording/server/service"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/types"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/client/fake"
)

func TestHealthz(t *testing.T) {
	recorder := httptest.NewRecorder()
	request := httptest.NewRequest(http.MethodGet, "/healthz", nil)

	newTestServer(t).Handler().ServeHTTP(recorder, request)

	if recorder.Code != http.StatusOK {
		t.Fatalf("status = %d, want %d", recorder.Code, http.StatusOK)
	}
	if recorder.Body.String() != "ok\n" {
		t.Fatalf("body = %q, want %q", recorder.Body.String(), "ok\n")
	}
}

func TestCameraAndTakeAPI(t *testing.T) {
	server, k8sClient := newTestServerWithClient(t)

	cameraRecorder := httptest.NewRecorder()
	cameraRequest := jsonRequest(t, http.MethodPost, "/cameras", map[string]any{
		"displayName": "Studio A",
		"protocol":    "srt",
	})
	server.Handler().ServeHTTP(cameraRecorder, cameraRequest)

	if cameraRecorder.Code != http.StatusCreated {
		t.Fatalf("create camera status = %d, body = %s", cameraRecorder.Code, cameraRecorder.Body.String())
	}
	var camera struct {
		ID       string `json:"id"`
		Endpoint struct {
			URL        string `json:"url"`
			QRCodeText string `json:"qrCodeText"`
		} `json:"endpoint"`
		LiveKit struct {
			Room string `json:"room"`
		} `json:"liveKit"`
	}
	decodeJSON(t, cameraRecorder, &camera)
	if camera.ID != "studio-a" {
		t.Fatalf("camera id = %q, want studio-a", camera.ID)
	}
	if !strings.HasPrefix(camera.Endpoint.URL, "srt://capture.example.com:") {
		t.Fatalf("camera endpoint = %q, want capture.example.com srt URL", camera.Endpoint.URL)
	}
	if camera.Endpoint.QRCodeText != camera.Endpoint.URL {
		t.Fatalf("qrCodeText = %q, want endpoint URL", camera.Endpoint.QRCodeText)
	}
	var stream domain.Stream
	if err := k8sClient.Get(t.Context(), types.NamespacedName{Name: camera.ID, Namespace: "default"}, &stream); err != nil {
		t.Fatalf("get persisted stream: %v", err)
	}
	if stream.Spec.DisplayName != "Studio A" {
		t.Fatalf("stream displayName = %q, want Studio A", stream.Spec.DisplayName)
	}
	if stream.Spec.Input.Port != 9000 {
		t.Fatalf("stream input port = %d, want 9000", stream.Spec.Input.Port)
	}

	endpointRecorder := httptest.NewRecorder()
	endpointRequest := httptest.NewRequest(http.MethodGet, "/cameras/"+camera.ID+"/endpoint", nil)
	server.Handler().ServeHTTP(endpointRecorder, endpointRequest)
	if endpointRecorder.Code != http.StatusOK {
		t.Fatalf("get endpoint status = %d, body = %s", endpointRecorder.Code, endpointRecorder.Body.String())
	}

	takeRecorder := httptest.NewRecorder()
	takeRequest := jsonRequest(t, http.MethodPost, "/takes", map[string]any{
		"cameraId": camera.ID,
		"output": map[string]any{
			"s3": map[string]any{
				"bucket": "mocap-recordings",
			},
		},
	})
	server.Handler().ServeHTTP(takeRecorder, takeRequest)
	if takeRecorder.Code != http.StatusCreated {
		t.Fatalf("start take status = %d, body = %s", takeRecorder.Code, takeRecorder.Body.String())
	}
	var take struct {
		ID       string `json:"id"`
		CameraID string `json:"cameraId"`
		Phase    string `json:"phase"`
	}
	decodeJSON(t, takeRecorder, &take)
	if take.CameraID != camera.ID {
		t.Fatalf("take camera id = %q, want %q", take.CameraID, camera.ID)
	}
	if take.Phase != "pending" {
		t.Fatalf("take phase = %q, want pending", take.Phase)
	}
	var takeCR domain.Take
	if err := k8sClient.Get(t.Context(), types.NamespacedName{Name: take.ID, Namespace: "default"}, &takeCR); err != nil {
		t.Fatalf("get persisted take: %v", err)
	}
	if takeCR.Spec.Output.S3.Bucket != "mocap-recordings" {
		t.Fatalf("take bucket = %q, want mocap-recordings", takeCR.Spec.Output.S3.Bucket)
	}

	stopRecorder := httptest.NewRecorder()
	stopRequest := jsonRequest(t, http.MethodPost, "/takes/"+take.ID+"/stop", map[string]any{})
	server.Handler().ServeHTTP(stopRecorder, stopRequest)
	if stopRecorder.Code != http.StatusOK {
		t.Fatalf("stop take status = %d, body = %s", stopRecorder.Code, stopRecorder.Body.String())
	}
	var stopped struct {
		Phase string `json:"phase"`
	}
	decodeJSON(t, stopRecorder, &stopped)
	if stopped.Phase != "stop_requested" {
		t.Fatalf("stopped take phase = %q, want stop_requested", stopped.Phase)
	}
	if err := k8sClient.Get(t.Context(), types.NamespacedName{Name: take.ID, Namespace: "default"}, &takeCR); err != nil {
		t.Fatalf("get stopped take: %v", err)
	}
	if takeCR.Spec.StopRequestedAt == nil {
		t.Fatalf("take stopRequestedAt was not persisted")
	}
}

func newTestServer(t *testing.T) *APIServer {
	t.Helper()
	server, _ := newTestServerWithClient(t)
	return server
}

func newTestServerWithClient(t *testing.T) (*APIServer, client.Client) {
	t.Helper()
	scheme := runtime.NewScheme()
	if err := k8s.AddToScheme(scheme); err != nil {
		t.Fatal(err)
	}
	k8sClient := fake.NewClientBuilder().WithScheme(scheme).Build()
	server, err := NewAPIServer(":0", service.NewKubernetesUsecase(k8sClient, service.UsecaseConfig{
		PublicIngestHost: "capture.example.com",
		LiveKitURL:       "ws://livekit.example.com",
	}))
	if err != nil {
		t.Fatal(err)
	}
	return server, k8sClient
}

func jsonRequest(t *testing.T, method string, target string, body any) *http.Request {
	t.Helper()
	var buf bytes.Buffer
	if err := json.NewEncoder(&buf).Encode(body); err != nil {
		t.Fatal(err)
	}
	request := httptest.NewRequest(method, target, &buf)
	request.Header.Set("Content-Type", "application/json")
	return request
}

func decodeJSON(t *testing.T, recorder *httptest.ResponseRecorder, output any) {
	t.Helper()
	if err := json.NewDecoder(recorder.Body).Decode(output); err != nil {
		t.Fatalf("decode response: %v; body = %s", err, recorder.Body.String())
	}
}
