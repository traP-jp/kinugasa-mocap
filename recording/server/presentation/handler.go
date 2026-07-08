package presentation

import (
	"context"
	"errors"
	"strings"
	"time"

	"github.com/comavius/kinugasa-mocap/recording/server/presentation/api"
	"github.com/comavius/kinugasa-mocap/recording/server/service"
)

type apiHandler struct {
	usecase service.Usecase
}

func newAPIHandler(usecase service.Usecase) *apiHandler {
	return &apiHandler{usecase: usecase}
}

func (h *apiHandler) GetHealthz(ctx context.Context) (api.GetHealthzOK, error) {
	return api.GetHealthzOK{Data: strings.NewReader("ok\n")}, nil
}

func (h *apiHandler) CreateCamera(ctx context.Context, req *api.CreateCameraRequest) (api.CreateCameraRes, error) {
	camera, err := h.usecase.CreateCamera(ctx, service.CreateCameraInput{
		DisplayName: req.DisplayName,
		Protocol:    fromAPICameraProtocol(req.Protocol),
		LiveKitRoom: req.LiveKitRoom.Or(""),
	})
	if err != nil {
		switch {
		case errors.Is(err, service.ErrInvalid):
			return createCameraBadRequest(err), nil
		case errors.Is(err, service.ErrConflict):
			return createCameraConflict(err), nil
		default:
			return nil, err
		}
	}
	return toAPICamera(camera), nil
}

func (h *apiHandler) ListCameras(ctx context.Context, params api.ListCamerasParams) (*api.CameraList, error) {
	var input service.ListCamerasInput
	if protocol, ok := params.Protocol.Get(); ok {
		value := fromAPICameraProtocol(protocol)
		input.Protocol = &value
	}
	cameras, err := h.usecase.ListCameras(ctx, input)
	if err != nil {
		return nil, err
	}
	items := make([]api.Camera, 0, len(cameras))
	for _, camera := range cameras {
		items = append(items, *toAPICamera(camera))
	}
	return &api.CameraList{Items: items}, nil
}

func (h *apiHandler) GetCamera(ctx context.Context, params api.GetCameraParams) (api.GetCameraRes, error) {
	camera, err := h.usecase.GetCamera(ctx, params.CameraId)
	if err != nil {
		if errors.Is(err, service.ErrNotFound) {
			return errorResponse("not_found", "camera not found"), nil
		}
		return nil, err
	}
	return toAPICamera(camera), nil
}

func (h *apiHandler) DeleteCamera(ctx context.Context, params api.DeleteCameraParams) (api.DeleteCameraRes, error) {
	err := h.usecase.DeleteCamera(ctx, params.CameraId)
	if err != nil {
		switch {
		case errors.Is(err, service.ErrNotFound):
			return deleteCameraNotFound(err), nil
		case errors.Is(err, service.ErrConflict):
			return deleteCameraConflict(err), nil
		default:
			return nil, err
		}
	}
	return &api.DeleteCameraNoContent{}, nil
}

func (h *apiHandler) GetCameraEndpoint(ctx context.Context, params api.GetCameraEndpointParams) (api.GetCameraEndpointRes, error) {
	endpoint, err := h.usecase.GetCameraEndpoint(ctx, params.CameraId)
	if err != nil {
		if errors.Is(err, service.ErrNotFound) {
			return errorResponse("not_found", "camera not found"), nil
		}
		return nil, err
	}
	return toAPICameraEndpoint(endpoint), nil
}

func (h *apiHandler) StartTake(ctx context.Context, req *api.StartTakeRequest) (api.StartTakeRes, error) {
	take, err := h.usecase.StartTake(ctx, service.StartTakeInput{
		CameraID: req.CameraId,
		Output: service.TakeOutputRequest{
			S3: service.S3OutputRequest{
				Bucket:    req.Output.S3.Bucket,
				ObjectKey: req.Output.S3.ObjectKey.Or(""),
				Endpoint:  req.Output.S3.Endpoint.Or(""),
				Region:    req.Output.S3.Region.Or(""),
			},
		},
	})
	if err != nil {
		switch {
		case errors.Is(err, service.ErrInvalid):
			return startTakeBadRequest(err), nil
		case errors.Is(err, service.ErrNotFound):
			return startTakeNotFound(err), nil
		case errors.Is(err, service.ErrConflict):
			return startTakeConflict(err), nil
		default:
			return nil, err
		}
	}
	return toAPITake(take), nil
}

