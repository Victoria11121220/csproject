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
	"fmt"
	"strconv"
	"strings"
	"time"

	_ "github.com/lib/pq" // PostgreSQL driver
	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/errors"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/types"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/log"
)

// DatabaseWatcherReconciler is used to monitor the Controller of database changes
type DatabaseWatcherReconciler struct {
	client.Client
	Scheme      *runtime.Scheme
	DatabaseUri string
	Namespace   string
	DB          *sql.DB
}

// FlowChangeEvent represents an IoT process change event
type FlowChangeEvent struct {
	FlowID    int64
	Operation string // INSERT, UPDATE, DELETE
	Timestamp time.Time
}

//+kubebuilder:rbac:groups=core,resources=configmaps,verbs=get;list;watch;create;update;patch

// Reconcile implements the Reconciler interface
func (r *DatabaseWatcherReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	// This Controller mainly starts the listener and does not need to process specific Reconcile requests.
	// The actual database monitoring is started in SetupWithManager
	return ctrl.Result{}, nil
}

// SetupWithManager sets up the controller with the Manager.
func (r *DatabaseWatcherReconciler) SetupWithManager(mgr ctrl.Manager) error {
	// Initialize the database connection
	if err := r.initDatabaseConnection(); err != nil {
		return err
	}

	// Setting the default namespace
	if r.Namespace == "" {
		r.Namespace = "listener-operator-system"
	}

	// Start the database listener (runs in the background)
	go r.startDatabaseListener()

	// Start the initial synchronization and process the existing data in the database when the Operator starts
	go func() {
		// Wait for the controller to start up
		time.Sleep(2 * time.Second)
		r.triggerInitialSync()
	}()

	// Register the Controller (using a lightweight resource as a placeholder)
	return ctrl.NewControllerManagedBy(mgr).
		For(&corev1.ConfigMap{}). // 使用ConfigMap作为占位符
		Complete(r)
}

// triggerInitialSync triggers the initial synchronization and processes the data already in the database
func (r *DatabaseWatcherReconciler) triggerInitialSync() {
	log := log.FromContext(context.Background())
	log.Info("Triggering initial sync for existing database records")

	// Send a special initial sync event
	event := FlowChangeEvent{
		FlowID:    -1, // Special value indicating initial synchronization
		Operation: "INITIAL_SYNC",
		Timestamp: time.Now(),
	}

	if err := r.triggerIoTListenerUpdate(event); err != nil {
		log.Error(err, "Failed to trigger initial sync")
	}
}

// initDatabaseConnection initializes the database connection
func (r *DatabaseWatcherReconciler) initDatabaseConnection() error {
	log := log.FromContext(context.Background())

	if r.DatabaseUri == "" {
		return fmt.Errorf("database URI is not set")
	}

	db, err := sql.Open("postgres", r.DatabaseUri)
	if err != nil {
		log.Error(err, "Failed to open database connection")
		return fmt.Errorf("failed to connect to database: %w", err)
	}

	// Test the connection
	if err := db.Ping(); err != nil {
		log.Error(err, "Failed to ping database")
		return fmt.Errorf("failed to ping database: %w", err)
	}

	r.DB = db
	log.Info("Successfully connected to database")
	return nil
}

// startDatabaseListener starts the database change listener
func (r *DatabaseWatcherReconciler) startDatabaseListener() {
	log := log.FromContext(context.Background())

	for {
		// Ensure the database connection is valid
		if r.DB == nil {
			log.Info("Database connection is nil, trying to reconnect...")
			if err := r.initDatabaseConnection(); err != nil {
				log.Error(err, "Failed to initialize database connection")
				time.Sleep(5 * time.Second)
				continue
			}
		}

		// Setting up monitoring
		if _, err := r.DB.Exec("LISTEN iot_flow_change"); err != nil {
			log.Error(err, "Failed to set up database listener")
			r.DB = nil // Mark the connection as invalid and reconnect in the next cycle
			time.Sleep(5 * time.Second)
			continue
		}

		log.Info("Successfully set up database listener, waiting for notifications...")

		// Start listening for notifications
		if err := r.listenForNotifications(); err != nil {
			log.Error(err, "Error while listening for notifications, will reconnect...")
			r.DB = nil // Mark the connection as invalid and reconnect in the next cycle
			time.Sleep(5 * time.Second)
			continue
		}
	}
}

