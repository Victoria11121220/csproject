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
	"os"
	"sync"

	_ "github.com/lib/pq" // PostgreSQL driver
	appsv1 "k8s.io/api/apps/v1"
	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/errors"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/types"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/handler"
	"sigs.k8s.io/controller-runtime/pkg/log"
	"sigs.k8s.io/controller-runtime/pkg/reconcile"

	iotv1alpha1 "visualiseinfo.com/m/api/v1alpha1"
)

const (
	CollectorImageName = "iot-collector"
	ProcessorImageName = "iot-processor"
	ManagedByLabel     = "app.kubernetes.io/managed-by"
	FlowIdLabel        = "iot.visualiseinfo.com/flow-id"
)

// IoTFlow represents the structure of a flow fetched from the database
type IoTFlow struct {
	ID    int
	Nodes string // JSON string
	Edges string // JSON string
}

// IoTListenerRequestReconciler reconciles a IoTListenerRequest object
type IoTListenerRequestReconciler struct {
	client.Client
	Scheme          *runtime.Scheme
	ImagePullSecret string
	CollectorImage  string
	ProcessorImage  string
	DatabaseUri     string
	KafkaBrokers    string
	KafkaTopic      string
	DB              *sql.DB
	initialized     bool       // Add this field to track whether the initial sync has occurred
	mutex           sync.Mutex // Protecting initialized fields
}

//+kubebuilder:rbac:groups=iot.visualiseinfo.com,resources=iotlistenerrequests,verbs=get;list;watch
//+kubebuilder:rbac:groups=apps,resources=deployments,verbs=get;list;watch;create;update;patch;delete
//+kubebuilder:rbac:groups="",resources=configmaps,verbs=get;list;watch;create;update;patch;delete

// extractSourcesFromNodes parses the nodes JSON and returns a new JSON array of source nodes
func extractSourcesFromNodes(nodesJSON string) (string, error) {
	var nodes []map[string]interface{}
	if err := json.Unmarshal([]byte(nodesJSON), &nodes); err != nil {
		return "", err
	}

	var sourceNodes []map[string]interface{}
	for _, node := range nodes {
		if nodeType, ok := node["type"].(string); ok && nodeType == "source" {
			// The Rust collector expects the 'value' part of the source node.
			if value, ok := node["value"]; ok {
				sourceNodes = append(sourceNodes, value.(map[string]interface{}))
			}
		}
	}

	sourcesJSON, err := json.Marshal(sourceNodes)
	if err != nil {
		return "", err
	}

	return string(sourcesJSON), nil
}

func (r *IoTListenerRequestReconciler) reconcileConfigMap(ctx context.Context, cm *corev1.ConfigMap) error {
	found := &corev1.ConfigMap{}
	err := r.Get(ctx, types.NamespacedName{Name: cm.Name, Namespace: cm.Namespace}, found)
	if err != nil && errors.IsNotFound(err) {
		log.FromContext(ctx).Info("Creating a new ConfigMap", "ConfigMap.Namespace", cm.Namespace, "ConfigMap.Name", cm.Name)
		return r.Create(ctx, cm)
	} else if err != nil {
		return err
	}

	if fmt.Sprintf("%v", found.Data) != fmt.Sprintf("%v", cm.Data) {
		found.Data = cm.Data
		log.FromContext(ctx).Info("Updating ConfigMap", "ConfigMap.Name", cm.Name)
		return r.Update(ctx, found)
	}
	return nil
}

func (r *IoTListenerRequestReconciler) reconcileDeployment(ctx context.Context, depl *appsv1.Deployment) error {
	found := &appsv1.Deployment{}
	err := r.Get(ctx, types.NamespacedName{Name: depl.Name, Namespace: depl.Namespace}, found)
	if err != nil && errors.IsNotFound(err) {
		log.FromContext(ctx).Info("Creating a new Deployment", "Deployment.Namespace", depl.Namespace, "Deployment.Name", depl.Name)
		return r.Create(ctx, depl)
	} else if err != nil {
		return err
	}
	// Naive update, a real implementation should be smarter
	// Only update if the image is different to avoid endless update loops
	if found.Spec.Template.Spec.Containers[0].Image != depl.Spec.Template.Spec.Containers[0].Image {
		found.Spec = depl.Spec
		log.FromContext(ctx).Info("Updating Deployment", "Deployment.Name", depl.Name)
		return r.Update(ctx, found)
	}
	return nil
}

