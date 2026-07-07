package k8s

import (
	"context"

	"github.com/comavius/kinugasa-mocap/recording/server/domain"
	appsv1 "k8s.io/api/apps/v1"
	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/equality"
	apierrors "k8s.io/apimachinery/pkg/api/errors"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/types"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/controller/controllerutil"
)

type StreamReconciler struct {
	client.Client
	Scheme  *runtime.Scheme
	Options StreamWorkloadOptions
}

func (r *StreamReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	var stream domain.Stream
	if err := r.Get(ctx, req.NamespacedName, &stream); err != nil {
		if apierrors.IsNotFound(err) {
			return ctrl.Result{}, nil
		}
		return ctrl.Result{}, err
	}

	if err := r.reconcileDeployment(ctx, &stream); err != nil {
		return ctrl.Result{}, err
	}
	if err := r.reconcileService(ctx, &stream); err != nil {
		return ctrl.Result{}, err
	}
	if err := r.updateStatus(ctx, &stream); err != nil {
		return ctrl.Result{}, err
	}
	return ctrl.Result{}, nil
}

func (r *StreamReconciler) reconcileDeployment(ctx context.Context, stream *domain.Stream) error {
	var existing appsv1.Deployment
	key := types.NamespacedName{Name: streamWorkloadName(stream), Namespace: stream.Namespace}
	err := r.Get(ctx, key, &existing)
	if apierrors.IsNotFound(err) {
		deployment := BuildStreamDeployment(stream, r.Options)
		if err := controllerutil.SetControllerReference(stream, deployment, r.Scheme); err != nil {
			return err
		}
		return r.Create(ctx, deployment)
	}
	if err != nil {
		return err
	}
	if UpdateStreamDeployment(&existing, stream, r.Options) {
		return r.Update(ctx, &existing)
	}
	return nil
}

func (r *StreamReconciler) reconcileService(ctx context.Context, stream *domain.Stream) error {
	var existing corev1.Service
	key := types.NamespacedName{Name: streamWorkloadName(stream), Namespace: stream.Namespace}
	err := r.Get(ctx, key, &existing)
	if apierrors.IsNotFound(err) {
		service := BuildStreamService(stream)
		if err := controllerutil.SetControllerReference(stream, service, r.Scheme); err != nil {
			return err
		}
		return r.Create(ctx, service)
	}
	if err != nil {
		return err
	}
	if UpdateStreamService(&existing, stream) {
		return r.Update(ctx, &existing)
	}
	return nil
}

func (r *StreamReconciler) updateStatus(ctx context.Context, stream *domain.Stream) error {
	next := stream.DeepCopyObject().(*domain.Stream)
	next.Status.ObservedGeneration = stream.Generation
	next.Status.Phase = "Running"
	next.Status.ServiceName = streamWorkloadName(stream)
	next.Status.RecordingEndpoint = streamRecordingEndpoint(stream)
	next.Status.Error = ""
	if equality.Semantic.DeepEqual(stream.Status, next.Status) {
		return nil
	}
	return r.Status().Update(ctx, next)
}

func (r *StreamReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&domain.Stream{}).
		Owns(&appsv1.Deployment{}).
		Owns(&corev1.Service{}).
		Complete(r)
}
