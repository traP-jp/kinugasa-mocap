package service

import (
	"context"
	"fmt"
	"net"
	"net/url"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/comavius/kinugasa-mocap/recording/server/domain"
	apierrors "k8s.io/apimachinery/pkg/api/errors"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/types"
	"sigs.k8s.io/controller-runtime/pkg/client"
)

const (
	defaultResourceNamespace = "default"
	defaultPublicIngestHost  = "127.0.0.1"
	defaultLiveKitURL        = "ws://localhost:7880"
	srtBasePort              = int32(9000)
	ristBasePort             = int32(9001)
	srtBaseNodePort          = int32(30900)
	ristBaseNodePort         = int32(30901)
	takeInputPort            = int32(10000)
)

type KubernetesUsecase struct {
	client client.Client
	cfg    UsecaseConfig
}

func NewKubernetesUsecase(k8sClient client.Client, config UsecaseConfig) *KubernetesUsecase {
	if config.ResourceNamespace == "" {
		config.ResourceNamespace = defaultResourceNamespace
	}
	if config.PublicIngestHost == "" {
		config.PublicIngestHost = defaultPublicIngestHost
	}
	if config.LiveKitPublicURL == "" {
		config.LiveKitPublicURL = defaultLiveKitURL
	}
	if config.Now == nil {
		config.Now = time.Now
	}
	return &KubernetesUsecase{
		client: k8sClient,
		cfg:    config,
	}
}

func (u *KubernetesUsecase) CreateCamera(ctx context.Context, input CreateCameraInput) (Camera, error) {
	if err := ctx.Err(); err != nil {
		return Camera{}, err
	}
	displayName := strings.TrimSpace(input.DisplayName)
	if displayName == "" {
		return Camera{}, fmt.Errorf("%w: displayName is required", ErrInvalid)
	}
	if input.Protocol != CameraProtocolSRT && input.Protocol != CameraProtocolRIST {
		return Camera{}, fmt.Errorf("%w: unsupported camera protocol", ErrInvalid)
	}

	streams, err := u.listStreams(ctx)
	if err != nil {
		return Camera{}, err
	}
	id := allocateStreamName(slug(displayName), streams)
	port := allocateInputPort(input.Protocol, streams)
	nodePort := nodePortForInput(input.Protocol, port)
	room := strings.TrimSpace(input.LiveKitRoom)
	if room == "" {
		room = id
	}

	stream := &domain.Stream{
		TypeMeta: metav1.TypeMeta{
			APIVersion: domain.GroupVersion.String(),
			Kind:       domain.StreamKind,
		},
		ObjectMeta: metav1.ObjectMeta{
			Name:      id,
			Namespace: u.cfg.ResourceNamespace,
		},
		Spec: domain.StreamSpec{
			DisplayName: displayName,
			Input: domain.StreamInputSpec{
				Protocol: string(input.Protocol),
				URI:      listenerURI(input.Protocol, port),
				Port:     port,
				NodePort: nodePort,
			},
			LiveKit: domain.LiveKitSpec{
				Room:                room,
				ParticipantIdentity: id,
				ParticipantName:     displayName,
			},
			Take: domain.StreamTakeSpec{
				Protocol: string(CameraProtocolSRT),
				Port:     takeInputPort,
			},
		},
	}
	if err := u.client.Create(ctx, stream); err != nil {
		if apierrors.IsAlreadyExists(err) {
			return Camera{}, fmt.Errorf("%w: camera already exists", ErrConflict)
		}
		return Camera{}, err
	}
	return u.cameraFromStream(*stream), nil
}

func (u *KubernetesUsecase) DeleteCamera(ctx context.Context, id string) error {
	if err := ctx.Err(); err != nil {
		return err
	}
	stream := &domain.Stream{}
	if err := u.client.Get(ctx, u.key(id), stream); err != nil {
		return mapK8sError(err)
	}
	takes, err := u.listTakes(ctx)
	if err != nil {
		return err
	}
	for _, take := range takes {
		if take.Spec.StreamRef.Name == id && takeActive(takePhaseFromCR(take)) {
			return fmt.Errorf("%w: camera has an active take", ErrConflict)
		}
	}
	return mapK8sError(u.client.Delete(ctx, stream))
}