func (r *IoTListenerRequestReconciler) cleanupOrphanedResources(ctx context.Context, dbFlows map[string]IoTFlow) error {
	log := log.FromContext(ctx)
	// List all deployments managed by this operator
	deplList := &appsv1.DeploymentList{}
	if err := r.List(ctx, deplList, client.MatchingLabels{ManagedByLabel: "iot-operator"}); err != nil {
		return err
	}

	for _, depl := range deplList.Items {
		if flowID, ok := depl.Labels[FlowIdLabel]; ok {
			if _, existsInDb := dbFlows[flowID]; !existsInDb {
				log.Info("Deleting orphaned deployment", "deployment", depl.Name)
				if err := r.Delete(ctx, &depl); err != nil {
					log.Error(err, "Failed to delete orphaned deployment", "deployment", depl.Name)
				}
			}
		}
	}

	// Cleanup ConfigMaps similarly
	cmList := &corev1.ConfigMapList{}
	if err := r.List(ctx, cmList, client.MatchingLabels{ManagedByLabel: "iot-operator"}); err != nil {
		return err
	}
	for _, cm := range cmList.Items {
		if flowID, ok := cm.Labels[FlowIdLabel]; ok {
			if _, existsInDb := dbFlows[flowID]; !existsInDb {
				log.Info("Deleting orphaned configmap", "configmap", cm.Name)
				if err := r.Delete(ctx, &cm); err != nil {
					log.Error(err, "Failed to delete orphaned configmap", "configmap", cm.Name)
				}
			}
		}
	}

	return nil
}

// --- Resource Definitions ---

func (r *IoTListenerRequestReconciler) defineCollectorConfigMap(flowID string, sourcesConfig string, nodes string, edges string) *corev1.ConfigMap {
	name := fmt.Sprintf("collector-config-%s", flowID)
	return &corev1.ConfigMap{
		ObjectMeta: metav1.ObjectMeta{
			Name:      name,
			Namespace: r.getNamespace(), // Or from a config
			Labels:    map[string]string{ManagedByLabel: "iot-operator", FlowIdLabel: flowID},
		},
		Data: map[string]string{
			"SOURCES": sourcesConfig,
			"nodes":   nodes,
			"edges":   edges,
		},
	}
}

// Define labels for the ConfigMap
func (r *IoTListenerRequestReconciler) defineProcessorConfigMap(flowID string, nodes string, edges string) *corev1.ConfigMap {
	name := fmt.Sprintf("processor-config-%s", flowID)
	return &corev1.ConfigMap{
		ObjectMeta: metav1.ObjectMeta{
			Name:      name,
			Namespace: r.getNamespace(),
			Labels:    map[string]string{ManagedByLabel: "iot-operator", FlowIdLabel: flowID},
		},
		Data: map[string]string{
			"nodes": nodes,
			"edges": edges,
		},
	}
}

