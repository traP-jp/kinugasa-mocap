package k8s

import (
	"github.com/comavius/kinugasa-mocap/recording/server/domain"
	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

func buildLiveKitSecret(stream *domain.Stream, ingress *LiveKitIngressInfo) *corev1.Secret {
	return &corev1.Secret{
		ObjectMeta: metav1.ObjectMeta{
			Name:      liveKitSecretName(stream),
			Namespace: stream.Namespace,
			Labels:    streamLabels(stream),
		},
		Type: corev1.SecretTypeOpaque,
		Data: map[string][]byte{
			liveKitIngressURLKey: []byte(ingress.URL),
			liveKitStreamKeyKey:  []byte(ingress.StreamKey),
		},
	}
}