func (u *KubernetesUsecase) GetCamera(ctx context.Context, id string) (Camera, error) {
	if err := ctx.Err(); err != nil {
		return Camera{}, err
	}
	stream := &domain.Stream{}
	if err := u.client.Get(ctx, u.key(id), stream); err != nil {
		return Camera{}, mapK8sError(err)
	}
	return u.cameraFromStream(*stream), nil
}

func (u *KubernetesUsecase) GetCameraEndpoint(ctx context.Context, id string) (CameraEndpoint, error) {
	camera, err := u.GetCamera(ctx, id)
	if err != nil {
		return CameraEndpoint{}, err
	}
	return camera.Endpoint, nil
}

func (u *KubernetesUsecase) ListCameras(ctx context.Context, input ListCamerasInput) ([]Camera, error) {
	if err := ctx.Err(); err != nil {
		return nil, err
	}
	streams, err := u.listStreams(ctx)
	if err != nil {
		return nil, err
	}
	cameras := make([]Camera, 0, len(streams))
	for _, stream := range streams {
		if input.Protocol != nil && CameraProtocol(stream.Spec.Input.Protocol) != *input.Protocol {
			continue
		}
		cameras = append(cameras, u.cameraFromStream(stream))
	}
	sort.Slice(cameras, func(i, j int) bool {
		return cameras[i].ID < cameras[j].ID
	})
	return cameras, nil
}

func (u *KubernetesUsecase) StartTake(ctx context.Context, input StartTakeInput) (Take, error) {
	if err := ctx.Err(); err != nil {
		return Take{}, err
	}
	if strings.TrimSpace(input.CameraID) == "" {
		return Take{}, fmt.Errorf("%w: cameraId is required", ErrInvalid)
	}
	if strings.TrimSpace(input.Output.S3.Bucket) == "" {
		return Take{}, fmt.Errorf("%w: output.s3.bucket is required", ErrInvalid)
	}
	stream := &domain.Stream{}
	if err := u.client.Get(ctx, u.key(input.CameraID), stream); err != nil {
		return Take{}, mapK8sError(err)
	}
	takes, err := u.listTakes(ctx)
	if err != nil {
		return Take{}, err
	}
	for _, take := range takes {
		if take.Spec.StreamRef.Name == input.CameraID && takeActive(takePhaseFromCR(take)) {
			return Take{}, fmt.Errorf("%w: camera already has an active take", ErrConflict)
		}
	}

	id := allocateTakeName(input.CameraID, takes)
	objectKey := strings.TrimSpace(input.Output.S3.ObjectKey)
	if objectKey == "" {
		objectKey = "takes/" + id + ".mp4"
	}
	take := &domain.Take{
		TypeMeta: metav1.TypeMeta{
			APIVersion: domain.GroupVersion.String(),
			Kind:       domain.TakeKind,
		},
		ObjectMeta: metav1.ObjectMeta{
			Name:      id,
			Namespace: u.cfg.ResourceNamespace,
		},
		Spec: domain.TakeSpec{
			StreamRef: domain.LocalObjectReference{Name: input.CameraID},
			Output: domain.TakeOutputSpec{
				S3: domain.S3OutputSpec{
					Bucket:    input.Output.S3.Bucket,
					ObjectKey: objectKey,
					Endpoint:  input.Output.S3.Endpoint,
					Region:    input.Output.S3.Region,
				},
			},
		},
	}
	if err := u.client.Create(ctx, take); err != nil {
		if apierrors.IsAlreadyExists(err) {
			return Take{}, fmt.Errorf("%w: take already exists", ErrConflict)
		}
		return Take{}, err
	}
	return takeFromCR(*take), nil
}

