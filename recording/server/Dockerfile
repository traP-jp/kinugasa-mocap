FROM golang:1.26 AS build
WORKDIR /src

COPY go.work ./
COPY recording/config/go.mod recording/config/go.mod
COPY recording/server/go.mod recording/server/go.sum recording/server/
RUN cd recording/server && go mod download

COPY recording/config recording/config
COPY recording/server recording/server
RUN cd recording/server && CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -o /recording-server .

FROM gcr.io/distroless/static-debian12:nonroot
COPY --from=build /recording-server /recording-server
USER nonroot:nonroot
ENTRYPOINT ["/recording-server"]
