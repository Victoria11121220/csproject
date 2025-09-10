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
	"database/sql"
	"encoding/json"
	"fmt"

	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/log"

	iotv1alpha1 "visualiseinfo.com/m/api/v1alpha1"

	// Kafka client
	kafka "github.com/segmentio/kafka-go"
)

const (
	CollectorImageName = "iot-collector"
	ProcessorImageName = "iot-processor"

	// Fixed pod names for singleton collector and processor
	CollectorPodName = "iot-collector"
	ProcessorPodName = "iot-processor"
)

// FlowUpdateMessage represents a message sent to Kafka when a flow is updated
type FlowUpdateMessage struct {
	FlowID int    `json:"flow_id"`
	Nodes  string `json:"nodes"`
	Edges  string `json:"edges"`
}

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
	KafkaProducer   *kafka.Writer
	KafkaTopic      string
}

// Function to create a Collector Pod (singleton)
func (r *IoTListenerRequestReconciler) createCollectorPod() *corev1.Pod {
	pod := &corev1.Pod{
		ObjectMeta: metav1.ObjectMeta{
			Name:      CollectorPodName,
			Namespace: "listener-operator-system",
			Labels: map[string]string{
				"app": "iot-collector",
			},
			Annotations: r.PodAnnotations,
		},
		Spec: corev1.PodSpec{
			ServiceAccountName: "listener-operator-controller-manager",
			HostAliases:        r.HostAliases,
			Containers: []corev1.Container{
				{
					Name:  CollectorImageName,
					Image: r.CollectorImage,
					Env: []corev1.EnvVar{
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
							Name: "FLOW_UPDATES_TOPIC",
							ValueFrom: &corev1.EnvVarSource{
								ConfigMapKeyRef: &corev1.ConfigMapKeySelector{
									LocalObjectReference: corev1.LocalObjectReference{
										Name: "iot-env-config",
									},
									Key: "FLOW_UPDATES_TOPIC",
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

// Function to create a Processor Pod (singleton)
func (r *IoTListenerRequestReconciler) createProcessorPod() *corev1.Pod {
	return &corev1.Pod{
		ObjectMeta: metav1.ObjectMeta{
			Name:      ProcessorPodName,
			Namespace: "listener-operator-system",
			Labels: map[string]string{
				"app": "iot-processor",
			},
			Annotations: r.PodAnnotations,
		},
		Spec: corev1.PodSpec{
			ServiceAccountName: "listener-operator-controller-manager",
			HostAliases:        r.HostAliases,
			Containers: []corev1.Container{
				{
					Name:  ProcessorImageName,
					Image: r.ProcessorImage,
					Env: []corev1.EnvVar{
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
							Name: "FLOW_UPDATES_TOPIC",
							ValueFrom: &corev1.EnvVarSource{
								ConfigMapKeyRef: &corev1.ConfigMapKeySelector{
									LocalObjectReference: corev1.LocalObjectReference{
										Name: "iot-env-config",
									},
									Key: "FLOW_UPDATES_TOPIC",
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

// checkIfPodExists checks if a pod with the given name exists in the namespace and is healthy
func (r *IoTListenerRequestReconciler) checkIfPodExists(ctx context.Context, podName string) (bool, error) {
	pod := &corev1.Pod{}
	err := r.Client.Get(ctx, client.ObjectKey{
		Namespace: "listener-operator-system",
		Name:      podName,
	}, pod)

	if err != nil {
		// If the error is "not found", the pod doesn't exist
		if client.IgnoreNotFound(err) == nil {
			return false, nil
		}
		// For any other error, return the error
		return false, err
	}

	// Pod exists, check if it's in a healthy state
	for _, condition := range pod.Status.Conditions {
		if condition.Type == corev1.PodReady {
			if condition.Status == corev1.ConditionTrue {
				return true, nil
			}
			break
		}
	}

	// Also check container status
	for _, containerStatus := range pod.Status.ContainerStatuses {
		if containerStatus.RestartCount > 5 && containerStatus.LastTerminationState.Terminated != nil {
			// If container has restarted many times and last state was terminated, consider it unhealthy
			return false, nil
		}
	}

	// Pod exists and appears healthy
	return true, nil
}

// sendFlowUpdateToKafka sends a flow update message to Kafka
func (r *IoTListenerRequestReconciler) sendFlowUpdateToKafka(flowID int, nodes, edges string) error {
	// Create the flow update message
	message := FlowUpdateMessage{
		FlowID: flowID,
		Nodes:  nodes,
		Edges:  edges,
	}

	// Serialize the message to JSON
	payload, err := json.Marshal(message)
	if err != nil {
		return err
	}

	// Send the message to Kafka
	topic := r.KafkaTopic
	if topic == "" {
		topic = "flow-updates" // Default topic name
	}

	err = r.KafkaProducer.WriteMessages(context.Background(), kafka.Message{
		Topic: topic,
		Value: payload,
	})

	return err
}

// StartCollectorAndProcessor starts the collector and processor pods (singleton)
// If the pods already exist, it sends an update message via Kafka
func (r *IoTListenerRequestReconciler) StartCollectorAndProcessor(ctx context.Context, flowID int, nodes, edges string) error {
	logger := log.FromContext(ctx)

	// Check if collector pod already exists and is healthy
	collectorExists, err := r.checkIfPodExists(ctx, CollectorPodName)
	if err != nil {
		logger.Error(err, "Failed to check if collector pod exists", "pod", CollectorPodName)
		return err
	}

	// Check if processor pod already exists and is healthy
	processorExists, err := r.checkIfPodExists(ctx, ProcessorPodName)
	if err != nil {
		logger.Error(err, "Failed to check if processor pod exists", "pod", ProcessorPodName)
		return err
	}

	// If both pods exist and are healthy, send update via Kafka
	if collectorExists && processorExists {
		logger.Info("Both collector and processor pods already exist and are healthy, sending update via Kafka")
		msg := fmt.Sprintf("flowID: %d; Nodes: %s; Edges: %s", flowID, nodes, edges)
		logger.Info(msg)

		err := r.sendFlowUpdateToKafka(flowID, nodes, edges)
		if err != nil {
			logger.Error(err, "Failed to send flow update to Kafka", "flowID", flowID)
			return err
		}
		logger.Info("Successfully sent flow update to Kafka", "flowID", flowID)
		return nil
	}

	// If collector pod doesn't exist, create it
	if !collectorExists {
		// Create new collector pod
		collectorPod := r.createCollectorPod()
		err = r.Client.Create(ctx, collectorPod)
		if err != nil {
			logger.Error(err, "Failed to create collector pod", "pod", collectorPod.Name)
			return err
		}
		logger.Info("Created collector pod", "pod", collectorPod.Name)
	}

	// If processor pod doesn't exist, create it
	if !processorExists {
		// Create new processor pod
		processorPod := r.createProcessorPod()
		err = r.Client.Create(ctx, processorPod)
		if err != nil {
			logger.Error(err, "Failed to create processor pod", "pod", processorPod.Name)
			return err
		}
		logger.Info("Created processor pod", "pod", processorPod.Name)
	}

	return nil
}

// SetupWithManager sets up the controller with the Manager.
func (r *IoTListenerRequestReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&iotv1alpha1.IoTListenerRequest{}).
		Owns(&corev1.Pod{}).
		Complete(r)
}