func (u *KubernetesUsecase) StopTake(ctx context.Context, input StopTakeInput) (Take, error) {
	if err := ctx.Err(); err != nil {
		return Take{}, err
	}
	take := &domain.Take{}
	if err := u.client.Get(ctx, u.key(input.TakeID), take); err != nil {
		return Take{}, mapK8sError(err)
	}
	if !takeActive(takePhaseFromCR(*take)) {
		return Take{}, fmt.Errorf("%w: take is not active", ErrConflict)
	}
	requestedAt := input.RequestedAt
	if requestedAt.IsZero() {
		requestedAt = u.cfg.Now().UTC()
	}
	take.Spec.StopRequestedAt = &metav1.Time{Time: requestedAt}
	if err := u.client.Update(ctx, take); err != nil {
		return Take{}, mapK8sError(err)
	}
	return takeFromCR(*take), nil
}

func (u *KubernetesUsecase) GetTake(ctx context.Context, id string) (Take, error) {
	if err := ctx.Err(); err != nil {
		return Take{}, err
	}
	take := &domain.Take{}
	if err := u.client.Get(ctx, u.key(id), take); err != nil {
		return Take{}, mapK8sError(err)
	}
	return takeFromCR(*take), nil
}

func (u *KubernetesUsecase) ListTakes(ctx context.Context, input ListTakesInput) ([]Take, error) {
	if err := ctx.Err(); err != nil {
		return nil, err
	}
	crs, err := u.listTakes(ctx)
	if err != nil {
		return nil, err
	}
	takes := make([]Take, 0, len(crs))
	for _, cr := range crs {
		take := takeFromCR(cr)
		if input.CameraID != "" && take.CameraID != input.CameraID {
			continue
		}
		if input.Phase != nil && take.Phase != *input.Phase {
			continue
		}
		takes = append(takes, take)
	}
	sort.Slice(takes, func(i, j int) bool {
		return takes[i].ID < takes[j].ID
	})
	return takes, nil
}

func (u *KubernetesUsecase) GetLiveKitInfo(ctx context.Context) (LiveKitInfo, error) {
	if err := ctx.Err(); err != nil {
		return LiveKitInfo{}, err
	}
	return LiveKitInfo{URL: u.cfg.LiveKitPublicURL}, nil
}

func (u *KubernetesUsecase) ListLiveKitRooms(ctx context.Context) ([]LiveKitRoom, error) {
	if err := ctx.Err(); err != nil {
		return nil, err
	}
	streams, err := u.listStreams(ctx)
	if err != nil {
		return nil, err
	}
	rooms := make([]LiveKitRoom, 0, len(streams))
	for _, stream := range streams {
		rooms = append(rooms, LiveKitRoom{
			Name:      stream.Spec.LiveKit.Room,
			CameraID:  stream.Name,
			Connected: stream.Status.LiveKitConnected,
		})
	}
	sort.Slice(rooms, func(i, j int) bool {
		return rooms[i].Name < rooms[j].Name
	})
	return rooms, nil
}

func (u *KubernetesUsecase) GetLiveKitConnection(ctx context.Context, input LiveKitConnectionInput) (LiveKitConnection, error) {
	if err := ctx.Err(); err != nil {
		return LiveKitConnection{}, err
	}
	streams, err := u.listStreams(ctx)
	if err != nil {
		return LiveKitConnection{}, err
	}
	for _, stream := range streams {
		if stream.Spec.LiveKit.Room != input.RoomName {
			continue
		}
		identity := strings.TrimSpace(input.ParticipantIdentity)
		if identity == "" {
			identity = stream.Name + "-viewer"
		}
		name := strings.TrimSpace(input.ParticipantName)
		if name == "" {
			name = identity
		}
		token, err := u.issueLiveKitToken(ctx, LiveKitTokenRequest{
			RoomName:            input.RoomName,
			ParticipantIdentity: identity,
			ParticipantName:     name,
		})
		if err != nil {
			return LiveKitConnection{}, err
		}
		return LiveKitConnection{
			URL:                 u.cfg.LiveKitPublicURL,
			Room:                input.RoomName,
			Token:               token,
			ParticipantIdentity: identity,
			ParticipantName:     name,
		}, nil
	}
	return LiveKitConnection{}, ErrNotFound
}