func (r *IoTListenerRequestReconciler) defineCollectorDeployment(flowID string, configMapName string) *appsv1.Deployment {
	name := fmt.Sprintf("iot-collector-%s", flowID)
	replicas := int32(1)
	return &appsv1.Deployment{
		ObjectMeta: metav1.ObjectMeta{
			Name:      name,
			Namespace: r.getNamespace(),
			Labels:    map[string]string{ManagedByLabel: "iot-operator", FlowIdLabel: flowID},
		},
		Spec: appsv1.DeploymentSpec{
			Replicas: &replicas,
			Selector: &metav1.LabelSelector{MatchLabels: map[string]string{"app": name}},
			Template: corev1.PodTemplateSpec{
				ObjectMeta: metav1.ObjectMeta{Labels: map[string]string{"app": name}},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{{
						Name:    CollectorImageName,
						Image:   r.CollectorImage,
						EnvFrom: []corev1.EnvFromSource{{ConfigMapRef: &corev1.ConfigMapEnvSource{LocalObjectReference: corev1.LocalObjectReference{Name: configMapName}}}},
						Env: []corev1.EnvVar{
							{Name: "KAFKA_BOOTSTRAP_SERVERS", Value: r.KafkaBrokers},
							{Name: "KAFKA_PROCESSOR_TOPIC", Value: r.KafkaTopic},
							{Name: "uri", Value: r.DatabaseUri},
							{Name: "flow_id", Value: flowID},
						},
					}},
				},
			},
		},
	}
}

func (r *IoTListenerRequestReconciler) defineProcessorDeployment(flowID string, configMapName string) *appsv1.Deployment {
	name := fmt.Sprintf("iot-processor-%s", flowID)
	replicas := int32(1)
	return &appsv1.Deployment{
		ObjectMeta: metav1.ObjectMeta{
			Name:      name,
			Namespace: r.getNamespace(),
			Labels:    map[string]string{ManagedByLabel: "iot-operator", FlowIdLabel: flowID},
		},
		Spec: appsv1.DeploymentSpec{
			Replicas: &replicas,
			Selector: &metav1.LabelSelector{MatchLabels: map[string]string{"app": name}},
			Template: corev1.PodTemplateSpec{
				ObjectMeta: metav1.ObjectMeta{Labels: map[string]string{"app": name}},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{{
						Name:            ProcessorImageName,
						Image:           r.ProcessorImage,
						ImagePullPolicy: corev1.PullIfNotPresent,
						EnvFrom:         []corev1.EnvFromSource{{ConfigMapRef: &corev1.ConfigMapEnvSource{LocalObjectReference: corev1.LocalObjectReference{Name: configMapName}}}},
						Env: []corev1.EnvVar{
							{Name: "KAFKA_BOOTSTRAP_SERVERS", Value: r.KafkaBrokers},
							{Name: "KAFKA_PROCESSOR_TOPIC", Value: r.KafkaTopic},
							{Name: "KAFKA_PROCESSOR_GROUP_ID", Value: "iot-processor-group-" + flowID},
							{Name: "uri", Value: r.DatabaseUri},
							{Name: "flow_id", Value: flowID},
						},
					}},
				},
			},
		},
	}
}

