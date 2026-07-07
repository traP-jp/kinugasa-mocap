package k8s

import (
	"strings"
	"testing"

	"github.com/comavius/kinugasa-mocap/recording/server/domain"
	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/types"
)

func TestBuildRecordingJob(t *testing.T) {
	recording := &domain.Recording{
		ObjectMeta: metav1.ObjectMeta{
			Name:      "session-1",
			Namespace: "default",
			UID:       types.UID("recording-uid"),
		},
		Spec: domain.RecordingSpec{
			StreamRef: domain.LocalObjectReference{Name: "studio"},
			Output: domain.RecordingOutputSpec{
				S3: domain.S3OutputSpec{
					Bucket:    "mocap-recordings",
					ObjectKey: "recordings/session-1.mp4",
					SecretRef: &domain.S3SecretReference{
						Name: "s3-credentials",
					},
				},
			},
		},
	}
	stream := &domain.Stream{
		ObjectMeta: metav1.ObjectMeta{
			Name:      "studio",
			Namespace: "default",
		},
		Spec: domain.StreamSpec{
			Recording: domain.StreamRecordingSpec{Protocol: "srt", Port: 10000},
		},
	}

	job := BuildRecordingJob(recording, stream, RecordingJobOptions{
		RecorderImage: "recorder:dev",
		UploaderImage: "rclone:dev",
	})

	if job.Name != "recording-session-1" {
		t.Fatalf("job name = %q, want recording-session-1", job.Name)
	}
	if job.Spec.BackoffLimit == nil || *job.Spec.BackoffLimit != 0 {
		t.Fatalf("backoff limit = %v, want 0", job.Spec.BackoffLimit)
	}
	if job.Spec.Template.Spec.RestartPolicy != corev1.RestartPolicyNever {
		t.Fatalf("restart policy = %q, want Never", job.Spec.Template.Spec.RestartPolicy)
	}
	if job.Spec.Template.Spec.ShareProcessNamespace == nil || !*job.Spec.Template.Spec.ShareProcessNamespace {
		t.Fatalf("share process namespace = %v, want true", job.Spec.Template.Spec.ShareProcessNamespace)
	}

	if len(job.Spec.Template.Spec.Containers) != 2 {
		t.Fatalf("container count = %d, want 2", len(job.Spec.Template.Spec.Containers))
	}
	recorder := job.Spec.Template.Spec.Containers[0]
	uploader := job.Spec.Template.Spec.Containers[1]
	if recorder.Name != "recorder" {
		t.Fatalf("first container = %q, want recorder", recorder.Name)
	}
	if uploader.Name != "uploader" {
		t.Fatalf("second container = %q, want uploader", uploader.Name)
	}

	if got := recorder.Command; len(got) != 2 || got[0] != "/bin/sh" || got[1] != "-ec" {
		t.Fatalf("recorder command = %#v, want /bin/sh -ec", got)
	}
	for _, want := range []string{
		"ffmpeg -hide_banner -loglevel info",
		"-i \"${STREAM_URL}\"",
		"output=\"/recording/data/recording.mp4\"",
		"printf '%s\\n' \"${ffmpeg_pid}\" > \"${pid_file}\"",
		"touch \"${complete}\"",
		"kill -TERM \"${ffmpeg_pid}\"",
	} {
		if !containsSubstring(recorder.Args[0], want) {
			t.Fatalf("recorder script = %q, want substring %q", recorder.Args[0], want)
		}
	}
	if !hasEnv(recorder.Env, "STREAM_URL") {
		t.Fatalf("recorder env = %#v, want STREAM_URL", recorder.Env)
	}
	if hasEnv(recorder.Env, "S3_BUCKET") {
		t.Fatalf("recorder env = %#v, want no S3 env", recorder.Env)
	}
	if len(recorder.VolumeMounts) != 2 {
		t.Fatalf("recorder volume mounts = %d, want 2", len(recorder.VolumeMounts))
	}
	if got := uploader.Command; len(got) != 2 || got[0] != "/bin/sh" || got[1] != "-ec" {
		t.Fatalf("uploader command = %#v, want /bin/sh -ec", got)
	}
	for _, want := range []string{
		"rclone copyto \"${input}\" \"${remote}\"",
		"provider=AWS,env_auth=true",
		"${S3_BUCKET}/${S3_OBJECT_KEY}",
		"recorder did not start within ${start_timeout}s",
		"recorder exited without completion marker",
	} {
		if !containsSubstring(uploader.Args[0], want) {
			t.Fatalf("uploader script = %q, want substring %q", uploader.Args[0], want)
		}
	}
	for _, want := range []string{
		"S3_BUCKET",
		"S3_OBJECT_KEY",
		"AWS_ACCESS_KEY_ID",
		"AWS_SECRET_ACCESS_KEY",
	} {
		if !hasEnv(uploader.Env, want) {
			t.Fatalf("uploader env = %#v, want %q", uploader.Env, want)
		}
	}
	if len(uploader.VolumeMounts) != 1 {
		t.Fatalf("uploader volume mounts = %d, want 1", len(uploader.VolumeMounts))
	}
	if len(job.Spec.Template.Spec.Volumes) != 2 {
		t.Fatalf("volumes = %d, want 2", len(job.Spec.Template.Spec.Volumes))
	}
}

