package k8s

import (
	"strings"

	"github.com/comavius/kinugasa-mocap/recording/server/domain"
	batchv1 "k8s.io/api/batch/v1"
	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

const stopRequestedAtAnnotation = "recording.kinugasa.dev/stop-requested-at"

type RecordingJobOptions struct {
	RecorderImage string
	UploaderImage string
}

func BuildRecordingJob(recording *domain.Recording, stream *domain.Stream, options RecordingJobOptions) *batchv1.Job {
	backoffLimit := int32(0)
	labels := recordingLabels(recording)
	return &batchv1.Job{
		ObjectMeta: metav1.ObjectMeta{
			Name:      recordingJobName(recording),
			Namespace: recording.Namespace,
			Labels:    labels,
		},
		Spec: batchv1.JobSpec{
			BackoffLimit: &backoffLimit,
			Template: corev1.PodTemplateSpec{
				ObjectMeta: metav1.ObjectMeta{
					Labels:      labels,
					Annotations: recordingPodAnnotations(recording),
				},
				Spec: corev1.PodSpec{
					RestartPolicy:         corev1.RestartPolicyNever,
					ShareProcessNamespace: boolPtr(true),
					Containers: []corev1.Container{
						recordingContainer(recording, stream, options),
						uploaderContainer(recording, options),
					},
					Volumes: []corev1.Volume{
						recordingControlVolume(),
						recordingDataVolume(),
					},
				},
			},
		},
	}
}

func recordingContainer(recording *domain.Recording, stream *domain.Stream, options RecordingJobOptions) corev1.Container {
	return corev1.Container{
		Name:            "recorder",
		Image:           options.RecorderImage,
		ImagePullPolicy: corev1.PullIfNotPresent,
		Command:         []string{"/bin/sh", "-ec"},
		Args:            []string{recordingScript()},
		Env:             recordingEnv(recording, stream),
		VolumeMounts: []corev1.VolumeMount{
			{
				Name:      "recording-control",
				MountPath: "/var/run/kinugasa/recording-control",
				ReadOnly:  true,
			},
			{
				Name:      "recording-data",
				MountPath: "/recording/data",
			},
		},
	}
}

func recordingScript() string {
	return strings.TrimSpace(`
output="/recording/data/recording.mp4"
failed="/recording/data/recording-failed"
complete="/recording/data/recording-complete"
pid_file="/recording/data/ffmpeg.pid"
rm -f "${failed}" "${complete}" "${pid_file}"

ffmpeg -hide_banner -loglevel info \
  -i "${STREAM_URL}" \
  -map 0 \
  -c copy \
  -f mp4 \
  "${output}" &

ffmpeg_pid="$!"
printf '%s\n' "${ffmpeg_pid}" > "${pid_file}"
stop_requested=false
while kill -0 "${ffmpeg_pid}" 2>/dev/null; do
  if [ -s /var/run/kinugasa/recording-control/stop-requested-at ]; then
    stop_requested=true
    kill -TERM "${ffmpeg_pid}"
    break
  fi
  sleep 1
done

status=0
wait "${ffmpeg_pid}" || status="$?"
if [ "${stop_requested}" = "true" ] && { [ "${status}" = "0" ] || [ "${status}" = "143" ] || [ "${status}" = "255" ]; }; then
  touch "${complete}"
  exit 0
fi
if [ "${status}" = "0" ]; then
  touch "${complete}"
else
  printf '%s\n' "${status}" > "${failed}"
fi
exit "${status}"
`)
}

func recordingEnv(recording *domain.Recording, stream *domain.Stream) []corev1.EnvVar {
	return []corev1.EnvVar{
		{Name: "STREAM_URL", Value: streamRecordingEndpoint(stream)},
	}
}

func uploaderContainer(recording *domain.Recording, options RecordingJobOptions) corev1.Container {
	return corev1.Container{
		Name:            "uploader",
		Image:           options.UploaderImage,
		ImagePullPolicy: corev1.PullIfNotPresent,
		Command:         []string{"/bin/sh", "-ec"},
		Args:            []string{uploadScript()},
		Env:             s3Env(recording),
		VolumeMounts: []corev1.VolumeMount{
			{
				Name:      "recording-data",
				MountPath: "/recording/data",
				ReadOnly:  true,
			},
		},
	}
}

func uploadScript() string {
	return strings.TrimSpace(`
complete="/recording/data/recording-complete"
failed="/recording/data/recording-failed"
input="/recording/data/recording.mp4"
pid_file="/recording/data/ffmpeg.pid"
start_timeout=60
elapsed=0

while [ ! -e "${complete}" ] && [ ! -e "${failed}" ]; do
  if [ -s "${pid_file}" ]; then
    ffmpeg_pid="$(cat "${pid_file}")"
    if ! kill -0 "${ffmpeg_pid}" 2>/dev/null; then
      sleep 1
      if [ ! -e "${complete}" ] && [ ! -e "${failed}" ]; then
        echo "recorder exited without completion marker" >&2
        exit 1
      fi
    fi
  else
    elapsed="$((elapsed + 1))"
    if [ "${elapsed}" -ge "${start_timeout}" ]; then
      echo "recorder did not start within ${start_timeout}s" >&2
      exit 1
    fi
  fi
  sleep 1
done

if [ -e "${failed}" ]; then
  echo "recorder failed before upload" >&2
  exit 1
fi

remote=":s3,provider=AWS,env_auth=true"
if [ -n "${AWS_REGION:-}" ]; then
  remote="${remote},region=${AWS_REGION}"
fi
if [ -n "${S3_ENDPOINT:-}" ]; then
  remote="${remote},endpoint=${S3_ENDPOINT}"
fi
remote="${remote}:${S3_BUCKET}/${S3_OBJECT_KEY}"

rclone copyto "${input}" "${remote}"
`)
}

func recordingPodAnnotations(recording *domain.Recording) map[string]string {
	annotations := map[string]string{
		stopRequestedAtAnnotation: "",
	}
	if recording.Spec.StopRequestedAt != nil {
		annotations[stopRequestedAtAnnotation] = recording.Spec.StopRequestedAt.Format("2006-01-02T15:04:05Z07:00")
	}
	return annotations
}

func recordingControlVolume() corev1.Volume {
	return corev1.Volume{
		Name: "recording-control",
		VolumeSource: corev1.VolumeSource{
			DownwardAPI: &corev1.DownwardAPIVolumeSource{
				Items: []corev1.DownwardAPIVolumeFile{
					{
						Path: "stop-requested-at",
						FieldRef: &corev1.ObjectFieldSelector{
							FieldPath: "metadata.annotations['" + stopRequestedAtAnnotation + "']",
						},
					},
				},
			},
		},
	}
}

func recordingDataVolume() corev1.Volume {
	return corev1.Volume{
		Name: "recording-data",
		VolumeSource: corev1.VolumeSource{
			EmptyDir: &corev1.EmptyDirVolumeSource{},
		},
	}
}

func boolPtr(value bool) *bool {
	return &value
}

func s3Env(recording *domain.Recording) []corev1.EnvVar {
	env := []corev1.EnvVar{
		{Name: "S3_BUCKET", Value: recording.Spec.Output.S3.Bucket},
		{Name: "S3_OBJECT_KEY", Value: recording.Spec.Output.S3.ObjectKey},
	}
	if recording.Spec.Output.S3.Endpoint != "" {
		env = append(env, corev1.EnvVar{Name: "S3_ENDPOINT", Value: recording.Spec.Output.S3.Endpoint})
	}
	if recording.Spec.Output.S3.Region != "" {
		env = append(env, corev1.EnvVar{Name: "AWS_REGION", Value: recording.Spec.Output.S3.Region})
		env = append(env, corev1.EnvVar{Name: "AWS_DEFAULT_REGION", Value: recording.Spec.Output.S3.Region})
	}

	secretRef := recording.Spec.Output.S3.SecretRef
	if secretRef == nil {
		return env
	}

	accessKeyIDKey := secretRef.AccessKeyIDKey
	if accessKeyIDKey == "" {
		accessKeyIDKey = "AWS_ACCESS_KEY_ID"
	}
	secretAccessKeyKey := secretRef.SecretAccessKeyKey
	if secretAccessKeyKey == "" {
		secretAccessKeyKey = "AWS_SECRET_ACCESS_KEY"
	}

	env = append(env,
		secretEnv("AWS_ACCESS_KEY_ID", secretRef.Name, accessKeyIDKey),
		secretEnv("AWS_SECRET_ACCESS_KEY", secretRef.Name, secretAccessKeyKey),
	)
	if secretRef.SessionTokenKey != "" {
		env = append(env, secretEnv("AWS_SESSION_TOKEN", secretRef.Name, secretRef.SessionTokenKey))
	}
	return env
}

func secretEnv(envName, secretName, key string) corev1.EnvVar {
	return corev1.EnvVar{
		Name: envName,
		ValueFrom: &corev1.EnvVarSource{
			SecretKeyRef: &corev1.SecretKeySelector{
				LocalObjectReference: corev1.LocalObjectReference{Name: secretName},
				Key:                  key,
			},
		},
	}
}

func recordingJobName(recording *domain.Recording) string {
	return resourceName("recording", recording.Name)
}

func recordingLabels(recording *domain.Recording) map[string]string {
	return map[string]string{
		"app.kubernetes.io/name":           "kinugasa-recorder",
		"app.kubernetes.io/managed-by":     "recording-server",
		"recording.kinugasa.dev/recording": string(recording.UID),
	}
}

func streamRecordingEndpoint(stream *domain.Stream) string {
	if stream.Status.RecordingEndpoint != "" {
		return stream.Status.RecordingEndpoint
	}
	if stream.Spec.Recording.Endpoint != "" {
		return stream.Spec.Recording.Endpoint
	}
	return serviceDNS(streamWorkloadName(stream), stream.Namespace, recordingPort(stream), recordingProtocol(stream))
}