func (h *apiHandler) ListTakes(ctx context.Context, params api.ListTakesParams) (*api.TakeList, error) {
	var input service.ListTakesInput
	input.CameraID = params.CameraId.Or("")
	if phase, ok := params.Phase.Get(); ok {
		value := fromAPITakePhase(phase)
		input.Phase = &value
	}
	takes, err := h.usecase.ListTakes(ctx, input)
	if err != nil {
		return nil, err
	}
	items := make([]api.Take, 0, len(takes))
	for _, take := range takes {
		items = append(items, *toAPITake(take))
	}
	return &api.TakeList{Items: items}, nil
}

func (h *apiHandler) GetTake(ctx context.Context, params api.GetTakeParams) (api.GetTakeRes, error) {
	take, err := h.usecase.GetTake(ctx, params.TakeId)
	if err != nil {
		if errors.Is(err, service.ErrNotFound) {
			return errorResponse("not_found", "take not found"), nil
		}
		return nil, err
	}
	return toAPITake(take), nil
}

func (h *apiHandler) StopTake(ctx context.Context, req api.OptStopTakeRequest, params api.StopTakeParams) (api.StopTakeRes, error) {
	input := service.StopTakeInput{TakeID: params.TakeId}
	if value, ok := req.Get(); ok {
		input.RequestedAt = value.RequestedAt.Or(input.RequestedAt)
	}
	take, err := h.usecase.StopTake(ctx, input)
	if err != nil {
		switch {
		case errors.Is(err, service.ErrNotFound):
			return stopTakeNotFound(err), nil
		case errors.Is(err, service.ErrConflict):
			return stopTakeConflict(err), nil
		default:
			return nil, err
		}
	}
	return toAPITake(take), nil
}

func (h *apiHandler) GetLiveKitInfo(ctx context.Context) (*api.LiveKitInfo, error) {
	info, err := h.usecase.GetLiveKitInfo(ctx)
	if err != nil {
		return nil, err
	}
	return &api.LiveKitInfo{URL: info.URL}, nil
}

func (h *apiHandler) ListLiveKitRooms(ctx context.Context) (*api.LiveKitRoomList, error) {
	rooms, err := h.usecase.ListLiveKitRooms(ctx)
	if err != nil {
		return nil, err
	}
	items := make([]api.LiveKitRoom, 0, len(rooms))
	for _, room := range rooms {
		items = append(items, api.LiveKitRoom{
			Name:      room.Name,
			CameraId:  room.CameraID,
			Connected: api.NewOptBool(room.Connected),
		})
	}
	return &api.LiveKitRoomList{Items: items}, nil
}

func (h *apiHandler) GetLiveKitConnection(ctx context.Context, params api.GetLiveKitConnectionParams) (api.GetLiveKitConnectionRes, error) {
	connection, err := h.usecase.GetLiveKitConnection(ctx, service.LiveKitConnectionInput{
		RoomName:            params.RoomName,
		ParticipantIdentity: params.ParticipantIdentity.Or(""),
		ParticipantName:     params.ParticipantName.Or(""),
	})
	if err != nil {
		if errors.Is(err, service.ErrNotFound) {
			return errorResponse("not_found", "livekit room not found"), nil
		}
		return nil, err
	}
	return &api.LiveKitConnection{
		URL:                 connection.URL,
		Room:                connection.Room,
		Token:               connection.Token,
		ParticipantIdentity: connection.ParticipantIdentity,
		ParticipantName:     optString(connection.ParticipantName),
	}, nil
}

func toAPICamera(camera service.Camera) *api.Camera {
	return &api.Camera{
		ID:          camera.ID,
		DisplayName: camera.DisplayName,
		Protocol:    toAPICameraProtocol(camera.Protocol),
		Phase:       toAPICameraPhase(camera.Phase),
		Endpoint:    *toAPICameraEndpoint(camera.Endpoint),
		LiveKit: api.CameraLiveKit{
			Room:                camera.LiveKit.Room,
			Connected:           camera.LiveKit.Connected,
			ParticipantIdentity: optString(camera.LiveKit.ParticipantIdentity),
			ParticipantName:     optString(camera.LiveKit.ParticipantName),
		},
		Error:     optString(camera.Error),
		CreatedAt: camera.CreatedAt,
		UpdatedAt: optTime(camera.UpdatedAt),
	}
}

