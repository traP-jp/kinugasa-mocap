package k8s

import (
	"github.com/comavius/kinugasa-mocap/recording/server/domain"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
)

func AddToScheme(scheme *runtime.Scheme) error {
	scheme.AddKnownTypes(
		domain.GroupVersion,
		&domain.Stream{},
		&domain.StreamList{},
		&domain.Take{},
		&domain.TakeList{},
	)
	metav1.AddToGroupVersion(scheme, domain.GroupVersion)
	return nil
}
