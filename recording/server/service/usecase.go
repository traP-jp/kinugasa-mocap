package service

import (
	"context"
	"errors"
	"regexp"
	"strings"
	"time"
)

var (
	ErrInvalid  = errors.New("invalid request")
	ErrNotFound = errors.New("not found")
	ErrConflict = errors.New("conflict")
)

type CameraProtocol string

const (
	CameraProtocolSRT  CameraProtocol = "srt"
	CameraProtocolRIST CameraProtocol = "rist"
)

type CameraPhase string

const (
	CameraPhasePending  CameraPhase = "pending"
	CameraPhaseReady    CameraPhase = "ready"
	CameraPhaseDegraded CameraPhase = "degraded"
	CameraPhaseFailed   CameraPhase = "failed"
)

type TakePhase string

const (
	TakePhasePending       TakePhase = "pending"
	TakePhaseCapturing     TakePhase = "capturing"
	TakePhaseStopRequested TakePhase = "stop_requested"
	TakePhaseUploading     TakePhase = "uploading"
	TakePhaseCompleted     TakePhase = "completed"
	TakePhaseFailed        TakePhase = "failed"
)

type Usecase interface {
	CreateCamera(context.Context, CreateCameraInput) (Camera, error)
	DeleteCamera(context.Context, string) error
	GetCamera(context.Context, string) (Camera, error)
	GetCameraEndpoint(context.Context, string) (CameraEndpoint, error)
	ListCameras(context.Context, ListCamerasInput) ([]Camera, error)
	StartTake(context.Context, StartTakeInput) (Take, error)
	StopTake(context.Context, StopTakeInput) (Take, error)
	GetTake(context.Context, string) (Take, error)
	ListTakes(context.Context, ListTakesInput) ([]Take, error)
	GetLiveKitInfo(context.Context) (LiveKitInfo, error)
	ListLiveKitRooms(context.Context) ([]LiveKitRoom, error)
	GetLiveKitConnection(context.Context, LiveKitConnectionInput) (LiveKitConnection, error)
}

type UsecaseConfig struct {
	PublicIngestHost   string
	ResourceNamespace  string
	LiveKitPublicURL   string
	LiveKitTokenIssuer LiveKitTokenIssuer
	Now                func() time.Time
}

type CreateCameraInput struct {
	DisplayName string
	Protocol    CameraProtocol
	LiveKitRoom string
}

type ListCamerasInput struct {
	Protocol *CameraProtocol
}

type Camera struct {
	ID          string
	DisplayName string
	Protocol    CameraProtocol
	Phase       CameraPhase
	Endpoint    CameraEndpoint
	LiveKit     CameraLiveKit
	Error       string
	CreatedAt   time.Time
	UpdatedAt   time.Time
}

type CameraEndpoint struct {
	Protocol   CameraProtocol
	URL        string
	QRCodeText string
}

type CameraLiveKit struct {
	Room                string
	Connected           bool
	ParticipantIdentity string
	ParticipantName     string
}

type StartTakeInput struct {
	CameraID string
	Output   TakeOutputRequest
}

type StopTakeInput struct {
	TakeID      string
	RequestedAt time.Time
}

type ListTakesInput struct {
	CameraID string
	Phase    *TakePhase
}

type TakeOutputRequest struct {
	S3 S3OutputRequest
}

type S3OutputRequest struct {
	Bucket    string
	ObjectKey string
	Endpoint  string
	Region    string
}

type Take struct {
	ID              string
	CameraID        string
	Phase           TakePhase
	Output          TakeOutput
	StartedAt       time.Time
	StopRequestedAt time.Time
	StoppedAt       time.Time
	CompletedAt     time.Time
	Error           string
	CreatedAt       time.Time
	UpdatedAt       time.Time
}

type TakeOutput struct {
	S3 S3Output
}

type S3Output struct {
	Bucket    string
	ObjectKey string
	URI       string
	Endpoint  string
	Region    string
}

type LiveKitInfo struct {
	URL string
}

type LiveKitRoom struct {
	Name      string
	CameraID  string
	Connected bool
}

type LiveKitConnectionInput struct {
	RoomName            string
	ParticipantIdentity string
	ParticipantName     string
}

type LiveKitConnection struct {
	URL                 string
	Room                string
	Token               string
	ParticipantIdentity string
	ParticipantName     string
}

type LiveKitTokenRequest struct {
	RoomName            string
	ParticipantIdentity string
	ParticipantName     string
}

type LiveKitTokenIssuer interface {
	IssueLiveKitToken(context.Context, LiveKitTokenRequest) (string, error)
}

func takeActive(phase TakePhase) bool {
	switch phase {
	case TakePhasePending, TakePhaseCapturing, TakePhaseStopRequested, TakePhaseUploading:
		return true
	default:
		return false
	}
}

var slugPattern = regexp.MustCompile(`[^a-z0-9-]+`)

func slug(value string) string {
	value = strings.ToLower(strings.TrimSpace(value))
	value = strings.ReplaceAll(value, "_", "-")
	value = strings.ReplaceAll(value, " ", "-")
	value = slugPattern.ReplaceAllString(value, "-")
	value = strings.Trim(value, "-")
	for strings.Contains(value, "--") {
		value = strings.ReplaceAll(value, "--", "-")
	}
	return value
}