func toAPICameraEndpoint(endpoint service.CameraEndpoint) *api.CameraEndpoint {
	return &api.CameraEndpoint{
		Protocol:   toAPICameraProtocol(endpoint.Protocol),
		URL:        endpoint.URL,
		QrCodeText: endpoint.QRCodeText,
	}
}

func toAPITake(take service.Take) *api.Take {
	return &api.Take{
		ID:       take.ID,
		CameraId: take.CameraID,
		Phase:    toAPITakePhase(take.Phase),
		Output: api.TakeOutput{S3: api.S3Output{
			Bucket:    take.Output.S3.Bucket,
			ObjectKey: take.Output.S3.ObjectKey,
			URI:       optString(take.Output.S3.URI),
			Endpoint:  optString(take.Output.S3.Endpoint),
			Region:    optString(take.Output.S3.Region),
		}},
		StartedAt:       optTime(take.StartedAt),
		StopRequestedAt: optTime(take.StopRequestedAt),
		StoppedAt:       optTime(take.StoppedAt),
		CompletedAt:     optTime(take.CompletedAt),
		Error:           optString(take.Error),
		CreatedAt:       take.CreatedAt,
		UpdatedAt:       optTime(take.UpdatedAt),
	}
}

func fromAPICameraProtocol(protocol api.CameraProtocol) service.CameraProtocol {
	return service.CameraProtocol(protocol)
}

func toAPICameraProtocol(protocol service.CameraProtocol) api.CameraProtocol {
	return api.CameraProtocol(protocol)
}

func toAPICameraPhase(phase service.CameraPhase) api.CameraPhase {
	return api.CameraPhase(phase)
}

func fromAPITakePhase(phase api.TakePhase) service.TakePhase {
	return service.TakePhase(phase)
}

func toAPITakePhase(phase service.TakePhase) api.TakePhase {
	return api.TakePhase(phase)
}

func optString(value string) api.OptString {
	if value == "" {
		return api.OptString{}
	}
	return api.NewOptString(value)
}

func optTime(value time.Time) api.OptDateTime {
	if value.IsZero() {
		return api.OptDateTime{}
	}
	return api.NewOptDateTime(value)
}

func errorResponse(code, message string) *api.Error {
	return &api.Error{Code: code, Message: message}
}

func createCameraBadRequest(err error) *api.CreateCameraBadRequest {
	res := api.CreateCameraBadRequest(*errorResponse("bad_request", err.Error()))
	return &res
}

func createCameraConflict(err error) *api.CreateCameraConflict {
	res := api.CreateCameraConflict(*errorResponse("conflict", err.Error()))
	return &res
}

func deleteCameraNotFound(err error) *api.DeleteCameraNotFound {
	res := api.DeleteCameraNotFound(*errorResponse("not_found", err.Error()))
	return &res
}

func deleteCameraConflict(err error) *api.DeleteCameraConflict {
	res := api.DeleteCameraConflict(*errorResponse("conflict", err.Error()))
	return &res
}

func startTakeBadRequest(err error) *api.StartTakeBadRequest {
	res := api.StartTakeBadRequest(*errorResponse("bad_request", err.Error()))
	return &res
}

func startTakeNotFound(err error) *api.StartTakeNotFound {
	res := api.StartTakeNotFound(*errorResponse("not_found", err.Error()))
	return &res
}

func startTakeConflict(err error) *api.StartTakeConflict {
	res := api.StartTakeConflict(*errorResponse("conflict", err.Error()))
	return &res
}

func stopTakeNotFound(err error) *api.StopTakeNotFound {
	res := api.StopTakeNotFound(*errorResponse("not_found", err.Error()))
	return &res
}

func stopTakeConflict(err error) *api.StopTakeConflict {
	res := api.StopTakeConflict(*errorResponse("conflict", err.Error()))
	return &res
}
