package domain

import (
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/runtime/schema"
)

var GroupVersion = schema.GroupVersion{
	Group:   APIGroup,
	Version: APIVersion,
}

type Stream struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`

	Spec   StreamSpec   `json:"spec,omitempty"`
	Status StreamStatus `json:"status,omitempty"`
}

type StreamSpec struct {
	DisplayName string          `json:"displayName,omitempty"`
	Input       StreamInputSpec `json:"input"`
	LiveKit     LiveKitSpec     `json:"livekit"`
	Take        StreamTakeSpec  `json:"take,omitempty"`
}

type StreamInputSpec struct {
	Protocol string `json:"protocol"`
	URI      string `json:"uri"`
	Port     int32  `json:"port,omitempty"`
	NodePort int32  `json:"nodePort,omitempty"`
}

type LiveKitSpec struct {
	URL                 string              `json:"url,omitempty"`
	Format              string              `json:"format,omitempty"`
	Room                string              `json:"room"`
	TokenSecretRef      *SecretKeyReference `json:"tokenSecretRef,omitempty"`
	ParticipantIdentity string              `json:"participantIdentity,omitempty"`
	ParticipantName     string              `json:"participantName,omitempty"`
}

type SecretKeyReference struct {
	Name string `json:"name"`
	Key  string `json:"key,omitempty"`
}

type StreamTakeSpec struct {
	Protocol string `json:"protocol,omitempty"`
	Endpoint string `json:"endpoint,omitempty"`
	Port     int32  `json:"port,omitempty"`
}

type StreamStatus struct {
	ObservedGeneration int64       `json:"observedGeneration,omitempty"`
	Phase              string      `json:"phase,omitempty"`
	PodName            string      `json:"podName,omitempty"`
	ServiceName        string      `json:"serviceName,omitempty"`
	TakeEndpoint       string      `json:"takeEndpoint,omitempty"`
	LiveKitIngressID   string      `json:"livekitIngressID,omitempty"`
	LiveKitSecretName  string      `json:"livekitSecretName,omitempty"`
	LiveKitConnected   bool        `json:"livekitConnected,omitempty"`
	Error              string      `json:"error,omitempty"`
	Conditions         []Condition `json:"conditions,omitempty"`
}

type StreamList struct {
	metav1.TypeMeta `json:",inline"`
	metav1.ListMeta `json:"metadata,omitempty"`

	Items []Stream `json:"items"`
}

type Take struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`

	Spec   TakeSpec   `json:"spec,omitempty"`
	Status TakeStatus `json:"status,omitempty"`
}

type TakeSpec struct {
	StreamRef       LocalObjectReference `json:"streamRef"`
	Output          TakeOutputSpec       `json:"output"`
	StopRequestedAt *metav1.Time         `json:"stopRequestedAt,omitempty"`
}

type LocalObjectReference struct {
	Name      string `json:"name"`
	Namespace string `json:"namespace,omitempty"`
}

type TakeOutputSpec struct {
	S3 S3OutputSpec `json:"s3"`
}

type S3OutputSpec struct {
	Bucket    string             `json:"bucket"`
	ObjectKey string             `json:"objectKey"`
	Endpoint  string             `json:"endpoint,omitempty"`
	Region    string             `json:"region,omitempty"`
	SecretRef *S3SecretReference `json:"secretRef,omitempty"`
}

type S3SecretReference struct {
	Name               string `json:"name"`
	AccessKeyIDKey     string `json:"accessKeyIDKey,omitempty"`
	SecretAccessKeyKey string `json:"secretAccessKeyKey,omitempty"`
	SessionTokenKey    string `json:"sessionTokenKey,omitempty"`
}

type TakeStatus struct {
	ObservedGeneration int64        `json:"observedGeneration,omitempty"`
	Phase              string       `json:"phase,omitempty"`
	JobName            string       `json:"jobName,omitempty"`
	StartedAt          *metav1.Time `json:"startedAt,omitempty"`
	StoppedAt          *metav1.Time `json:"stoppedAt,omitempty"`
	CompletedAt        *metav1.Time `json:"completedAt,omitempty"`
	S3                 TakeS3Status `json:"s3,omitempty"`
	Error              string       `json:"error,omitempty"`
	Conditions         []Condition  `json:"conditions,omitempty"`
}

type TakeS3Status struct {
	Bucket    string `json:"bucket,omitempty"`
	ObjectKey string `json:"objectKey,omitempty"`
	URI       string `json:"uri,omitempty"`
}

type TakeList struct {
	metav1.TypeMeta `json:",inline"`
	metav1.ListMeta `json:"metadata,omitempty"`

	Items []Take `json:"items"`
}

type Condition struct {
	Type               string      `json:"type"`
	Status             string      `json:"status"`
	Reason             string      `json:"reason,omitempty"`
	Message            string      `json:"message,omitempty"`
	LastTransitionTime metav1.Time `json:"lastTransitionTime"`
}

func (in *Stream) DeepCopyObject() runtime.Object {
	out := new(Stream)
	*out = *in
	in.ObjectMeta.DeepCopyInto(&out.ObjectMeta)
	if in.Spec.LiveKit.TokenSecretRef != nil {
		tokenSecretRef := *in.Spec.LiveKit.TokenSecretRef
		out.Spec.LiveKit.TokenSecretRef = &tokenSecretRef
	}
	out.Status.Conditions = append([]Condition(nil), in.Status.Conditions...)
	return out
}

func (in *StreamList) DeepCopyObject() runtime.Object {
	out := new(StreamList)
	*out = *in
	in.ListMeta.DeepCopyInto(&out.ListMeta)
	out.Items = make([]Stream, len(in.Items))
	for i := range in.Items {
		out.Items[i] = *in.Items[i].DeepCopyObject().(*Stream)
	}
	return out
}

func (in *Take) DeepCopyObject() runtime.Object {
	out := new(Take)
	*out = *in
	in.ObjectMeta.DeepCopyInto(&out.ObjectMeta)
	if in.Spec.StopRequestedAt != nil {
		out.Spec.StopRequestedAt = in.Spec.StopRequestedAt.DeepCopy()
	}
	if in.Spec.Output.S3.SecretRef != nil {
		secretRef := *in.Spec.Output.S3.SecretRef
		out.Spec.Output.S3.SecretRef = &secretRef
	}
	if in.Status.StartedAt != nil {
		out.Status.StartedAt = in.Status.StartedAt.DeepCopy()
	}
	if in.Status.StoppedAt != nil {
		out.Status.StoppedAt = in.Status.StoppedAt.DeepCopy()
	}
	if in.Status.CompletedAt != nil {
		out.Status.CompletedAt = in.Status.CompletedAt.DeepCopy()
	}
	out.Status.Conditions = append([]Condition(nil), in.Status.Conditions...)
	return out
}

func (in *TakeList) DeepCopyObject() runtime.Object {
	out := new(TakeList)
	*out = *in
	in.ListMeta.DeepCopyInto(&out.ListMeta)
	out.Items = make([]Take, len(in.Items))
	for i := range in.Items {
		out.Items[i] = *in.Items[i].DeepCopyObject().(*Take)
	}
	return out
}
