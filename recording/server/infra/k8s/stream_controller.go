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

const liveKitIngressFinalizer = "recording.kinugasa.dev/livekit-ingress"

type StreamReconciler struct {
	client.Client
	Scheme         *runtime.Scheme
	Options        StreamWorkloadOptions
	LiveKitIngress LiveKitIngressManager
}

func (r *StreamReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	var stream domain.Stream
	if err := r.Get(ctx, req.NamespacedName, &stream); err != nil {
		if apierrors.IsNotFound(err) {
			return ctrl.Result{}, nil
		}
		return ctrl.Result{}, err
	}

	if !stream.ObjectMeta.DeletionTimestamp.IsZero() {
		return ctrl.Result{}, r.finalizeStream(ctx, &stream)
	}
	if stream.Spec.LiveKit.URL == "" && !controllerutil.ContainsFinalizer(&stream, liveKitIngressFinalizer) {
		controllerutil.AddFinalizer(&stream, liveKitIngressFinalizer)
		return ctrl.Result{Requeue: true}, r.Update(ctx, &stream)
	}
	previousStatus := stream.Status
	if err := r.reconcileLiveKitIngress(ctx, &stream); err != nil {
		_ = r.updateStatusError(ctx, &stream, err)
		return ctrl.Result{}, err
	}

	if err := r.reconcileDeployment(ctx, &stream); err != nil {
		return ctrl.Result{}, err
	}
	if err := r.reconcileService(ctx, &stream); err != nil {
		return ctrl.Result{}, err
	}
	if err := r.updateStatus(ctx, &stream, previousStatus); err != nil {
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

func (r *StreamReconciler) reconcileLiveKitIngress(ctx context.Context, stream *domain.Stream) error {
	if stream.Spec.LiveKit.URL != "" {
		if err := r.cleanupManagedLiveKitIngress(ctx, stream); err != nil {
			return err
		}
		if controllerutil.ContainsFinalizer(stream, liveKitIngressFinalizer) {
			controllerutil.RemoveFinalizer(stream, liveKitIngressFinalizer)
			if err := r.Update(ctx, stream); err != nil {
				return err
			}
		}
		return nil
	}
	if r.LiveKitIngress == nil {
		return errMissingLiveKitIngressManager
	}

	ingress, err := r.LiveKitIngress.EnsureIngress(ctx, stream)
	if err != nil {
		return err
	}
	if ingress.URL == "" {
		return errMissingLiveKitIngressURL
	}
	if err := r.reconcileLiveKitSecret(ctx, stream, ingress); err != nil {
		return err
	}
	stream.Status.LiveKitIngressID = ingress.ID
	stream.Status.LiveKitSecretName = liveKitSecretName(stream)
	return nil
}

func (r *StreamReconciler) reconcileLiveKitSecret(ctx context.Context, stream *domain.Stream, ingress *LiveKitIngressInfo) error {
	secret := buildLiveKitSecret(stream, ingress)
	if err := controllerutil.SetControllerReference(stream, secret, r.Scheme); err != nil {
		return err
	}

	var existing corev1.Secret
	key := types.NamespacedName{Name: secret.Name, Namespace: secret.Namespace}
	err := r.Get(ctx, key, &existing)
	if apierrors.IsNotFound(err) {
		return r.Create(ctx, secret)
	}
	if err != nil {
		return err
	}

	changed := false
	if !equality.Semantic.DeepEqual(existing.Labels, secret.Labels) {
		existing.Labels = secret.Labels
		changed = true
	}
	if existing.Type != secret.Type {
		existing.Type = secret.Type
		changed = true
	}
	if !equality.Semantic.DeepEqual(existing.Data, secret.Data) {
		existing.Data = secret.Data
		changed = true
	}
	if !equality.Semantic.DeepEqual(existing.OwnerReferences, secret.OwnerReferences) {
		existing.OwnerReferences = secret.OwnerReferences
		changed = true
	}
	if changed {
		return r.Update(ctx, &existing)
	}
	return nil
}

func (r *StreamReconciler) finalizeStream(ctx context.Context, stream *domain.Stream) error {
	if !controllerutil.ContainsFinalizer(stream, liveKitIngressFinalizer) {
		return nil
	}
	if err := r.cleanupManagedLiveKitIngress(ctx, stream); err != nil {
		return err
	}
	controllerutil.RemoveFinalizer(stream, liveKitIngressFinalizer)
	return r.Update(ctx, stream)
}

func (r *StreamReconciler) cleanupManagedLiveKitIngress(ctx context.Context, stream *domain.Stream) error {
	if stream.Status.LiveKitIngressID != "" {
		if r.LiveKitIngress == nil {
			return errMissingLiveKitIngressManager
		}
		if err := r.LiveKitIngress.DeleteIngress(ctx, stream.Status.LiveKitIngressID); err != nil {
			return err
		}
	}

	secretName := stream.Status.LiveKitSecretName
	if secretName == "" {
		secretName = liveKitSecretName(stream)
	}
	var secret corev1.Secret
	key := types.NamespacedName{Name: secretName, Namespace: stream.Namespace}
	if err := r.Get(ctx, key, &secret); err != nil {
		return client.IgnoreNotFound(err)
	}
	if err := r.Delete(ctx, &secret); err != nil {
		return client.IgnoreNotFound(err)
	}
	stream.Status.LiveKitIngressID = ""
	stream.Status.LiveKitSecretName = ""
	return nil
}

func (r *StreamReconciler) updateStatus(ctx context.Context, stream *domain.Stream, previous domain.StreamStatus) error {
	next := stream.DeepCopyObject().(*domain.Stream)
	next.Status.ObservedGeneration = stream.Generation
	next.Status.Phase = "Running"
	next.Status.ServiceName = streamWorkloadName(stream)
	next.Status.TakeEndpoint = streamTakeEndpoint(stream)
	next.Status.Error = ""
	if equality.Semantic.DeepEqual(previous, next.Status) {
		return nil
	}
	return r.Status().Update(ctx, next)
}

func (r *StreamReconciler) updateStatusError(ctx context.Context, stream *domain.Stream, err error) error {
	next := stream.DeepCopyObject().(*domain.Stream)
	next.Status.ObservedGeneration = stream.Generation
	next.Status.Phase = "Failed"
	next.Status.ServiceName = streamWorkloadName(stream)
	next.Status.TakeEndpoint = streamTakeEndpoint(stream)
	next.Status.Error = err.Error()
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
		Owns(&corev1.Secret{}).
		Complete(r)
}
