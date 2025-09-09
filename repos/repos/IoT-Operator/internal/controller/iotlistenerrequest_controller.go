/*
Copyright 2024.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUTHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

package controller

import (
	"context"
	"database/sql"
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
	CollectorImageName = "iot-collector"
	ProcessorImageName = "iot-processor"
)

// IoTListenerRequestReconciler reconciles a IoTListenerRequest object
type IoTListenerRequestReconciler struct {
	client.Client
	Scheme *runtime.Scheme
	corev1.PullPolicy
	ImagePullSecret string
	CollectorImage  string
	ProcessorImage  string
	DatabaseUri     string
	ReleaseName     string
	HostAliases     []corev1.HostAlias
	PodAnnotations  map[string]string
	DB              *sql.DB
}

// Function to create a Collector Pod
func (r *IoTListenerRequestReconciler) createCollectorPod(flowID int, nodes, edges string) *corev1.Pod {
	pod := &corev1.Pod{
		ObjectMeta: metav1.ObjectMeta{
			Name:      "iot-collector-" + strconv.Itoa(flowID),
			Namespace: "listener-operator-system",
			Labels: map[string]string{
				"app": "iot-collector",
			},
			Annotations: r.PodAnnotations,
		},
		Spec: corev1.PodSpec{
			ServiceAccountName: "listener-operator-controller-manager",
			HostAliases:      r.HostAliases,
			Containers: []corev1.Container{
				{
					Name:  CollectorImageName,
					Image: r.CollectorImage,
					Env: []corev1.EnvVar{
						{
							Name:  "flow_id",
							Value: strconv.Itoa(flowID),
						},
						{
							Name:  "nodes",
							Value: nodes,
						},
						{
							Name:  "edges",
							Value: edges,
						},
						{
							Name:  "uri",
							Value: r.DatabaseUri,
						},
						{
							Name: "KAFKA_BROKERS",
							ValueFrom: &corev1.EnvVarSource{
								ConfigMapKeyRef: &corev1.ConfigMapKeySelector{
									LocalObjectReference: corev1.LocalObjectReference{
										Name: "iot-env-config",
									},
									Key: "KAFKA_BROKERS",
								},
							},
						},
						{
							Name: "KAFKA_TOPIC",
							ValueFrom: &corev1.EnvVarSource{
								ConfigMapKeyRef: &corev1.ConfigMapKeySelector{
									LocalObjectReference: corev1.LocalObjectReference{
										Name: "iot-env-config",
									},
									Key: "KAFKA_TOPIC",
								},
							},
						},
						{
							Name: "KAFKA_GROUP_ID",
							ValueFrom: &corev1.EnvVarSource{
								ConfigMapKeyRef: &corev1.ConfigMapKeySelector{
									LocalObjectReference: corev1.LocalObjectReference{
										Name: "iot-env-config",
									},
									Key: "KAFKA_GROUP_ID",
								},
							},
						},
						{
							Name: "MQTT_BROKER_HOST",
							ValueFrom: &corev1.EnvVarSource{
								ConfigMapKeyRef: &corev1.ConfigMapKeySelector{
									LocalObjectReference: corev1.LocalObjectReference{
										Name: "mqtt-config",
									},
									Key: "MQTT_BROKER_HOST",
								},
							},
						},
						{
							Name: "MQTT_BROKER_PORT",
							ValueFrom: &corev1.EnvVarSource{
								ConfigMapKeyRef: &corev1.ConfigMapKeySelector{
									LocalObjectReference: corev1.LocalObjectReference{
										Name: "mqtt-config",
									},
									Key: "MQTT_BROKER_PORT",
								},
							},
						},
					},
					ImagePullPolicy: r.PullPolicy,
				},
			},
		},
	}
	if r.ImagePullSecret != "" {
		pod.Spec.ImagePullSecrets = []corev1.LocalObjectReference{
			{
				Name: r.ImagePullSecret,
			},
		}
	}
	return pod
}

// Function to create a Processor Pod
func (r *IoTListenerRequestReconciler) createProcessorPod(flowID int, nodes, edges string) *corev1.Pod {
	return &corev1.Pod{
		ObjectMeta: metav1.ObjectMeta{
			Name:      "iot-processor-" + strconv.Itoa(flowID),
			Namespace: "listener-operator-system",
			Labels: map[string]string{
				"app": "iot-processor",
			},
			Annotations: r.PodAnnotations,
		},
		Spec: corev1.PodSpec{
			ServiceAccountName: "listener-operator-controller-manager",
			HostAliases: r.HostAliases,
			Containers: []corev1.Container{
				{
					Name:  ProcessorImageName,
					Image: r.ProcessorImage,
					Env: []corev1.EnvVar{
						{
							Name:  "flow_id",
							Value: strconv.Itoa(flowID),
						},
						{
							Name:  "nodes",
							Value: nodes,
						},
						{
							Name:  "edges",
							Value: edges,
						},
						{
							Name:  "uri",
							Value: r.DatabaseUri,
						},
						{
							Name: "KAFKA_BROKERS",
							ValueFrom: &corev1.EnvVarSource{
								ConfigMapKeyRef: &corev1.ConfigMapKeySelector{
									LocalObjectReference: corev1.LocalObjectReference{
										Name: "iot-env-config",
									},
									Key: "KAFKA_BROKERS",
								},
							},
						},
						{
							Name: "KAFKA_TOPIC",
							ValueFrom: &corev1.EnvVarSource{
								ConfigMapKeyRef: &corev1.ConfigMapKeySelector{
									LocalObjectReference: corev1.LocalObjectReference{
										Name: "iot-env-config",
									},
									Key: "KAFKA_TOPIC",
								},
							},
						},
						{
							Name: "KAFKA_GROUP_ID",
							ValueFrom: &corev1.EnvVarSource{
								ConfigMapKeyRef: &corev1.ConfigMapKeySelector{
									LocalObjectReference: corev1.LocalObjectReference{
										Name: "iot-env-config",
									},
									Key: "KAFKA_GROUP_ID",
								},
							},
						},
					},
					ImagePullPolicy: r.PullPolicy,
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

//+kubebuilder:rbac:groups=iot.visualiseinfo.com,resources=iotlistenerrequests,verbs=get;list;watch;create;update;patch;delete
//+kubebuilder:rbac:groups=iot.visualiseinfo.com,resources=iotlistenerrequests/status,verbs=get;update;patch
//+kubebuilder:rbac:groups=iot.visualiseinfo.com,resources=iotlistenerrequests/finalizers,verbs=update
//+kubebuilder:rbac:groups="",resources=pods,verbs=get;list;watch;create;update;patch;delete
//+kubebuilder:rbac:groups="",resources=configmaps,verbs=get;list;watch

// Reconcile is part of the main kubernetes reconciliation loop which aims to
// move the current state of the cluster closer to the desired state.
//
// For more details, check Reconcile and its Result here:
// - https://pkg.go.dev/sigs.k8s.io/controller-runtime@v0.17.3/pkg/reconcile
func (r *IoTListenerRequestReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	// This reconcile loop is no longer used.
	// The new logic is in the `listenForNewFlows` function in main.go,
	// which polls the database for new flows and calls StartCollectorAndProcessor.
	return ctrl.Result{}, nil
}

// StartCollectorAndProcessor starts the collector and processor pods for a given flow
func (r *IoTListenerRequestReconciler) StartCollectorAndProcessor(ctx context.Context, flowID int, nodes, edges string) error {
	logger := log.FromContext(ctx)

	// Create collector pod
	collectorPod := r.createCollectorPod(flowID, nodes, edges)
	err := r.Client.Create(ctx, collectorPod)
	if err != nil {
		logger.Error(err, "Failed to create collector pod", "pod", collectorPod.Name)
		return err
	}
	logger.Info("Created collector pod", "pod", collectorPod.Name)

	// Create processor pod
	processorPod := r.createProcessorPod(flowID, nodes, edges)
	err = r.Client.Create(ctx, processorPod)
	if err != nil {
		logger.Error(err, "Failed to create processor pod", "pod", processorPod.Name)
		return err
	}
	logger.Info("Created processor pod", "pod", processorPod.Name)

	return nil
}

// SetupWithManager sets up the controller with the Manager.
func (r *IoTListenerRequestReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&iotv1alpha1.IoTListenerRequest{}).
		Owns(&corev1.Pod{}).
		Complete(r)
}