func (u *KubernetesUsecase) issueLiveKitToken(ctx context.Context, request LiveKitTokenRequest) (string, error) {
	if u.cfg.LiveKitTokenIssuer == nil {
		return "", fmt.Errorf("%w: livekit token issuer is required", ErrInvalid)
	}
	return u.cfg.LiveKitTokenIssuer.IssueLiveKitToken(ctx, request)
}

func (u *KubernetesUsecase) listStreams(ctx context.Context) ([]domain.Stream, error) {
	var list domain.StreamList
	if err := u.client.List(ctx, &list, client.InNamespace(u.cfg.ResourceNamespace)); err != nil {
		return nil, err
	}
	return list.Items, nil
}

func (u *KubernetesUsecase) listTakes(ctx context.Context) ([]domain.Take, error) {
	var list domain.TakeList
	if err := u.client.List(ctx, &list, client.InNamespace(u.cfg.ResourceNamespace)); err != nil {
		return nil, err
	}
	return list.Items, nil
}

func (u *KubernetesUsecase) key(name string) types.NamespacedName {
	return types.NamespacedName{Name: name, Namespace: u.cfg.ResourceNamespace}
}

func (u *KubernetesUsecase) cameraFromStream(stream domain.Stream) Camera {
	displayName := stream.Spec.DisplayName
	if displayName == "" {
		displayName = stream.Name
	}
	return Camera{
		ID:          stream.Name,
		DisplayName: displayName,
		Protocol:    CameraProtocol(stream.Spec.Input.Protocol),
		Phase:       cameraPhaseFromStream(stream),
		Endpoint:    u.endpointFromStream(stream),
		LiveKit: CameraLiveKit{
			Room:                stream.Spec.LiveKit.Room,
			Connected:           stream.Status.LiveKitConnected,
			ParticipantIdentity: stream.Spec.LiveKit.ParticipantIdentity,
			ParticipantName:     stream.Spec.LiveKit.ParticipantName,
		},
		Error:     stream.Status.Error,
		CreatedAt: stream.CreationTimestamp.Time,
	}
}

func (u *KubernetesUsecase) endpointFromStream(stream domain.Stream) CameraEndpoint {
	endpoint := cameraEndpointURL(u.cfg.PublicIngestHost, CameraProtocol(stream.Spec.Input.Protocol), stream.Spec.Input.Port)
	return CameraEndpoint{
		Protocol:   CameraProtocol(stream.Spec.Input.Protocol),
		URL:        endpoint,
		QRCodeText: endpoint,
	}
}

func takeFromCR(cr domain.Take) Take {
	phase := takePhaseFromCR(cr)
	return Take{
		ID:              cr.Name,
		CameraID:        cr.Spec.StreamRef.Name,
		Phase:           phase,
		Output:          takeOutputFromCR(cr),
		StartedAt:       timePtrValue(cr.Status.StartedAt),
		StopRequestedAt: timePtrValue(cr.Spec.StopRequestedAt),
		StoppedAt:       timePtrValue(cr.Status.StoppedAt),
		CompletedAt:     timePtrValue(cr.Status.CompletedAt),
		Error:           cr.Status.Error,
		CreatedAt:       cr.CreationTimestamp.Time,
	}
}

func takeOutputFromCR(cr domain.Take) TakeOutput {
	s3 := cr.Spec.Output.S3
	uri := cr.Status.S3.URI
	if uri == "" && s3.Bucket != "" && s3.ObjectKey != "" {
		uri = "s3://" + s3.Bucket + "/" + s3.ObjectKey
	}
	return TakeOutput{S3: S3Output{
		Bucket:    s3.Bucket,
		ObjectKey: s3.ObjectKey,
		URI:       uri,
		Endpoint:  s3.Endpoint,
		Region:    s3.Region,
	}}
}

