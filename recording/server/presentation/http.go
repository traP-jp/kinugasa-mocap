package presentation

import (
	"context"
	"errors"
	"net/http"
	"time"

	"github.com/comavius/kinugasa-mocap/recording/server/presentation/api"
	"github.com/comavius/kinugasa-mocap/recording/server/service"
)

type APIServer struct {
	addr    string
	handler http.Handler
}

func NewAPIServer(addr string, usecase service.Usecase) (*APIServer, error) {
	handler, err := api.NewServer(newAPIHandler(usecase))
	if err != nil {
		return nil, err
	}
	return &APIServer{
		addr:    addr,
		handler: handler,
	}, nil
}

func (s *APIServer) Addr() string {
	return s.addr
}

func (s *APIServer) Handler() http.Handler {
	return s.handler
}

func (s *APIServer) Start(ctx context.Context) error {
	server := &http.Server{
		Addr:              s.addr,
		Handler:           s.handler,
		ReadHeaderTimeout: 10 * time.Second,
	}
	errCh := make(chan error, 1)
	go func() {
		errCh <- server.ListenAndServe()
	}()

	select {
	case <-ctx.Done():
		shutdownCtx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		err := server.Shutdown(shutdownCtx)
		if errors.Is(err, http.ErrServerClosed) {
			return nil
		}
		return err
	case err := <-errCh:
		if errors.Is(err, http.ErrServerClosed) {
			return nil
		}
		return err
	}
}
