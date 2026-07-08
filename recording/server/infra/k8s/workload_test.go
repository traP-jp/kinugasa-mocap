package k8s

import (
	"strings"
	"testing"

	"github.com/comavius/kinugasa-mocap/recording/server/domain"
	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/types"
)

func TestBuildTakeJob(t *testing.T) {
	take := &domain.Take{
		ObjectMeta: metav1.ObjectMeta{
			Name:      "take-1",
			Namespace: "default",
			UID:       types.UID("take-uid"),
		},
		Spec: domain.TakeSpec{
			StreamRef: domain.LocalObjectReference{Name: "studio"},
			Output: domain.TakeOutputSpec{
				S3: domain.S3OutputSpec{
					Bucket:    "mocap-recordings",
					ObjectKey: "takes/take-1.mp4",
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
			Take: domain.StreamTakeSpec{Protocol: "srt", Port: 10000},
		},
	}

	job := BuildTakeJob(take, stream, TakeJobOptions{
		RecorderImage: "recorder:dev",
		UploaderImage: "rclone:dev",
	})

	if job.Name != "take-take-1" {
		t.Fatalf("job name = %q, want take-take-1", job.Name)
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
		"output=\"/take/data/take.mp4\"",
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
				Format:         "mpegts",
				Room:           "room-a",
				TokenSecretRef: &domain.SecretKeyReference{Name: "livekit-token", Key: "token"},
			},
			Take: domain.StreamTakeSpec{
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
		"take_fanout &",
		"[f=mpegts:onfail=ignore]${TAKE_FANOUT_URI}",
	} {
		if !containsSubstring(container.Args[0], want) {
			t.Fatalf("relay script = %q, want substring %q", container.Args[0], want)
		}
	}
	for _, want := range []string{
		"INPUT_URI",
		"LIVEKIT_OUTPUT_URI",
		"LIVEKIT_OUTPUT_OPTIONS",
		"RELAY_CODEC_ARGS",
		"TAKE_FANOUT_URI",
		"TAKE_OUTPUT_URI",
		"LIVEKIT_TOKEN",
	} {
		if !hasEnv(container.Env, want) {
			t.Fatalf("relay env = %#v, want %q", container.Env, want)
		}
	}
	if service.Spec.Ports[0].Protocol != corev1.ProtocolUDP {
		t.Fatalf("take service protocol = %q, want UDP", service.Spec.Ports[0].Protocol)
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
	if value := envValue(container.Env, "RELAY_CODEC_ARGS"); value != "-c copy" {
		t.Fatalf("relay codec args = %q, want -c copy", value)
	}
	if value := envValue(container.Env, "TAKE_FANOUT_URI"); value != "udp://127.0.0.1:23000?pkt_size=1316" {
		t.Fatalf("take fanout uri = %q, want local UDP fanout", value)
	}
	if value := envValue(container.Env, "TAKE_OUTPUT_URI"); value != "srt://:10000?mode=listener" {
		t.Fatalf("take output uri = %q, want take listener output", value)
	}
}

func TestBuildStreamWorkloadWithRTMPLiveKit(t *testing.T) {
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
				URL:    "rtmp://livekit-ingress.recording-system.svc.cluster.local:1935/x/key",
				Format: "flv",
				Room:   "room-a",
			},
		},
	}

	deployment := BuildStreamDeployment(stream, StreamWorkloadOptions{RelayImage: "relay:dev"})
	container := deployment.Spec.Template.Spec.Containers[0]

	if hasEnv(container.Env, "LIVEKIT_TOKEN") {
		t.Fatalf("relay env = %#v, want no LIVEKIT_TOKEN when no token secret is set", container.Env)
	}
	if value := envValue(container.Env, "LIVEKIT_OUTPUT_URI"); value != stream.Spec.LiveKit.URL {
		t.Fatalf("livekit output uri = %q, want %q", value, stream.Spec.LiveKit.URL)
	}
	if value := envValue(container.Env, "LIVEKIT_OUTPUT_OPTIONS"); value != "[f=flv]" {
		t.Fatalf("livekit output options = %q, want [f=flv]", value)
	}
}

func TestBuildStreamWorkloadWithManagedLiveKitIngress(t *testing.T) {
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
				Room: "room-a",
			},
		},
		Status: domain.StreamStatus{
			LiveKitIngressID:  "IN_test",
			LiveKitSecretName: "stream-studio-livekit",
		},
	}

	deployment := BuildStreamDeployment(stream, StreamWorkloadOptions{RelayImage: "relay:dev"})
	container := deployment.Spec.Template.Spec.Containers[0]
	env := envByName(container.Env, "LIVEKIT_OUTPUT_URI")

	if deployment.Spec.Template.Annotations["recording.kinugasa.dev/livekit-ingress-id"] != "IN_test" {
		t.Fatalf("livekit ingress annotation = %q, want IN_test", deployment.Spec.Template.Annotations["recording.kinugasa.dev/livekit-ingress-id"])
	}
	if env.Value != "" {
		t.Fatalf("livekit output uri value = %q, want empty direct value", env.Value)
	}
	if env.ValueFrom == nil || env.ValueFrom.SecretKeyRef == nil {
		t.Fatalf("livekit output uri env = %#v, want secret key ref", env)
	}
	if env.ValueFrom.SecretKeyRef.Name != "stream-studio-livekit" {
		t.Fatalf("livekit secret name = %q, want stream-studio-livekit", env.ValueFrom.SecretKeyRef.Name)
	}
	if env.ValueFrom.SecretKeyRef.Key != liveKitIngressURLKey {
		t.Fatalf("livekit secret key = %q, want %q", env.ValueFrom.SecretKeyRef.Key, liveKitIngressURLKey)
	}
	if value := envValue(container.Env, "LIVEKIT_OUTPUT_OPTIONS"); value != "[f=whip]" {
		t.Fatalf("livekit output options = %q, want [f=whip]", value)
	}
	if value := envValue(container.Env, "RELAY_CODEC_ARGS"); value != "-c:v copy -c:a libopus -ac 2 -b:a 128k" {
		t.Fatalf("relay codec args = %q, want WHIP-compatible args", value)
	}

	secret := buildLiveKitSecret(stream, &LiveKitIngressInfo{
		ID:        "IN_test",
		URL:       "http://livekit-ingress.recording-system.svc.cluster.local:8080/whip/key",
		StreamKey: "key",
	})
	if string(secret.Data[liveKitIngressURLKey]) != "http://livekit-ingress.recording-system.svc.cluster.local:8080/whip/key" {
		t.Fatalf("secret url = %q, want livekit whip url", string(secret.Data[liveKitIngressURLKey]))
	}
	if string(secret.Data[liveKitStreamKeyKey]) != "key" {
		t.Fatalf("secret stream key = %q, want key", string(secret.Data[liveKitStreamKeyKey]))
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

func envByName(items []corev1.EnvVar, name string) corev1.EnvVar {
	for _, item := range items {
		if item.Name == name {
			return item
		}
	}
	return corev1.EnvVar{}
}