func (r *IoTListenerRequestReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	log := log.FromContext(ctx)

	// Check if the request is from our trigger ConfigMap
	if req.NamespacedName.Name == "iot-flow-changes" {
		// Get event information from ConfigMap
		cm := &corev1.ConfigMap{}
		if err := r.Get(ctx, req.NamespacedName, cm); err != nil {
			if !errors.IsNotFound(err) {
				log.Error(err, "Failed to get trigger ConfigMap")
				return ctrl.Result{}, err
			}
			// ConfigMap does not exist, it may have been deleted, no processing
			return ctrl.Result{}, nil
		}

		// Check if it is the initial synchronization event
		if cm.Data != nil {
			if lastEvent, ok := cm.Data["lastEvent"]; ok && len(lastEvent) > 0 {
				log.Info("Processing event", "event", lastEvent)
				// Check whether it is an initial synchronization event
				if lastEvent == "-1:INITIAL_SYNC:" {
					log.Info("Processing initial sync")
					return r.reconcileInitialFlows(ctx)
				}
			}
		}
	}

	// Query database for all flows
	rows, err := r.DB.QueryContext(ctx, "SELECT id, nodes, edges FROM iot_flow")
	if err != nil {
		log.Error(err, "Failed to query iot_flow table")
		return ctrl.Result{}, err
	}
	defer rows.Close()

	dbFlows := make(map[string]IoTFlow)
	for rows.Next() {
		var flow IoTFlow
		if err := rows.Scan(&flow.ID, &flow.Nodes, &flow.Edges); err != nil {
			log.Error(err, "Failed to scan flow row")
			continue
		}
		flowIDStr := fmt.Sprintf("%d", flow.ID)
		dbFlows[flowIDStr] = flow
	}

	log.Info(fmt.Sprintf("Found %d flows in database", len(dbFlows)))

	// --- Reconciliation Logic ---
	for flowID, flow := range dbFlows {
		log.Info("Reconciling flow", "flowID", flowID)

		sourcesConfig, err := extractSourcesFromNodes(flow.Nodes)
		if err != nil {
			log.Error(err, "Failed to extract sources from nodes JSON", "flowID", flowID)
			continue
		}

		// Reconcile ConfigMap for Collector
		cmCollector := r.defineCollectorConfigMap(flowID, sourcesConfig, flow.Nodes, flow.Edges)
		if err := r.reconcileConfigMap(ctx, cmCollector); err != nil {
			log.Error(err, "Failed to reconcile collector configmap", "flowID", flowID)
			continue
		}

		// Reconcile ConfigMap for Processor
		cmProcessor := r.defineProcessorConfigMap(flowID, flow.Nodes, flow.Edges)
		if err := r.reconcileConfigMap(ctx, cmProcessor); err != nil {
			log.Error(err, "Failed to reconcile processor configmap", "flowID", flowID)
			continue
		}

		// Reconcile Deployment for Collector
		deplCollector := r.defineCollectorDeployment(flowID, cmCollector.Name)
		if err := r.reconcileDeployment(ctx, deplCollector); err != nil {
			log.Error(err, "Failed to reconcile collector deployment", "flowID", flowID)
			continue
		}

		// Reconcile Deployment for Processor
		deplProcessor := r.defineProcessorDeployment(flowID, cmProcessor.Name)
		if err := r.reconcileDeployment(ctx, deplProcessor); err != nil {
			log.Error(err, "Failed to reconcile processor deployment", "flowID", flowID)
			continue
		}
	}

	// --- Cleanup Logic ---
	if err := r.cleanupOrphanedResources(ctx, dbFlows); err != nil {
		log.Error(err, "Failed to cleanup orphaned resources")
	}

	// With the new subscription-based approach, we don't need to requeue periodically
	return ctrl.Result{}, nil
}

