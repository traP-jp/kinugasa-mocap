package k8s

import (
	"context"

	"github.com/comavius/kinugasa-mocap/recording/server/domain"
	batchv1 "k8s.io/api/batch/v1"
	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/equality"
	apierrors "k8s.io/apimachinery/pkg/api/errors"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/types"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/controller/controllerutil"
)

type TakeReconciler struct {
	client.Client
	Scheme  *runtime.Scheme
	Options TakeJobOptions
}

func (r *TakeReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	var take domain.Take
	if err := r.Get(ctx, req.NamespacedName, &take); err != nil {
		if apierrors.IsNotFound(err) {
			return ctrl.Result{}, nil
		}
		return ctrl.Result{}, err
	}

	stream, err := r.getStream(ctx, &take)
	if err != nil {
		if apierrors.IsNotFound(err) {
			return ctrl.Result{}, r.updateStatus(ctx, &take, takeStatusInput{
				Phase: "Failed",
				Error: "referenced stream was not found",
			})
		}
		return ctrl.Result{}, err
	}

	jobName := takeJobName(&take)
	var job batchv1.Job
	err = r.Get(ctx, types.NamespacedName{Name: jobName, Namespace: take.Namespace}, &job)
	if apierrors.IsNotFound(err) {
		job := BuildTakeJob(&take, stream, r.Options)
		if err := controllerutil.SetControllerReference(&take, job, r.Scheme); err != nil {
			return ctrl.Result{}, err
		}
		if err := r.Create(ctx, job); err != nil {
			return ctrl.Result{}, err
		}
		return ctrl.Result{}, r.updateStatus(ctx, &take, takeStatusInput{
			Phase:   "Capturing",
			JobName: jobName,
		})
	}
	if err != nil {
		return ctrl.Result{}, err
	}

	if err := r.reconcileStopRequest(ctx, &take); err != nil {
		return ctrl.Result{}, err
	}
	return ctrl.Result{}, r.updateStatusFromJob(ctx, &take, &job)
}

func (r *TakeReconciler) getStream(ctx context.Context, take *domain.Take) (*domain.Stream, error) {
	namespace := take.Spec.StreamRef.Namespace
	if namespace == "" {
		namespace = take.Namespace
	}
	var stream domain.Stream
	if err := r.Get(ctx, types.NamespacedName{Name: take.Spec.StreamRef.Name, Namespace: namespace}, &stream); err != nil {
		return nil, err
	}
	return &stream, nil
}

func (r *TakeReconciler) reconcileStopRequest(ctx context.Context, take *domain.Take) error {
	if take.Spec.StopRequestedAt == nil {
		return nil
	}

	var pods corev1.PodList
	if err := r.List(ctx, &pods, client.InNamespace(take.Namespace), client.MatchingLabels(takeLabels(take))); err != nil {
		return err
	}
	value := take.Spec.StopRequestedAt.Format("2006-01-02T15:04:05Z07:00")
	for i := range pods.Items {
		pod := &pods.Items[i]
		if pod.Annotations[stopRequestedAtAnnotation] == value {
			continue
		}
		before := pod.DeepCopy()
		if pod.Annotations == nil {
			pod.Annotations = map[string]string{}
		}
		pod.Annotations[stopRequestedAtAnnotation] = value
		if err := r.Patch(ctx, pod, client.MergeFrom(before)); err != nil {
			return err
		}
	}
	return nil
}

func (r *TakeReconciler) updateStatusFromJob(ctx context.Context, take *domain.Take, job *batchv1.Job) error {
	input := takeStatusInput{
		Phase:   "Capturing",
		JobName: job.Name,
	}
	if job.Status.StartTime != nil {
		input.StartedAt = job.Status.StartTime
	}
	if take.Spec.StopRequestedAt != nil {
		input.Phase = "StopRequested"
		input.StoppedAt = take.Spec.StopRequestedAt
	}

	for _, condition := range job.Status.Conditions {
		switch condition.Type {
		case batchv1.JobComplete:
			if condition.Status == "True" {
				input.Phase = "Completed"
				input.CompletedAt = &condition.LastTransitionTime
				input.Error = ""
			}
		case batchv1.JobFailed:
			if condition.Status == "True" {
				input.Phase = "Failed"
				input.CompletedAt = &condition.LastTransitionTime
				input.Error = condition.Message
				if input.Error == "" {
					input.Error = condition.Reason
				}
			}
		}
	}

	return r.updateStatus(ctx, take, input)
}

type takeStatusInput struct {
	Phase       string
	JobName     string
	StartedAt   *metav1.Time
	StoppedAt   *metav1.Time
	CompletedAt *metav1.Time
	Error       string
}

func (r *TakeReconciler) updateStatus(ctx context.Context, take *domain.Take, input takeStatusInput) error {
	next := take.DeepCopyObject().(*domain.Take)
	next.Status.ObservedGeneration = take.Generation
	next.Status.Phase = input.Phase
	next.Status.JobName = input.JobName
	next.Status.StartedAt = input.StartedAt
	next.Status.StoppedAt = input.StoppedAt
	next.Status.CompletedAt = input.CompletedAt
	next.Status.Error = input.Error
	next.Status.S3.Bucket = take.Spec.Output.S3.Bucket
	next.Status.S3.ObjectKey = take.Spec.Output.S3.ObjectKey
	if take.Spec.Output.S3.Bucket != "" && take.Spec.Output.S3.ObjectKey != "" {
		next.Status.S3.URI = "s3://" + take.Spec.Output.S3.Bucket + "/" + take.Spec.Output.S3.ObjectKey
	}
	if input.StartedAt == nil {
		next.Status.StartedAt = take.Status.StartedAt
	}
	if input.StoppedAt == nil {
		next.Status.StoppedAt = take.Status.StoppedAt
	}
	if input.CompletedAt == nil {
		next.Status.CompletedAt = take.Status.CompletedAt
	}
	if equality.Semantic.DeepEqual(take.Status, next.Status) {
		return nil
	}
	return r.Status().Update(ctx, next)
}

func (r *TakeReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&domain.Take{}).
		Owns(&batchv1.Job{}).
		Complete(r)
}