func TestBuildStreamWorkload(t *testing.T) {
	stream := &domain.Stream{
		ObjectMeta: metav1.ObjectMeta{
			Name:      "studio",
			Namespace: "default",
			UID:       types.UID("stream-uid"),
		},
		Spec: domain.StreamSpec{
			Input: domain.StreamInputSpec{
				Protocol: "srt",
				URI:      "srt://:9000?mode=listener",
				Port:     9000,
				NodePort: 30900,
			},
			LiveKit: domain.LiveKitSpec{
				URL:            "wss://livekit.example.com",
				Room:           "room-a",
				TokenSecretRef: domain.SecretKeyReference{Name: "livekit-token", Key: "token"},
			},
			Recording: domain.StreamRecordingSpec{
				Protocol: "srt",
				Port:     10000,
			},
		},
	}

	deployment := BuildStreamDeployment(stream, StreamWorkloadOptions{RelayImage: "relay:dev"})
	service := BuildStreamService(stream)

	if deployment.Name != "stream-studio" {
		t.Fatalf("deployment name = %q, want stream-studio", deployment.Name)
	}
	container := deployment.Spec.Template.Spec.Containers[0]
	if got := container.Command; len(got) != 2 || got[0] != "/bin/sh" || got[1] != "-ec" {
		t.Fatalf("relay command = %#v, want /bin/sh -ec", got)
	}
	for _, want := range []string{
		"ffmpeg -hide_banner -loglevel info",
		"-i \"${INPUT_URI}\"",
		"auth_headers=\"-headers Authorization: Bearer ${LIVEKIT_TOKEN}\"",
	} {
		if !containsSubstring(container.Args[0], want) {
			t.Fatalf("relay script = %q, want substring %q", container.Args[0], want)
		}
	}
	for _, want := range []string{
		"INPUT_URI",
		"LIVEKIT_OUTPUT_URI",
		"LIVEKIT_OUTPUT_OPTIONS",
		"RECORDING_OUTPUT_URI",
		"LIVEKIT_TOKEN",
	} {
		if !hasEnv(container.Env, want) {
			t.Fatalf("relay env = %#v, want %q", container.Env, want)
		}
	}
	if service.Spec.Ports[0].Protocol != corev1.ProtocolUDP {
		t.Fatalf("recording service protocol = %q, want UDP", service.Spec.Ports[0].Protocol)
	}
	if service.Spec.Type != corev1.ServiceTypeNodePort {
		t.Fatalf("stream service type = %q, want NodePort", service.Spec.Type)
	}
	if len(service.Spec.Ports) != 2 {
		t.Fatalf("stream service ports = %d, want 2", len(service.Spec.Ports))
	}
	if service.Spec.Ports[1].NodePort != 30900 {
		t.Fatalf("input node port = %d, want 30900", service.Spec.Ports[1].NodePort)
	}
}

func TestBuildStreamWorkloadWithMockLiveKit(t *testing.T) {
	stream := &domain.Stream{
		ObjectMeta: metav1.ObjectMeta{
			Name:      "studio",
			Namespace: "default",
			UID:       types.UID("stream-uid"),
		},
		Spec: domain.StreamSpec{
			Input: domain.StreamInputSpec{
				Protocol: "srt",
				URI:      "srt://:9000?mode=listener",
			},
			LiveKit: domain.LiveKitSpec{
				Mock: true,
				Room: "room-a",
			},
		},
	}

	deployment := BuildStreamDeployment(stream, StreamWorkloadOptions{RelayImage: "relay:dev"})
	container := deployment.Spec.Template.Spec.Containers[0]

	if hasEnv(container.Env, "LIVEKIT_TOKEN") {
		t.Fatalf("relay env = %#v, want no LIVEKIT_TOKEN in mock mode", container.Env)
	}
	if value := envValue(container.Env, "LIVEKIT_OUTPUT_URI"); value != "/dev/null" {
		t.Fatalf("mock livekit output uri = %q, want /dev/null", value)
	}
	if value := envValue(container.Env, "LIVEKIT_OUTPUT_OPTIONS"); value != "[f=null]" {
		t.Fatalf("mock livekit output options = %q, want [f=null]", value)
	}
}

func contains(items []string, want string) bool {
	for _, item := range items {
		if item == want {
			return true
		}
	}
	return false
}

func containsSubstring(value, want string) bool {
	return strings.Contains(value, want)
}

func hasEnv(items []corev1.EnvVar, name string) bool {
	for _, item := range items {
		if item.Name == name {
			return true
		}
	}
	return false
}

func envValue(items []corev1.EnvVar, name string) string {
	for _, item := range items {
		if item.Name == name {
			return item.Value
		}
	}
	return ""
}
