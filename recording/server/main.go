package main

import (
	"context"
	"flag"
	"log"
	"os"
	"os/signal"
	"syscall"

	"github.com/comavius/kinugasa-mocap/recording/config"
	"github.com/comavius/kinugasa-mocap/recording/server/presentation"
	"github.com/comavius/kinugasa-mocap/recording/server/service"
)

func main() {
	addr := flag.String("listen", ":8080", "address to listen on")
	printCRD := flag.Bool("print-crd", false, "print the bundled Recording CRD and exit")
	flag.Parse()

	crdService := service.NewCRDService(config.RecordingCRDManifest())
	if *printCRD {
		if _, err := os.Stdout.Write(crdService.Manifest()); err != nil {
			log.Fatal(err)
		}
		return
	}

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	apiServer := presentation.NewAPIServer(*addr, crdService)
	log.Printf("recording server listening on %s", *addr)
	if err := apiServer.Start(ctx); err != nil {
		log.Fatal(err)
	}
}
