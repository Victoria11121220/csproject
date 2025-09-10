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

package main

import (
	"context"
	"crypto/tls"
	"database/sql"
	"encoding/json"
	"flag"
	"fmt"
	"strings"

	"os"
	"time"

	// Import all Kubernetes client auth plugins (e.g. Azure, GCP, OIDC, etc.)
	// to ensure that exec-entrypoint and run can make use of them.

	"golang.org/x/exp/rand"
	_ "k8s.io/client-go/plugin/pkg/client/auth"

	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/runtime"
	utilruntime "k8s.io/apimachinery/pkg/util/runtime"
	clientgoscheme "k8s.io/client-go/kubernetes/scheme"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/healthz"
	"sigs.k8s.io/controller-runtime/pkg/log/zap"
	metricsserver "sigs.k8s.io/controller-runtime/pkg/metrics/server"
	"sigs.k8s.io/controller-runtime/pkg/webhook"

	// PostgreSQL driver
	"github.com/lib/pq"
	_ "github.com/lib/pq"

	// Kafka client
	kafka "github.com/segmentio/kafka-go"

	iotv1alpha1 "visualiseinfo.com/m/api/v1alpha1"
	"visualiseinfo.com/m/internal/controller"
	//+kubebuilder:scaffold:imports
)

var (
	scheme   = runtime.NewScheme()
	setupLog = ctrl.Log.WithName("setup")
)

func init() {

	rand.Seed(uint64(time.Now().UnixNano()))

	utilruntime.Must(clientgoscheme.AddToScheme(scheme))

	utilruntime.Must(iotv1alpha1.AddToScheme(scheme))
	//+kubebuilder:scaffold:scheme
}

type Config struct {
	CollectorImage  string
	ProcessorImage  string
	PullPolicy      corev1.PullPolicy
	ImagePullSecret string
	ReleaseName     string
	HostAliases     []corev1.HostAlias
	Annotations     map[string]string
}

// ReadConfig reads the configuration from the mounted config files
// Returns a config struct or exits the program if an error occurs
func ReadConfig() Config {
	var config Config

	// Read collector image config
	collector_image, err := os.ReadFile("./etc/config/COLLECTOR_IMAGE")
	if err != nil {
		setupLog.Error(err, "unable to read COLLECTOR_IMAGE")
		os.Exit(1)
	}
	config.CollectorImage = string(collector_image)

	// Read processor image config
	processor_image, err := os.ReadFile("./etc/config/PROCESSOR_IMAGE")
	if err != nil {
		setupLog.Error(err, "unable to read PROCESSOR_IMAGE")
		os.Exit(1)
	}
	config.ProcessorImage = string(processor_image)

	pull_policy, err := os.ReadFile("./etc/config/IMAGE_PULL_POLICY")
	if err != nil {
		setupLog.Error(err, "unable to read IMAGE_PULL_POLICY")
		os.Exit(1)
	}
	config.PullPolicy = corev1.PullPolicy(string(pull_policy))

	image_pull_secret, err := os.ReadFile("./etc/config/IMAGE_PULL_SECRET")
	if err != nil {
		setupLog.Error(err, "unable to read IMAGE_PULL_SECRET")
		os.Exit(1)
	}
	config.ImagePullSecret = string(image_pull_secret)

	release_name, err := os.ReadFile("./etc/config/RELEASE_NAME")
	if err != nil {
		setupLog.Error(err, "unable to read RELEASE_NAME")
		os.Exit(1)
	}
	config.ReleaseName = string(release_name)

	host_aliases_str, err := os.ReadFile("./etc/config/HOST_ALIASES")
	if err != nil {
		setupLog.Error(err, "unable to read HOST_ALIASES")
	}
	// Deserialize the host aliases into corev1.HostAliases
	var hostAliases []corev1.HostAlias
	if host_aliases_str != nil {
		err = json.Unmarshal(host_aliases_str, &hostAliases)
		if err != nil {
			// Do not return here, as we can still run the program without host aliases
			setupLog.Error(err, "unable to deserialize HOST_ALIASES")
		} else {
			setupLog.Info("successfully deserialized HOST_ALIASES", "hostAliases", hostAliases)
		}
	}
	config.HostAliases = hostAliases

	annotations_str, err := os.ReadFile("./etc/config/POD_ANNOTATIONS")
	var annotations map[string]string
	if err != nil {
		setupLog.Error(err, "unable to read POD_ANNOTATIONS")
	} else if annotations_str != nil {
		err = json.Unmarshal(annotations_str, &annotations)
		if err != nil {
			setupLog.Error(err, "unable to deserialise annotations object")
		} else {
			setupLog.Info("successfully deserialized POD_ANNOTATIONS", "Annotations", annotations)
		}
	}
	config.Annotations = annotations

	return config
}

