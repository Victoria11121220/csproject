/*
Copyright 2024.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

package controller

import (
	"context"
	"strconv"

	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/log"

	iotv1alpha1 "visualiseinfo.com/m/api/v1alpha1"
)

const (
	ListenerImageName = "iot-listener"
)

// Function to create a Pod given a custom resource
func (r *IoTListenerRequestReconciler) createPod(cr iotv1alpha1.IoTListenerRequest) *corev1.Pod {

	return &corev1.Pod{
		ObjectMeta: metav1.ObjectMeta{
			Name:      cr.Name,
			Namespace: cr.Namespace,
			Labels: map[string]string{
				"app": ListenerImageName,
			},
			OwnerReferences: []metav1.OwnerReference{
				*metav1.NewControllerRef(&cr, iotv1alpha1.GroupVersion.WithKind("IoTListenerRequest")),
			},
			Annotations: r.PodAnnotations,
		},
		Spec: corev1.PodSpec{
			HostAliases: r.HostAliases,
			Containers: []corev1.Container{
				{
					Name:  ListenerImageName,
					Image: r.ListenerImage,
					Env: []corev1.EnvVar{
						{
							Name:  "flow_id",
							Value: strconv.Itoa(int(cr.Spec.FlowID)),
						},
						{
							Name:  "nodes",
							Value: cr.Spec.Nodes,
						},
						{
							Name:  "edges",
							Value: cr.Spec.Edges,
						},
						{
							Name:  "uri",
							Value: r.DatabaseUri,
						},
						{
							Name:  "ROCKET_ADDRESS",
							Value: "0.0.0.0",
						},
					},
					ImagePullPolicy: corev1.PullIfNotPresent,
					Ports: []corev1.ContainerPort{
						{
							ContainerPort: 8000,
							Name:          "metrics",
						},
					},
				},
			},
			ImagePullSecrets: []corev1.LocalObjectReference{
				{
					Name: r.ImagePullSecret,
				},
			},
		},
	}
}

// IoTListenerRequestReconciler reconciles a IoTListenerRequest object
type IoTListenerRequestReconciler struct {
	client.Client
	Scheme *runtime.Scheme
	corev1.PullPolicy
	ImagePullSecret string
	ListenerImage   string
	DatabaseUri     string
	ReleaseName     string
	HostAliases     []corev1.HostAlias
	PodAnnotations  map[string]string
}

//+kubebuilder:rbac:groups=iot.visualiseinfo.com,resources=iotlistenerrequests,verbs=get;list;watch;create;update;patch;delete
//+kubebuilder:rbac:groups=iot.visualiseinfo.com,resources=iotlistenerrequests/status,verbs=get;update;patch
//+kubebuilder:rbac:groups=iot.visualiseinfo.com,resources=iotlistenerrequests/finalizers,verbs=update

//+kubebuilder:rbac:groups="",resources=pods,verbs=get;list;watch;create;update;patch;delete

// 如果你的 Operator 还需要创建 Service 或 Deployment，也需要相应的权限
//+kubebuilder:rbac:groups=apps,resources=deployments,verbs=get;list;watch;create;update;patch;delete
//+kubebuilder:rbac:groups="",resources=services,verbs=get;list;watch;create;update;patch;delete

// Reconcile is part of the main kubernetes reconciliation loop which aims to
// move the current state of the cluster closer to the desired state.
//
// For more details, check Reconcile and its Result here:
// - https://pkg.go.dev/sigs.k8s.io/controller-runtime@v0.17.3/pkg/reconcile
func (r *IoTListenerRequestReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	_ = log.FromContext(ctx)

	// Fetch all custom resources
	listenerRequests := &iotv1alpha1.IoTListenerRequestList{}
	err := r.Client.List(ctx, listenerRequests)
	if err != nil {
		log.FromContext(ctx).Error(err, "Failed to list IoTListenerRequests")
	}

	// Fetch all pods with the label "app=iot-listener"
	podList := &corev1.PodList{}
	err = r.Client.List(ctx, podList, client.MatchingLabels{"app": ListenerImageName})
	if err != nil {
		log.FromContext(ctx).Error(err, "Failed to list pods")
	}

	// Iterate over the custom resources
	for _, listenerRequest := range listenerRequests.Items {
		// Check if pod exists
		foundPod := &corev1.Pod{}
		err = r.Client.Get(ctx, client.ObjectKey{Namespace: listenerRequest.Namespace, Name: listenerRequest.Name}, foundPod)
		if err != nil {
			if client.IgnoreNotFound(err) == nil {
				// Create the pod
				foundPod = r.createPod(listenerRequest)
				err = r.Client.Create(ctx, foundPod)
				log.FromContext(ctx).Info("Creating pod", "pod", foundPod.Name)
				if err != nil {
					log.FromContext(ctx).Error(err, "Failed to create pod", "pod", foundPod.Name)
				}
			} else {
				log.FromContext(ctx).Error(err, "Failed to get pod", "pod", listenerRequest.Name)
			}
		}
	}

	return ctrl.Result{}, nil

}

// SetupWithManager sets up the controller with the Manager.
func (r *IoTListenerRequestReconciler) SetupWithManager(mgr ctrl.Manager) error {

	return ctrl.NewControllerManagedBy(mgr).
		For(&iotv1alpha1.IoTListenerRequest{}).
		Owns(&corev1.Pod{}).
		Complete(r)
}