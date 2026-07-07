package k8s

import (
	"strings"

	"github.com/comavius/kinugasa-mocap/recording/server/domain"
	appsv1 "k8s.io/api/apps/v1"
	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/equality"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

const (
	defaultRecordingProtocol = "srt"
	defaultRecordingPort     = int32(10000)
)

type StreamWorkloadOptions struct {
	RelayImage string
}

func BuildStreamDeployment(stream *domain.Stream, options StreamWorkloadOptions) *appsv1.Deployment {
	replicas := int32(1)
	labels := streamLabels(stream)

	return &appsv1.Deployment{
		ObjectMeta: metav1.ObjectMeta{
			Name:      streamWorkloadName(stream),
			Namespace: stream.Namespace,
			Labels:    labels,
		},
		Spec: appsv1.DeploymentSpec{
			Replicas: &replicas,
			Selector: &metav1.LabelSelector{MatchLabels: labels},
			Template: corev1.PodTemplateSpec{
				ObjectMeta: metav1.ObjectMeta{Labels: labels},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{streamRelayContainer(stream, options)},
				},
			},
		},
	}
}

func UpdateStreamDeployment(deployment *appsv1.Deployment, stream *domain.Stream, options StreamWorkloadOptions) bool {
	replicas := int32(1)
	labels := streamLabels(stream)
	containers := []corev1.Container{streamRelayContainer(stream, options)}
	changed := false

	if !equality.Semantic.DeepEqual(deployment.Labels, labels) {
		deployment.Labels = labels
		changed = true
	}
	if deployment.Spec.Replicas == nil || *deployment.Spec.Replicas != replicas {
		deployment.Spec.Replicas = &replicas
		changed = true
	}
	if !equality.Semantic.DeepEqual(deployment.Spec.Selector, &metav1.LabelSelector{MatchLabels: labels}) {
		deployment.Spec.Selector = &metav1.LabelSelector{MatchLabels: labels}
		changed = true
	}
	if !equality.Semantic.DeepEqual(deployment.Spec.Template.Labels, labels) {
		deployment.Spec.Template.Labels = labels
		changed = true
	}
	if !equality.Semantic.DeepEqual(deployment.Spec.Template.Spec.Containers, containers) {
		deployment.Spec.Template.Spec.Containers = containers
		changed = true
	}

	return changed
}

func BuildStreamService(stream *domain.Stream) *corev1.Service {
	labels := streamLabels(stream)
	return &corev1.Service{
		ObjectMeta: metav1.ObjectMeta{
			Name:      streamWorkloadName(stream),
			Namespace: stream.Namespace,
			Labels:    labels,
		},
		Spec: corev1.ServiceSpec{
			Selector: labels,
			Type:     streamServiceType(stream),
			Ports:    streamServicePorts(stream),
		},
	}
}

func UpdateStreamService(service *corev1.Service, stream *domain.Stream) bool {
	labels := streamLabels(stream)
	serviceType := streamServiceType(stream)
	ports := streamServicePorts(stream)
	changed := false

	if !equality.Semantic.DeepEqual(service.Labels, labels) {
		service.Labels = labels
		changed = true
	}
	if !equality.Semantic.DeepEqual(service.Spec.Selector, labels) {
		service.Spec.Selector = labels
		changed = true
	}
	if service.Spec.Type != serviceType {
		service.Spec.Type = serviceType
		changed = true
	}
	if !equality.Semantic.DeepEqual(service.Spec.Ports, ports) {
		service.Spec.Ports = ports
		changed = true
	}
	return changed
}

func streamRelayContainer(stream *domain.Stream, options StreamWorkloadOptions) corev1.Container {
	port := recordingPort(stream)
	return corev1.Container{
		Name:            "relay",
		Image:           options.RelayImage,
		ImagePullPolicy: corev1.PullIfNotPresent,
		Command:         []string{"/bin/sh", "-ec"},
		Args:            []string{streamRelayScript()},
		Env:             streamRelayEnv(stream),
		Ports:           streamContainerPorts(stream, port),
	}
}