// reconcileInitialFlows handles the initial synchronization and only processes the first 5 records
func (r *IoTListenerRequestReconciler) reconcileInitialFlows(ctx context.Context) (ctrl.Result, error) {
	log := log.FromContext(ctx)
	log.Info("Performing initial sync, processing first 5 flows")

	// Query the first 5 records
	rows, err := r.DB.QueryContext(ctx, "SELECT id, nodes, edges FROM iot_flow ORDER BY id LIMIT 5")
	if err != nil {
		log.Error(err, "Failed to query iot_flow table for initial sync")
		return ctrl.Result{}, err
	}
	defer rows.Close()

	dbFlows := make(map[string]IoTFlow)
	for rows.Next() {
		var flow IoTFlow
		if err := rows.Scan(&flow.ID, &flow.Nodes, &flow.Edges); err != nil {
			log.Error(err, "Failed to scan flow row during initial sync")
			continue
		}
		flowIDStr := fmt.Sprintf("%d", flow.ID)
		dbFlows[flowIDStr] = flow
	}

	log.Info(fmt.Sprintf("Found %d flows for initial sync", len(dbFlows)))

	// --- Reconciliation Logic for initial flows ---
	for flowID, flow := range dbFlows {
		log.Info("Reconciling flow during initial sync", "flowID", flowID)

		sourcesConfig, err := extractSourcesFromNodes(flow.Nodes)
		if err != nil {
			log.Error(err, "Failed to extract sources from nodes JSON during initial sync", "flowID", flowID)
			continue
		}

		// Reconcile ConfigMap for Collector
		cmCollector := r.defineCollectorConfigMap(flowID, sourcesConfig, flow.Nodes, flow.Edges)
		if err := r.reconcileConfigMap(ctx, cmCollector); err != nil {
			log.Error(err, "Failed to reconcile collector configmap during initial sync", "flowID", flowID)
			continue
		}

		// Reconcile ConfigMap for Processor
		cmProcessor := r.defineProcessorConfigMap(flowID, flow.Nodes, flow.Edges)
		if err := r.reconcileConfigMap(ctx, cmProcessor); err != nil {
			log.Error(err, "Failed to reconcile processor configmap during initial sync", "flowID", flowID)
			continue
		}

		// Reconcile Deployment for Collector
		deplCollector := r.defineCollectorDeployment(flowID, cmCollector.Name)
		if err := r.reconcileDeployment(ctx, deplCollector); err != nil {
			log.Error(err, "Failed to reconcile collector deployment during initial sync", "flowID", flowID)
			continue
		}

		// Reconcile Deployment for Processor
		deplProcessor := r.defineProcessorDeployment(flowID, cmProcessor.Name)
		if err := r.reconcileDeployment(ctx, deplProcessor); err != nil {
			log.Error(err, "Failed to reconcile processor deployment during initial sync", "flowID", flowID)
			continue
		}
	}

	// --- Cleanup Logic ---
	if err := r.cleanupOrphanedResources(ctx, dbFlows); err != nil {
		log.Error(err, "Failed to cleanup orphaned resources during initial sync")
	}

	return ctrl.Result{}, nil
}

// getNamespace gets the namespace, preferably from the environment variable, otherwise use "listener-operator-system"
func (r *IoTListenerRequestReconciler) getNamespace() string {
	if namespace := os.Getenv("NAMESPACE"); namespace != "" {
		return namespace
	}
	return "listener-operator-system"
}

// SetupWithManager sets up the controller with the Manager.
func (r *IoTListenerRequestReconciler) SetupWithManager(mgr ctrl.Manager) error {
	// Initialize database connection
	if r.DB == nil {
		log := log.FromContext(context.Background())
		log.Info("Initializing database connection")
		db, err := sql.Open("postgres", r.DatabaseUri)
		if err != nil {
			log.Error(err, "Failed to connect to database")
			return err
		}
		r.DB = db
	}

	return ctrl.NewControllerManagedBy(mgr).
		For(&iotv1alpha1.IoTListenerRequest{}).
		Owns(&appsv1.Deployment{}).
		Owns(&corev1.ConfigMap{}).
		Watches(&corev1.ConfigMap{}, handler.EnqueueRequestsFromMapFunc(r.configMapToIoTListenerRequest)).
		Complete(r)
}

// configMapToIoTListenerRequest maps ConfigMap updates to IoTListenerRequest reconcile requests
func (r *IoTListenerRequestReconciler) configMapToIoTListenerRequest(ctx context.Context, obj client.Object) []reconcile.Request {
	// Only process our specific trigger ConfigMap
	if labels := obj.GetLabels(); labels != nil {
		if labels["iot.visualiseinfo.com/type"] == "flow-change-trigger" {
			// When the trigger ConfigMap is updated, reconcile all IoTListenerRequest resources
			// In a more sophisticated implementation, we could parse the ConfigMap to determine
			// which specific flows need to be reconciled

			// List all IoTListenerRequest resources
			list := &iotv1alpha1.IoTListenerRequestList{}
			if err := r.List(ctx, list); err != nil {
				return nil
			}

			// Create reconcile requests for each IoTListenerRequest
			requests := make([]reconcile.Request, len(list.Items))
			for i, item := range list.Items {
				requests[i] = reconcile.Request{
					NamespacedName: types.NamespacedName{
						Name:      item.Name,
						Namespace: item.Namespace,
					},
				}
			}

			return requests
		}
	}

	// For other ConfigMaps, no action is triggered
	return nil
}