package livekit

import (
	"errors"
	"testing"
	"time"

	"github.com/comavius/kinugasa-mocap/recording/server/service"
	"github.com/golang-jwt/jwt/v5"
)

func TestTokenIssuerIssuesViewerToken(t *testing.T) {
	issuer, err := NewTokenIssuer(TokenIssuerOptions{
		APIKey:    "devkey",
		APISecret: "secret",
		TokenTTL:  time.Minute,
	})
	if err != nil {
		t.Fatal(err)
	}

	token, err := issuer.IssueLiveKitToken(t.Context(), service.LiveKitTokenRequest{
		RoomName:            "studio-a",
		ParticipantIdentity: "viewer-1",
		ParticipantName:     "Viewer 1",
	})
	if err != nil {
		t.Fatal(err)
	}

	claims := jwt.MapClaims{}
	parsed, err := jwt.ParseWithClaims(token, claims, func(token *jwt.Token) (any, error) {
		return []byte("secret"), nil
	})
	if err != nil {
		t.Fatal(err)
	}
	if !parsed.Valid {
		t.Fatalf("token is not valid")
	}
	if claims["iss"] != "devkey" {
		t.Fatalf("issuer = %q, want devkey", claims["iss"])
	}
	if claims["sub"] != "viewer-1" {
		t.Fatalf("subject = %q, want viewer-1", claims["sub"])
	}
	if claims["name"] != "Viewer 1" {
		t.Fatalf("name = %q, want Viewer 1", claims["name"])
	}
	video, ok := claims["video"].(map[string]any)
	if !ok {
		t.Fatalf("video claims = %#v, want object", claims["video"])
	}
	if video["room"] != "studio-a" {
		t.Fatalf("room = %q, want studio-a", video["room"])
	}
	if video["roomJoin"] != true {
		t.Fatalf("roomJoin = %v, want true", video["roomJoin"])
	}
	if video["canSubscribe"] != true {
		t.Fatalf("canSubscribe = %v, want true", video["canSubscribe"])
	}
	if video["canPublish"] != false {
		t.Fatalf("canPublish = %v, want false", video["canPublish"])
	}
	if video["canPublishData"] != false {
		t.Fatalf("canPublishData = %v, want false", video["canPublishData"])
	}
}

func TestTokenIssuerRejectsMissingRoom(t *testing.T) {
	issuer, err := NewTokenIssuer(TokenIssuerOptions{APIKey: "devkey", APISecret: "secret"})
	if err != nil {
		t.Fatal(err)
	}

	_, err = issuer.IssueLiveKitToken(t.Context(), service.LiveKitTokenRequest{
		ParticipantIdentity: "viewer-1",
	})
	if !errors.Is(err, service.ErrInvalid) {
		t.Fatalf("err = %v, want %v", err, service.ErrInvalid)
	}
}