func streamRelayEnv(stream *domain.Stream) []corev1.EnvVar {
	env := []corev1.EnvVar{
		{Name: "INPUT_URI", Value: stream.Spec.Input.URI},
		{Name: "LIVEKIT_OUTPUT_URI", Value: liveKitOutput(stream)},
		{Name: "LIVEKIT_OUTPUT_OPTIONS", Value: liveKitOutputOptions(stream)},
		{Name: "RECORDING_OUTPUT_URI", Value: recordingOutput(stream)},
	}
	if !stream.Spec.LiveKit.Mock {
		env = append(env, corev1.EnvVar{
			Name: "LIVEKIT_TOKEN",
			ValueFrom: &corev1.EnvVarSource{
				SecretKeyRef: &corev1.SecretKeySelector{
					LocalObjectReference: corev1.LocalObjectReference{Name: stream.Spec.LiveKit.TokenSecretRef.Name},
					Key:                  stream.Spec.LiveKit.TokenSecretRef.Key,
				},
			},
		})
	}
	return env
}

func streamRelayScript() string {
	return strings.TrimSpace(`
auth_headers=""
if [ -n "${LIVEKIT_TOKEN:-}" ]; then
  auth_headers="-headers Authorization: Bearer ${LIVEKIT_TOKEN}"
fi

ffmpeg -hide_banner -loglevel info \
  -i "${INPUT_URI}" \
  -map 0 \
  -c copy \
  ${auth_headers} \
  -f tee \
  "${LIVEKIT_OUTPUT_OPTIONS}${LIVEKIT_OUTPUT_URI}|${RECORDING_OUTPUT_URI}"
`)
}

func liveKitOutput(stream *domain.Stream) string {
	if stream.Spec.LiveKit.Mock {
		return "/dev/null"
	}
	return stream.Spec.LiveKit.URL
}

func liveKitOutputOptions(stream *domain.Stream) string {
	if stream.Spec.LiveKit.Mock {
		return "[f=null]"
	}
	return "[f=mpegts]"
}

func recordingOutput(stream *domain.Stream) string {
	return "[f=mpegts]" + recordingProtocol(stream) + "://:" + int32String(recordingPort(stream)) + "?mode=listener"
}

func streamServiceType(stream *domain.Stream) corev1.ServiceType {
	if stream.Spec.Input.NodePort != 0 {
		return corev1.ServiceTypeNodePort
	}
	return corev1.ServiceTypeClusterIP
}

func streamServicePorts(stream *domain.Stream) []corev1.ServicePort {
	ports := []corev1.ServicePort{
		{
			Name:     "recording",
			Port:     recordingPort(stream),
			Protocol: corev1.ProtocolUDP,
		},
	}
	if stream.Spec.Input.Port != 0 {
		servicePort := corev1.ServicePort{
			Name:     "input",
			Port:     stream.Spec.Input.Port,
			Protocol: corev1.ProtocolUDP,
		}
		if stream.Spec.Input.NodePort != 0 {
			servicePort.NodePort = stream.Spec.Input.NodePort
		}
		ports = append(ports, servicePort)
	}
	return ports
}

func streamContainerPorts(stream *domain.Stream, recordingPort int32) []corev1.ContainerPort {
	ports := []corev1.ContainerPort{
		{
			Name:          "recording",
			ContainerPort: recordingPort,
			Protocol:      corev1.ProtocolUDP,
		},
	}
	if stream.Spec.Input.Port != 0 {
		ports = append(ports, corev1.ContainerPort{
			Name:          "input",
			ContainerPort: stream.Spec.Input.Port,
			Protocol:      corev1.ProtocolUDP,
		})
	}
	return ports
}

func streamWorkloadName(stream *domain.Stream) string {
	return resourceName("stream", stream.Name)
}

func streamLabels(stream *domain.Stream) map[string]string {
	return map[string]string{
		"app.kubernetes.io/name":        "kinugasa-stream",
		"app.kubernetes.io/managed-by":  "recording-server",
		"recording.kinugasa.dev/stream": string(stream.UID),
	}
}

func recordingProtocol(stream *domain.Stream) string {
	if stream.Spec.Recording.Protocol == "" {
		return defaultRecordingProtocol
	}
	return stream.Spec.Recording.Protocol
}

func recordingPort(stream *domain.Stream) int32 {
	if stream.Spec.Recording.Port == 0 {
		return defaultRecordingPort
	}
	return stream.Spec.Recording.Port
}
