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
	_ "github.com/lib/pq"

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

// listenForNewFlows listens for new flows in the iot_flow table using PostgreSQL LISTEN/NOTIFY
func listenForNewFlows(ctx context.Context, db *sql.DB, reconciler *controller.IoTListenerRequestReconciler) {
	setupLog.Info("Starting to listen for new flows in iot_flow table...")

	// Listen for notifications on the 'iot_flow_insert' channel
	_, err := db.Exec("LISTEN iot_flow_insert")
	if err != nil {
		setupLog.Error(err, "unable to listen for notifications")
		return
	}

	// Use Polling approach since LISTEN/NOTIFY with pq driver requires a dedicated connection
	for {
		select {
		case <-ctx.Done():
			return
		default:
			// Check for new flows by querying the database
			processNewFlows(ctx, db, reconciler)

			// Sleep for a bit before checking again
			time.Sleep(10 * time.Second)
		}
	}
}

// processNewFlows processes new flows from the iot_flow table
func processNewFlows(ctx context.Context, db *sql.DB, reconciler *controller.IoTListenerRequestReconciler) {
	// Query for flows that have not been processed yet
	// For simplicity, we'll just get the latest flow
	query := "SELECT id, nodes, edges FROM iot_flow ORDER BY created_at DESC LIMIT 1"
	row := db.QueryRowContext(ctx, query)

	var flowID int
	var nodes, edges string
	err := row.Scan(&flowID, &nodes, &edges)
	if err != nil {
		if err == sql.ErrNoRows {
			setupLog.Info("No flows found in iot_flow table")
			return
		}
		setupLog.Error(err, "unable to query latest flow")
		return
	}

	setupLog.Info("Processing new flow", "flowID", flowID)

	// Start collector and processor pods
	err = reconciler.StartCollectorAndProcessor(ctx, flowID, nodes, edges)
	if err != nil {
		setupLog.Error(err, "unable to start collector and processor pods")
		return
	}

	setupLog.Info("Successfully started collector and processor pods", "flowID", flowID)
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

	// Start listening for new flows in a separate goroutine
	go listenForNewFlows(context.Background(), db, reconciler)

	setupLog.Info("starting manager")
	if err := mgr.Start(ctrl.SetupSignalHandler()); err != nil {
		setupLog.Error(err, "problem running manager")
		os.Exit(1)
	}

}