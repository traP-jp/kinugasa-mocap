package presentation

import (
	"context"
	"errors"
	"net/http"
	"time"

	"github.com/comavius/kinugasa-mocap/recording/server/service"
	"github.com/labstack/echo/v4"
)

type APIServer struct {
	addr    string
	service *service.CRDService
}

func NewAPIServer(addr string, service *service.CRDService) *APIServer {
	return &APIServer{
		addr:    addr,
		service: service,
	}
}

func (s *APIServer) Start(ctx context.Context) error {
	e := s.Router()
	errCh := make(chan error, 1)
	go func() {
		errCh <- e.Start(s.addr)
	}()

	select {
	case <-ctx.Done():
		shutdownCtx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		err := e.Shutdown(shutdownCtx)
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

func (s *APIServer) Router() *echo.Echo {
	e := echo.New()
	e.HideBanner = true
	e.HidePort = true

	e.GET("/healthz", func(c echo.Context) error {
		return c.String(http.StatusOK, "ok\n")
	})
	e.GET("/crd", func(c echo.Context) error {
		return c.Blob(http.StatusOK, "application/yaml; charset=utf-8", s.service.Manifest())
	})

	return e
}