func cameraPhaseFromStream(stream domain.Stream) CameraPhase {
	switch stream.Status.Phase {
	case "Running":
		return CameraPhaseReady
	case "Degraded":
		return CameraPhaseDegraded
	case "Failed":
		return CameraPhaseFailed
	default:
		return CameraPhasePending
	}
}

func takePhaseFromCR(take domain.Take) TakePhase {
	switch take.Status.Phase {
	case "Capturing":
		if take.Spec.StopRequestedAt != nil {
			return TakePhaseStopRequested
		}
		return TakePhaseCapturing
	case "StopRequested":
		return TakePhaseStopRequested
	case "Uploading":
		return TakePhaseUploading
	case "Completed":
		return TakePhaseCompleted
	case "Failed":
		return TakePhaseFailed
	default:
		if take.Spec.StopRequestedAt != nil {
			return TakePhaseStopRequested
		}
		return TakePhasePending
	}
}

func timePtrValue(value *metav1.Time) time.Time {
	if value == nil {
		return time.Time{}
	}
	return value.Time
}

func allocateStreamName(base string, streams []domain.Stream) string {
	if base == "" {
		base = "camera"
	}
	existing := make(map[string]struct{}, len(streams))
	for _, stream := range streams {
		existing[stream.Name] = struct{}{}
	}
	id := base
	for suffix := 2; ; suffix++ {
		if _, ok := existing[id]; !ok {
			return id
		}
		id = fmt.Sprintf("%s-%d", base, suffix)
	}
}

func allocateTakeName(cameraID string, takes []domain.Take) string {
	base := slug(cameraID)
	if base == "" {
		base = "take"
	}
	base += "-take"
	existing := make(map[string]struct{}, len(takes))
	for _, take := range takes {
		existing[take.Name] = struct{}{}
	}
	id := base
	for suffix := 1; ; suffix++ {
		if _, ok := existing[id]; !ok {
			return id
		}
		id = fmt.Sprintf("%s-%d", base, suffix)
	}
}

func allocateInputPort(protocol CameraProtocol, streams []domain.Stream) int32 {
	base := srtBasePort
	if protocol == CameraProtocolRIST {
		base = ristBasePort
	}
	used := map[int32]struct{}{}
	for _, stream := range streams {
		if stream.Spec.Input.Port != 0 {
			used[stream.Spec.Input.Port] = struct{}{}
		}
	}
	for port := base; ; port += 2 {
		if _, ok := used[port]; !ok {
			return port
		}
	}
}

func nodePortForInput(protocol CameraProtocol, port int32) int32 {
	if protocol == CameraProtocolRIST {
		return ristBaseNodePort + (port - ristBasePort)
	}
	return srtBaseNodePort + (port - srtBasePort)
}

func listenerURI(protocol CameraProtocol, port int32) string {
	if protocol == CameraProtocolRIST {
		return fmt.Sprintf("rist://@:%d", port)
	}
	return fmt.Sprintf("srt://:%d?mode=listener", port)
}

func cameraEndpointURL(host string, protocol CameraProtocol, port int32) string {
	if protocol == CameraProtocolRIST {
		return (&url.URL{
			Scheme: "rist",
			Host:   net.JoinHostPort(host, strconv.Itoa(int(port))),
		}).String()
	}
	return (&url.URL{
		Scheme:   "srt",
		Host:     net.JoinHostPort(host, strconv.Itoa(int(port))),
		RawQuery: "mode=caller&latency=200000",
	}).String()
}

func mapK8sError(err error) error {
	if err == nil {
		return nil
	}
	if apierrors.IsNotFound(err) {
		return ErrNotFound
	}
	if apierrors.IsAlreadyExists(err) {
		return ErrConflict
	}
	return err
}
