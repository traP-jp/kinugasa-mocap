package main

import (
	"context"
	"flag"
	"log"
	"os"
	"os/signal"
	"syscall"

	"github.com/comavius/kinugasa-mocap/recording/config"
	"github.com/comavius/kinugasa-mocap/recording/server/infra/k8s"
	"github.com/comavius/kinugasa-mocap/recording/server/presentation"
	"github.com/comavius/kinugasa-mocap/recording/server/service"
	"k8s.io/apimachinery/pkg/runtime"
	clientgoscheme "k8s.io/client-go/kubernetes/scheme"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/log/zap"
	metricsserver "sigs.k8s.io/controller-runtime/pkg/metrics/server"
)

func main() {
	addr := flag.String("listen", ":8080", "address to listen on")
	printCRD := flag.Bool("print-crd", false, "print the bundled Recording CRDs and exit")
	enableOperator := flag.Bool("enable-operator", false, "run Kubernetes controllers for Stream and Recording resources")
	relayImage := flag.String("stream-relay-image", "linuxserver/ffmpeg:latest", "image used for Stream relay pods; must provide /bin/sh and ffmpeg")
	recorderImage := flag.String("recording-recorder-image", "linuxserver/ffmpeg:latest", "image used for one-shot Recording recorder containers; must provide /bin/sh and ffmpeg")
	uploaderImage := flag.String("recording-uploader-image", "rclone/rclone:latest", "image used for one-shot Recording uploader containers; must provide /bin/sh and rclone")
	liveKitURL := flag.String("livekit-url", "http://livekit-server.recording-system.svc.cluster.local:7880", "LiveKit server URL used by the operator")
	liveKitAPIKey := flag.String("livekit-api-key", "devkey", "LiveKit API key used by the operator")
	liveKitAPISecret := flag.String("livekit-api-secret", "secret", "LiveKit API secret used by the operator")
	liveKitWHIPBaseURL := flag.String("livekit-whip-base-url", "http://livekit-ingress.recording-system.svc.cluster.local:8080/whip", "WHIP base URL exposed by LiveKit ingress inside the cluster")
	metricsAddr := flag.String("metrics-bind-address", "0", "address for controller-runtime metrics; 0 disables metrics")
	opts := zap.Options{Development: true}
	opts.BindFlags(flag.CommandLine)
	flag.Parse()

	crdService := service.NewCRDService(config.CRDManifest())
	if *printCRD {
		if _, err := os.Stdout.Write(crdService.Manifest()); err != nil {
			log.Fatal(err)
		}
		return
	}

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	apiServer := presentation.NewAPIServer(*addr, crdService)
	if *enableOperator {
		if err := runOperator(ctx, apiServer, operatorOptions{
			MetricsAddr:   *metricsAddr,
			RelayImage:    *relayImage,
			RecorderImage: *recorderImage,
			UploaderImage: *uploaderImage,
			LiveKit: k8s.LiveKitIngressOptions{
				URL:         *liveKitURL,
				APIKey:      *liveKitAPIKey,
				APISecret:   *liveKitAPISecret,
				WHIPBaseURL: *liveKitWHIPBaseURL,
			},
			ZapOptions: opts,
		}); err != nil {
			log.Fatal(err)
		}
		return
	}

	log.Printf("recording server listening on %s", *addr)
	if err := apiServer.Start(ctx); err != nil {
		log.Fatal(err)
	}
}

type operatorOptions struct {
	MetricsAddr   string
	RelayImage    string
	RecorderImage string
	UploaderImage string
	LiveKit       k8s.LiveKitIngressOptions
	ZapOptions    zap.Options
}

func runOperator(ctx context.Context, apiServer *presentation.APIServer, options operatorOptions) error {
	ctrl.SetLogger(zap.New(zap.UseFlagOptions(&options.ZapOptions)))

	scheme := runtime.NewScheme()
	if err := clientgoscheme.AddToScheme(scheme); err != nil {
		return err
	}
	if err := k8s.AddToScheme(scheme); err != nil {
		return err
	}

	mgr, err := ctrl.NewManager(ctrl.GetConfigOrDie(), ctrl.Options{
		Scheme:  scheme,
		Metrics: metricsserver.Options{BindAddress: options.MetricsAddr},
	})
	if err != nil {
		return err
	}

	liveKitIngress, err := k8s.NewSDKLiveKitIngressManager(options.LiveKit)
	if err != nil {
		return err
	}

	if err := (&k8s.StreamReconciler{
		Client:         mgr.GetClient(),
		Scheme:         mgr.GetScheme(),
		Options:        k8s.StreamWorkloadOptions{RelayImage: options.RelayImage},
		LiveKitIngress: liveKitIngress,
	}).SetupWithManager(mgr); err != nil {
		return err
	}
	if err := (&k8s.RecordingReconciler{
		Client: mgr.GetClient(),
		Scheme: mgr.GetScheme(),
		Options: k8s.RecordingJobOptions{
			RecorderImage: options.RecorderImage,
			UploaderImage: options.UploaderImage,
		},
	}).SetupWithManager(mgr); err != nil {
		return err
	}
	if err := mgr.Add(apiServer); err != nil {
		return err
	}

	log.Printf("recording operator listening on %s", apiServer.Addr())
	return mgr.Start(ctx)
}