// createKafkaProducer creates a Kafka producer using the configuration from the iot-env-config ConfigMap
func createKafkaProducer() (*kafka.Writer, error) {
	// Get Kafka brokers from environment or config
	brokers := os.Getenv("KAFKA_BROKERS")
	if brokers == "" {
		// Try to read from config file as fallback
		brokersBytes, err := os.ReadFile("./etc/config/KAFKA_BROKERS")
		if err != nil {
			return nil, fmt.Errorf("unable to read KAFKA_BROKERS from environment or file: %v", err)
		}
		brokers = strings.TrimSpace(string(brokersBytes))
	}

	// Validate that we have brokers
	if brokers == "" {
		return nil, fmt.Errorf("KAFKA_BROKERS is not set in environment or config file")
	}

	// Create Kafka writer (producer)
	writer := kafka.Writer{
		Addr:         kafka.TCP(brokers),
		Balancer:     &kafka.LeastBytes{},
		RequiredAcks: kafka.RequireAll,
	}

	return &writer, nil
}

// listenForNewFlows listens for new flows in the iot_flow table using PostgreSQL LISTEN/NOTIFY
func listenForNewFlows(ctx context.Context, db *sql.DB, postgresURI string, reconciler *controller.IoTListenerRequestReconciler) {
	setupLog.Info("Starting to listen for new flows in iot_flow table...")

	// Create a dedicated listener connection for PostgreSQL NOTIFY
	// The minimum and maximum reconnect intervals are set to reasonable defaults
	minReconnectInterval := 10 * time.Second
	maxReconnectInterval := time.Minute

	// Create a new listener - we need a separate connection for LISTEN/NOTIFY
	listener := pq.NewListener(postgresURI+"?sslmode=disable", minReconnectInterval, maxReconnectInterval, nil)
	defer listener.Close()

	// Listen for notifications on the 'iot_flow_insert' channel
	err := listener.Listen("iot_flow_insert")
	if err != nil {
		setupLog.Error(err, "unable to listen for notifications")
		return
	}

	setupLog.Info("Successfully started listening for PostgreSQL notifications on channel 'iot_flow_insert'")

	// Process notifications in a separate goroutine
	for {
		select {
		case <-ctx.Done():
			setupLog.Info("Stopping listener for new flows")
			return
		case notification := <-listener.Notify:
			if notification != nil {
				setupLog.Info("Received notification", "channel", notification.Channel, "payload", notification.Extra)
				// Process the new flow when we receive a notification
				processNewFlowsByNotification(ctx, db, reconciler, notification.Extra)
			}
		case <-time.After(90 * time.Second):
			// Periodically ping the database to keep the connection alive
			setupLog.Info("Pinging database to keep connection alive")
			listener.Ping()
		}
	}
}

// processNewFlowsByNotification processes a new flow from the iot_flow table based on notification
func processNewFlowsByNotification(ctx context.Context, db *sql.DB, reconciler *controller.IoTListenerRequestReconciler, flowIDStr string) {
	// Convert the flow ID from the notification payload
	setupLog.Info("Processing new flow from notification", "flowIDStr", flowIDStr)

	// Query for the specific flow that was inserted
	query := "SELECT id, nodes, edges FROM iot_flow WHERE id = $1"
	row := db.QueryRowContext(ctx, query, flowIDStr)

	var flowID int
	var nodes, edges string
	err := row.Scan(&flowID, &nodes, &edges)
	if err != nil {
		if err == sql.ErrNoRows {
			setupLog.Info("Flow not found in iot_flow table", "flowID", flowIDStr)
			return
		}
		setupLog.Error(err, "unable to query flow", "flowID", flowIDStr)
		return
	}

	setupLog.Info("Processing new flow", "flowID", flowID)

	// Start collector and processor pods (singleton) - this will send the flow data via Kafka
	err = reconciler.StartCollectorAndProcessor(ctx, flowID, nodes, edges)
	if err != nil {
		setupLog.Error(err, "unable to start collector and processor pods", "flowID", flowID)
		return
	}

	setupLog.Info("Successfully processed new flow", "flowID", flowID)
}

