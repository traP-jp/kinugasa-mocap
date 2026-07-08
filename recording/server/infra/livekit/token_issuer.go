package livekit

import (
	"context"
	"errors"
	"fmt"
	"strings"
	"time"

	"github.com/comavius/kinugasa-mocap/recording/server/service"
	"github.com/livekit/protocol/auth"
)

const defaultTokenTTL = time.Hour

type TokenIssuerOptions struct {
	APIKey    string
	APISecret string
	TokenTTL  time.Duration
}

type TokenIssuer struct {
	apiKey    string
	apiSecret string
	tokenTTL  time.Duration
}

func NewTokenIssuer(options TokenIssuerOptions) (*TokenIssuer, error) {
	if strings.TrimSpace(options.APIKey) == "" {
		return nil, errors.New("livekit api key is required")
	}
	if strings.TrimSpace(options.APISecret) == "" {
		return nil, errors.New("livekit api secret is required")
	}
	tokenTTL := options.TokenTTL
	if tokenTTL == 0 {
		tokenTTL = defaultTokenTTL
	}
	return &TokenIssuer{
		apiKey:    options.APIKey,
		apiSecret: options.APISecret,
		tokenTTL:  tokenTTL,
	}, nil
}

func (i *TokenIssuer) IssueLiveKitToken(ctx context.Context, request service.LiveKitTokenRequest) (string, error) {
	if err := ctx.Err(); err != nil {
		return "", err
	}
	room := strings.TrimSpace(request.RoomName)
	if room == "" {
		return "", fmt.Errorf("%w: livekit room is required", service.ErrInvalid)
	}
	identity := strings.TrimSpace(request.ParticipantIdentity)
	if identity == "" {
		return "", fmt.Errorf("%w: livekit participant identity is required", service.ErrInvalid)
	}

	canPublish := false
	canSubscribe := true
	canPublishData := false
	accessToken := auth.NewAccessToken(i.apiKey, i.apiSecret).
		SetIdentity(identity).
		SetValidFor(i.tokenTTL).
		SetVideoGrant(&auth.VideoGrant{
			RoomJoin:       true,
			Room:           room,
			CanPublish:     &canPublish,
			CanSubscribe:   &canSubscribe,
			CanPublishData: &canPublishData,
		})
	if name := strings.TrimSpace(request.ParticipantName); name != "" {
		accessToken.SetName(name)
	}
	return accessToken.ToJWT()
}