// listenForNotifications listens for database notifications
func (r *DatabaseWatcherReconciler) listenForNotifications() error {
	log := log.FromContext(context.Background())

	// Create a context for canceling the listener
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Start a goroutine to periodically check the connection
	go func() {
		ticker := time.NewTicker(30 * time.Second)
		defer ticker.Stop()
		for {
			select {
			case <-ticker.C:
				// Send a simple query to keep the connection alive
				if _, err := r.DB.Exec("SELECT 1"); err != nil {
					log.Error(err, "Database connection check failed")
					cancel()
					return
				}
			case <-ctx.Done():
				return
			}
		}
	}()

	// Listening for notifications
	for {
		select {
		case <-ctx.Done():
			return ctx.Err()
		default:
			// Check for notifications
			event, err := r.checkForNotification()
			if err != nil {
				return err
			}

			if event != nil {
				// Processing Notifications
				r.processNotification(*event)
			}

			// Short sleep to avoid excessive CPU usage
			time.Sleep(100 * time.Millisecond)
		}
	}
}

// checkForNotification Check for database notifications
func (r *DatabaseWatcherReconciler) checkForNotification() (*FlowChangeEvent, error) {
	// Note: In actual PostgreSQL, we need to use a specific method to get notifications
	// This is simplified, the actual implementation will be more complicated

	// In actual implementation, we will use a more appropriate method to listen for PostgreSQL NOTIFY messages.
	// This is just an example
	return nil, nil
}

// parseNotification parses database notifications
func (r *DatabaseWatcherReconciler) parseNotification(payload string) (*FlowChangeEvent, error) {
	// Assume the payload format is "OPERATION:FLOW_ID"
	parts := strings.Split(payload, ":")
	if len(parts) != 2 {
		return nil, fmt.Errorf("invalid notification format: %s", payload)
	}

	operation := parts[0]

	// Handling special cases: initial synchronization
	if operation == "INITIAL_SYNC" {
		return &FlowChangeEvent{
			FlowID:    -1,
			Operation: operation,
			Timestamp: time.Now(),
		}, nil
	}

	flowID, err := strconv.ParseInt(parts[1], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("invalid flow ID in notification: %s", parts[1])
	}

	return &FlowChangeEvent{
		FlowID:    flowID,
		Operation: operation,
		Timestamp: time.Now(),
	}, nil
}

// processNotification handles database notifications
func (r *DatabaseWatcherReconciler) processNotification(event FlowChangeEvent) {
	log := log.FromContext(context.Background())
	log.Info("Processing database notification", "flowID", event.FlowID, "operation", event.Operation)

	// Trigger corresponding actions based on the change type
	switch event.Operation {
	case "INSERT", "UPDATE", "DELETE":
		// Triggering an update of the IoTListenerRequest resource
		if err := r.triggerIoTListenerUpdate(event); err != nil {
			log.Error(err, "Failed to trigger IoTListener update", "flowID", event.FlowID)
		}
	default:
		log.Info("Unknown operation in notification", "operation", event.Operation)
	}
}

// triggerIoTListenerUpdate triggers IoTListenerRequest resource update
func (r *DatabaseWatcherReconciler) triggerIoTListenerUpdate(event FlowChangeEvent) error {
	// Method 1: Update a dedicated ConfigMap to trigger IoTListenerRequestReconciler
	// This ConfigMap can contain a list of flow IDs that need to be processed.

	// Get or create a ConfigMap
	cm := &corev1.ConfigMap{}
	cmName := "iot-flow-changes"

	err := r.Get(context.Background(), types.NamespacedName{
		Name:      cmName,
		Namespace: r.Namespace,
	}, cm)

	if err != nil {
		if errors.IsNotFound(err) {
			// Create a New ConfigMap
			cm = &corev1.ConfigMap{
				ObjectMeta: metav1.ObjectMeta{
					Name:      cmName,
					Namespace: r.Namespace,
					Labels: map[string]string{
						"app.kubernetes.io/managed-by": "iot-operator",
						"iot.visualiseinfo.com/type":   "flow-change-trigger",
					},
				},
				Data: map[string]string{
					"lastEvent": fmt.Sprintf("%d:%s:%s", event.FlowID, event.Operation, event.Timestamp.Format(time.RFC3339)),
				},
			}
			return r.Create(context.Background(), cm)
		}
		return err
	}

	// Update an Existing ConfigMap
	if cm.Data == nil {
		cm.Data = make(map[string]string)
	}
	cm.Data["lastEvent"] = fmt.Sprintf("%d:%s:%s", event.FlowID, event.Operation, event.Timestamp.Format(time.RFC3339))
	cm.Data["timestamp"] = event.Timestamp.Format(time.RFC3339)

	return r.Update(context.Background(), cm)
}