package k8s

import (
	"context"
	"errors"
	"fmt"
	"strings"

	"github.com/comavius/kinugasa-mocap/recording/server/domain"
	"github.com/livekit/protocol/livekit"
	lksdk "github.com/livekit/server-sdk-go/v2"
	"github.com/twitchtv/twirp"
)

type LiveKitIngressOptions struct {
	URL         string
	APIKey      string
	APISecret   string
	WHIPBaseURL string
}

type LiveKitIngressInfo struct {
	ID        string
	URL       string
	StreamKey string
}

var (
	errMissingLiveKitIngressManager = errors.New("livekit ingress manager is required for managed livekit ingress")
	errMissingLiveKitIngressURL     = errors.New("livekit ingress did not return an output URL")
)

type LiveKitIngressManager interface {
	EnsureIngress(context.Context, *domain.Stream) (*LiveKitIngressInfo, error)
	DeleteIngress(context.Context, string) error
}

type SDKLiveKitIngressManager struct {
	client      *lksdk.IngressClient
	whipBaseURL string
}

func NewSDKLiveKitIngressManager(options LiveKitIngressOptions) (*SDKLiveKitIngressManager, error) {
	if options.URL == "" {
		return nil, errors.New("livekit url is required")
	}
	if options.APIKey == "" {
		return nil, errors.New("livekit api key is required")
	}
	if options.APISecret == "" {
		return nil, errors.New("livekit api secret is required")
	}
	if options.WHIPBaseURL == "" {
		return nil, errors.New("livekit whip base url is required")
	}
	return &SDKLiveKitIngressManager{
		client:      lksdk.NewIngressClient(options.URL, options.APIKey, options.APISecret),
		whipBaseURL: strings.TrimRight(options.WHIPBaseURL, "/"),
	}, nil
}

func (m *SDKLiveKitIngressManager) EnsureIngress(ctx context.Context, stream *domain.Stream) (*LiveKitIngressInfo, error) {
	if stream.Spec.LiveKit.Room == "" {
		return nil, errors.New("livekit room is required")
	}

	name := liveKitIngressName(stream)
	existing, err := m.findIngress(ctx, stream.Spec.LiveKit.Room, name)
	if err != nil {
		return nil, err
	}
	if existing != nil {
		if existing.GetInputType() != livekit.IngressInput_WHIP_INPUT {
			if err := m.DeleteIngress(ctx, existing.GetIngressId()); err != nil {
				return nil, err
			}
		} else {
			return m.toIngressInfo(existing), nil
		}
	}

	ingress, err := m.client.CreateIngress(ctx, &livekit.CreateIngressRequest{
		InputType:           livekit.IngressInput_WHIP_INPUT,
		Name:                name,
		RoomName:            stream.Spec.LiveKit.Room,
		ParticipantIdentity: liveKitParticipantIdentity(stream),
		ParticipantName:     liveKitParticipantName(stream),
	})
	if err != nil {
		return nil, err
	}
	return m.toIngressInfo(ingress), nil
}

func (m *SDKLiveKitIngressManager) DeleteIngress(ctx context.Context, ingressID string) error {
	if ingressID == "" {
		return nil
	}
	_, err := m.client.DeleteIngress(ctx, &livekit.DeleteIngressRequest{IngressId: ingressID})
	if isTwirpNotFound(err) {
		return nil
	}
	return err
}

func (m *SDKLiveKitIngressManager) findIngress(ctx context.Context, room, name string) (*livekit.IngressInfo, error) {
	resp, err := m.client.ListIngress(ctx, &livekit.ListIngressRequest{RoomName: room})
	if err != nil {
		return nil, err
	}
	for _, ingress := range resp.GetItems() {
		if ingress.GetName() == name {
			return ingress, nil
		}
	}
	return nil, nil
}

func (m *SDKLiveKitIngressManager) toIngressInfo(info *livekit.IngressInfo) *LiveKitIngressInfo {
	streamKey := info.GetStreamKey()
	url := info.GetUrl()
	if streamKey != "" {
		url = fmt.Sprintf("%s/%s", m.whipBaseURL, streamKey)
	}
	return &LiveKitIngressInfo{
		ID:        info.GetIngressId(),
		URL:       url,
		StreamKey: streamKey,
	}
}

func liveKitIngressName(stream *domain.Stream) string {
	return stream.Namespace + "-" + stream.Name
}

func liveKitParticipantIdentity(stream *domain.Stream) string {
	if stream.Spec.LiveKit.ParticipantIdentity != "" {
		return stream.Spec.LiveKit.ParticipantIdentity
	}
	return stream.Name + "-ingress"
}

func liveKitParticipantName(stream *domain.Stream) string {
	if stream.Spec.LiveKit.ParticipantName != "" {
		return stream.Spec.LiveKit.ParticipantName
	}
	return stream.Name + " ingress"
}

func isTwirpNotFound(err error) bool {
	if err == nil {
		return false
	}
	var twerr twirp.Error
	return errors.As(err, &twerr) && twerr.Code() == twirp.NotFound
}
