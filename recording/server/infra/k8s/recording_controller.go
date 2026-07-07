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

type RecordingReconciler struct {
	client.Client
	Scheme  *runtime.Scheme
	Options RecordingJobOptions
}

func (r *RecordingReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	var recording domain.Recording
	if err := r.Get(ctx, req.NamespacedName, &recording); err != nil {
		if apierrors.IsNotFound(err) {
			return ctrl.Result{}, nil
		}
		return ctrl.Result{}, err
	}

	stream, err := r.getStream(ctx, &recording)
	if err != nil {
		if apierrors.IsNotFound(err) {
			return ctrl.Result{}, r.updateStatus(ctx, &recording, recordingStatusInput{
				Phase: "Failed",
				Error: "referenced stream was not found",
			})
		}
		return ctrl.Result{}, err
	}

	jobName := recordingJobName(&recording)
	var job batchv1.Job
	err = r.Get(ctx, types.NamespacedName{Name: jobName, Namespace: recording.Namespace}, &job)
	if apierrors.IsNotFound(err) {
		job := BuildRecordingJob(&recording, stream, r.Options)
		if err := controllerutil.SetControllerReference(&recording, job, r.Scheme); err != nil {
			return ctrl.Result{}, err
		}
		if err := r.Create(ctx, job); err != nil {
			return ctrl.Result{}, err
		}
		return ctrl.Result{}, r.updateStatus(ctx, &recording, recordingStatusInput{
			Phase:   "Recording",
			JobName: jobName,
		})
	}
	if err != nil {
		return ctrl.Result{}, err
	}

	if err := r.reconcileStopRequest(ctx, &recording); err != nil {
		return ctrl.Result{}, err
	}
	return ctrl.Result{}, r.updateStatusFromJob(ctx, &recording, &job)
}

func (r *RecordingReconciler) getStream(ctx context.Context, recording *domain.Recording) (*domain.Stream, error) {
	namespace := recording.Spec.StreamRef.Namespace
	if namespace == "" {
		namespace = recording.Namespace
	}
	var stream domain.Stream
	if err := r.Get(ctx, types.NamespacedName{Name: recording.Spec.StreamRef.Name, Namespace: namespace}, &stream); err != nil {
		return nil, err
	}
	return &stream, nil
}

func (r *RecordingReconciler) reconcileStopRequest(ctx context.Context, recording *domain.Recording) error {
	if recording.Spec.StopRequestedAt == nil {
		return nil
	}

	var pods corev1.PodList
	if err := r.List(ctx, &pods, client.InNamespace(recording.Namespace), client.MatchingLabels(recordingLabels(recording))); err != nil {
		return err
	}
	value := recording.Spec.StopRequestedAt.Format("2006-01-02T15:04:05Z07:00")
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

func (r *RecordingReconciler) updateStatusFromJob(ctx context.Context, recording *domain.Recording, job *batchv1.Job) error {
	input := recordingStatusInput{
		Phase:   "Recording",
		JobName: job.Name,
	}
	if job.Status.StartTime != nil {
		input.StartedAt = job.Status.StartTime
	}
	if recording.Spec.StopRequestedAt != nil {
		input.Phase = "StopRequested"
		input.StoppedAt = recording.Spec.StopRequestedAt
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

	return r.updateStatus(ctx, recording, input)
}

type recordingStatusInput struct {
	Phase       string
	JobName     string
	StartedAt   *metav1.Time
	StoppedAt   *metav1.Time
	CompletedAt *metav1.Time
	Error       string
}

func (r *RecordingReconciler) updateStatus(ctx context.Context, recording *domain.Recording, input recordingStatusInput) error {
	next := recording.DeepCopyObject().(*domain.Recording)
	next.Status.ObservedGeneration = recording.Generation
	next.Status.Phase = input.Phase
	next.Status.JobName = input.JobName
	next.Status.StartedAt = input.StartedAt
	next.Status.StoppedAt = input.StoppedAt
	next.Status.CompletedAt = input.CompletedAt
	next.Status.Error = input.Error
	next.Status.S3.Bucket = recording.Spec.Output.S3.Bucket
	next.Status.S3.ObjectKey = recording.Spec.Output.S3.ObjectKey
	if recording.Spec.Output.S3.Bucket != "" && recording.Spec.Output.S3.ObjectKey != "" {
		next.Status.S3.URI = "s3://" + recording.Spec.Output.S3.Bucket + "/" + recording.Spec.Output.S3.ObjectKey
	}
	if input.StartedAt == nil {
		next.Status.StartedAt = recording.Status.StartedAt
	}
	if input.StoppedAt == nil {
		next.Status.StoppedAt = recording.Status.StoppedAt
	}
	if input.CompletedAt == nil {
		next.Status.CompletedAt = recording.Status.CompletedAt
	}
	if equality.Semantic.DeepEqual(recording.Status, next.Status) {
		return nil
	}
	return r.Status().Update(ctx, next)
}

func (r *RecordingReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&domain.Recording{}).
		Owns(&batchv1.Job{}).
		Complete(r)
}