// processExistingFlows processes all existing flows from the iot_flow table at startup
func processExistingFlows(ctx context.Context, db *sql.DB, reconciler *controller.IoTListenerRequestReconciler) {
	setupLog.Info("Processing all existing flows in iot_flow table...")

	// Query for all flows in the table
	query := "SELECT id, nodes, edges FROM iot_flow ORDER BY created_at ASC"
	rows, err := db.QueryContext(ctx, query)
	if err != nil {
		setupLog.Error(err, "unable to query existing flows")
		return
	}
	defer rows.Close()

	// Process each flow - but only start the collector and processor once
	firstFlow := true
	for rows.Next() {
		var flowID int
		var nodes, edges string
		err := rows.Scan(&flowID, &nodes, &edges)
		if err != nil {
			setupLog.Error(err, "unable to scan flow row")
			continue
		}

		setupLog.Info("Processing existing flow", "flowID", flowID)

		// Start collector and processor pods (singleton) - this will either create the pods or send flow data via Kafka
		err = reconciler.StartCollectorAndProcessor(ctx, flowID, nodes, edges)
		if err != nil {
			setupLog.Error(err, "unable to process existing flow", "flowID", flowID)
			// Continue processing other flows even if one fails
			continue
		}

		if firstFlow {
			setupLog.Info("Started collector and processor pods for first flow", "flowID", flowID)
			firstFlow = false
		} else {
			setupLog.Info("Sent flow update to existing collector and processor pods", "flowID", flowID)
		}
	}

	// Check for errors during iteration
	if err = rows.Err(); err != nil {
		setupLog.Error(err, "error iterating over flow rows")
	}

	setupLog.Info("Finished processing all existing flows")
}

