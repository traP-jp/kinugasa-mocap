package k8s

import (
	"strings"

	"github.com/comavius/kinugasa-mocap/recording/server/domain"
	batchv1 "k8s.io/api/batch/v1"
	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

const stopRequestedAtAnnotation = "recording.kinugasa.dev/stop-requested-at"

type TakeJobOptions struct {
	RecorderImage string
	UploaderImage string
}

func BuildTakeJob(take *domain.Take, stream *domain.Stream, options TakeJobOptions) *batchv1.Job {
	backoffLimit := int32(0)
	labels := takeLabels(take)
	return &batchv1.Job{
		ObjectMeta: metav1.ObjectMeta{
			Name:      takeJobName(take),
			Namespace: take.Namespace,
			Labels:    labels,
		},
		Spec: batchv1.JobSpec{
			BackoffLimit: &backoffLimit,
			Template: corev1.PodTemplateSpec{
				ObjectMeta: metav1.ObjectMeta{
					Labels:      labels,
					Annotations: takePodAnnotations(take),
				},
				Spec: corev1.PodSpec{
					RestartPolicy:         corev1.RestartPolicyNever,
					ShareProcessNamespace: boolPtr(true),
					Containers: []corev1.Container{
						recorderContainer(take, stream, options),
						uploaderContainer(take, options),
					},
					Volumes: []corev1.Volume{
						takeControlVolume(),
						takeDataVolume(),
					},
				},
			},
		},
	}
}

func recorderContainer(take *domain.Take, stream *domain.Stream, options TakeJobOptions) corev1.Container {
	return corev1.Container{
		Name:            "recorder",
		Image:           options.RecorderImage,
		ImagePullPolicy: corev1.PullIfNotPresent,
		Command:         []string{"/bin/sh", "-ec"},
		Args:            []string{takeScript()},
		Env:             takeEnv(take, stream),
		VolumeMounts: []corev1.VolumeMount{
			{
				Name:      "take-control",
				MountPath: "/var/run/kinugasa/take-control",
				ReadOnly:  true,
			},
			{
				Name:      "take-data",
				MountPath: "/take/data",
			},
		},
	}
}

func takeScript() string {
	return strings.TrimSpace(`
output="/take/data/take.mp4"
failed="/take/data/take-failed"
complete="/take/data/take-complete"
pid_file="/take/data/ffmpeg.pid"
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
  if [ -s /var/run/kinugasa/take-control/stop-requested-at ]; then
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

func takeEnv(take *domain.Take, stream *domain.Stream) []corev1.EnvVar {
	return []corev1.EnvVar{
		{Name: "STREAM_URL", Value: streamTakeEndpoint(stream)},
	}
}

func uploaderContainer(take *domain.Take, options TakeJobOptions) corev1.Container {
	return corev1.Container{
		Name:            "uploader",
		Image:           options.UploaderImage,
		ImagePullPolicy: corev1.PullIfNotPresent,
		Command:         []string{"/bin/sh", "-ec"},
		Args:            []string{uploadScript()},
		Env:             s3Env(take),
		VolumeMounts: []corev1.VolumeMount{
			{
				Name:      "take-data",
				MountPath: "/take/data",
				ReadOnly:  true,
			},
		},
	}
}

func uploadScript() string {
	return strings.TrimSpace(`
complete="/take/data/take-complete"
failed="/take/data/take-failed"
input="/take/data/take.mp4"
pid_file="/take/data/ffmpeg.pid"
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

func takePodAnnotations(take *domain.Take) map[string]string {
	annotations := map[string]string{
		stopRequestedAtAnnotation: "",
	}
	if take.Spec.StopRequestedAt != nil {
		annotations[stopRequestedAtAnnotation] = take.Spec.StopRequestedAt.Format("2006-01-02T15:04:05Z07:00")
	}
	return annotations
}

func takeControlVolume() corev1.Volume {
	return corev1.Volume{
		Name: "take-control",
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

func takeDataVolume() corev1.Volume {
	return corev1.Volume{
		Name: "take-data",
		VolumeSource: corev1.VolumeSource{
			EmptyDir: &corev1.EmptyDirVolumeSource{},
		},
	}
}

func boolPtr(value bool) *bool {
	return &value
}

func s3Env(take *domain.Take) []corev1.EnvVar {
	env := []corev1.EnvVar{
		{Name: "S3_BUCKET", Value: take.Spec.Output.S3.Bucket},
		{Name: "S3_OBJECT_KEY", Value: take.Spec.Output.S3.ObjectKey},
	}
	if take.Spec.Output.S3.Endpoint != "" {
		env = append(env, corev1.EnvVar{Name: "S3_ENDPOINT", Value: take.Spec.Output.S3.Endpoint})
	}
	if take.Spec.Output.S3.Region != "" {
		env = append(env, corev1.EnvVar{Name: "AWS_REGION", Value: take.Spec.Output.S3.Region})
		env = append(env, corev1.EnvVar{Name: "AWS_DEFAULT_REGION", Value: take.Spec.Output.S3.Region})
	}

	secretRef := take.Spec.Output.S3.SecretRef
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

func takeJobName(take *domain.Take) string {
	return resourceName("take", take.Name)
}

func takeLabels(take *domain.Take) map[string]string {
	return map[string]string{
		"app.kubernetes.io/name":       "kinugasa-recorder",
		"app.kubernetes.io/managed-by": "recording-server",
		"recording.kinugasa.dev/take":  string(take.UID),
	}
}

func streamTakeEndpoint(stream *domain.Stream) string {
	if stream.Status.TakeEndpoint != "" {
		return stream.Status.TakeEndpoint
	}
	if stream.Spec.Take.Endpoint != "" {
		return stream.Spec.Take.Endpoint
	}
	return serviceDNS(streamWorkloadName(stream), stream.Namespace, takePort(stream), takeProtocol(stream))
}