func main() {

	var metricsAddr string
	var enableLeaderElection bool
	var probeAddr string
	var secureMetrics bool
	var enableHTTP2 bool
	flag.StringVar(&metricsAddr, "metrics-bind-address", ":8080", "The address the metric endpoint binds to.")
	flag.StringVar(&probeAddr, "health-probe-bind-address", ":8081", "The address the probe endpoint binds to.")
	flag.BoolVar(&enableLeaderElection, "leader-elect", false,
		"Enable leader election for controller manager. "+
			"Enabling this will ensure there is only one active controller manager.")
	flag.BoolVar(&secureMetrics, "metrics-secure", false,
		"If set the metrics endpoint is served securely")
	flag.BoolVar(&enableHTTP2, "enable-http2", false,
		"If set, HTTP/2 will be enabled for the metrics and webhook servers")
	opts := zap.Options{
		Development: true,
	}
	opts.BindFlags(flag.CommandLine)
	flag.Parse()

	ctrl.SetLogger(zap.New(zap.UseFlagOptions(&opts)))

	config := ReadConfig()

	// Run ls /etc/secrets to see the secrets that are mounted
	results, err := os.ReadDir("./etc/secrets")
	if err != nil {
		setupLog.Error(err, "unable to read secrets directory")
		os.Exit(1)
	}

	for _, result := range results {
		setupLog.Info(result.Name())
	}

	// Reading postgres uri from the secrets volume
	postgres_uri, err := os.ReadFile("./etc/secrets/uri")
	if err != nil {
		setupLog.Error(err, "unable to read DATABASE_URL")
		os.Exit(1)
	}

	// Connect to the PostgreSQL database
	db, err := sql.Open("postgres", string(postgres_uri)+"?sslmode=disable")
	if err != nil {
		setupLog.Error(err, "unable to connect to database")
		os.Exit(1)
	}
	defer db.Close()

	// Test the database connection
	err = db.Ping()
	if err != nil {
		setupLog.Error(err, "unable to ping database")
		os.Exit(1)
	}

	setupLog.Info("Successfully connected to database")

	// if the enable-http2 flag is false (the default), http/2 should be disabled
	// due to its vulnerabilities. More specifically, disabling http/2 will
	// prevent from being vulnerable to the HTTP/2 Stream Cancellation and
	// Rapid Reset CVEs. For more information see:
	// - https://github.com/advisories/GHSA-qppj-fm5r-hxr3
	// - https://github.com/advisories/GHSA-4374-p667-p6c8
	disableHTTP2 := func(c *tls.Config) {
		setupLog.Info("disabling http/2")
		c.NextProtos = []string{"http/1.1"}
	}

	tlsOpts := []func(*tls.Config){}
	if !enableHTTP2 {
		tlsOpts = append(tlsOpts, disableHTTP2)
	}

	webhookServer := webhook.NewServer(webhook.Options{
		TLSOpts: tlsOpts,
	})

	mgr, err := ctrl.NewManager(ctrl.GetConfigOrDie(), ctrl.Options{
		Scheme: scheme,
		Metrics: metricsserver.Options{
			BindAddress:   metricsAddr,
			SecureServing: secureMetrics,
			TLSOpts:       tlsOpts,
		},
		WebhookServer:          webhookServer,
		HealthProbeBindAddress: probeAddr,
		LeaderElection:         enableLeaderElection,
		LeaderElectionID:       "68777113.visualiseinfo.com",
		// LeaderElectionReleaseOnCancel defines if the leader should step down voluntarily
		// when the Manager ends. This requires the binary to immediately end when the
		// Manager is stopped, otherwise, this setting is unsafe. Setting this significantly
		// speeds up voluntary leader transitions as the new leader don't have to wait
		// LeaseDuration time first.
		//
		// In the default scaffold provided, the program ends immediately after
		// the manager stops, so would be fine to enable this option. However,
		// if you are doing or is intended to do any operation such as perform cleanups
		// after the manager stops then its usage might be unsafe.
		// LeaderElectionReleaseOnCancel: true,
	})
	if err != nil {
		setupLog.Error(err, "unable to start manager")
		os.Exit(1)
	}

	// Setup reconciliation loop
	reconciler := &controller.IoTListenerRequestReconciler{
		Client:          mgr.GetClient(),
		Scheme:          mgr.GetScheme(),
		CollectorImage:  config.CollectorImage,
		ProcessorImage:  config.ProcessorImage,
		PullPolicy:      config.PullPolicy,
		ImagePullSecret: config.ImagePullSecret,
		ReleaseName:     config.ReleaseName,
		DatabaseUri:     string(postgres_uri),
		HostAliases:     config.HostAliases,
		PodAnnotations:  config.Annotations,
	}

	// Create Kafka producer and set it in the reconciler
	kafkaProducer, err := createKafkaProducer()
	if err != nil {
		setupLog.Error(err, "unable to create Kafka producer")
		os.Exit(1)
	}
	reconciler.KafkaProducer = kafkaProducer

	// Read Kafka topic from config file
	kafkaTopic := "flow-updates" // default value
	kafkaTopicBytes, err := os.ReadFile("./etc/config/FLOW_UPDATES_TOPIC")
	if err != nil {
		setupLog.Info("unable to read FLOW_UPDATES_TOPIC from config file, using default", "error", err)
	} else {
		kafkaTopic = strings.TrimSpace(string(kafkaTopicBytes))
	}
	reconciler.KafkaTopic = kafkaTopic

	if err = reconciler.SetupWithManager(mgr); err != nil {
		setupLog.Error(err, "unable to create controller", "controller", "IoTListenerRequest")
		os.Exit(1)
	}
	//+kubebuilder:scaffold:builder

	if err := mgr.AddHealthzCheck("healthz", healthz.Ping); err != nil {
		setupLog.Error(err, "unable to set up health check")
		os.Exit(1)
	}
	if err := mgr.AddReadyzCheck("readyz", healthz.Ping); err != nil {
		setupLog.Error(err, "unable to set up ready check")
		os.Exit(1)
	}

	// Process all existing flows at startup
	processExistingFlows(context.Background(), db, reconciler)

	// Start listening for new flows in a separate goroutine
	go listenForNewFlows(context.Background(), db, string(postgres_uri), reconciler)

	setupLog.Info("starting manager")
	if err := mgr.Start(ctrl.SetupSignalHandler()); err != nil {
		setupLog.Error(err, "problem running manager")
		os.Exit(1)
	}

}