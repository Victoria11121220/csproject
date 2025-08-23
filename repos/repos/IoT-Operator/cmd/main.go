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
	"crypto/tls"
	"flag"
	"os"

	// Import all Kubernetes client auth plugins (e.g. Azure, GCP, OIDC, etc.)
	// to ensure that exec-entrypoint and run can make use of them.
	_ "k8s.io/client-go/plugin/pkg/client/auth"

	"k8s.io/apimachinery/pkg/runtime"
	utilruntime "k8s.io/apimachinery/pkg/util/runtime"
	clientgoscheme "k8s.io/client-go/kubernetes/scheme"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/healthz"
	"sigs.k8s.io/controller-runtime/pkg/log/zap"
	metricsserver "sigs.k8s.io/controller-runtime/pkg/metrics/server"
	"sigs.k8s.io/controller-runtime/pkg/webhook"

	iotv1alpha1 "visualiseinfo.com/m/api/v1alpha1"
	"visualiseinfo.com/m/internal/controller"
	//+kubebuilder:scaffold:imports
)

var (
	scheme   = runtime.NewScheme()
	setupLog = ctrl.Log.WithName("setup")
)

func init() {
	utilruntime.Must(clientgoscheme.AddToScheme(scheme))

	utilruntime.Must(iotv1alpha1.AddToScheme(scheme))
	//+kubebuilder:scaffold:scheme
}

// Config holds configuration values for the operator
type Config struct {
	CollectorImage  string
	ProcessorImage  string
	ImagePullSecret string
	KafkaBrokers    string
	KafkaTopic      string
}

// ReadConfig reads the configuration from the mounted config files or environment variables
func ReadConfig() Config {
	var config Config

	// Try to read from environment variables first, fallback to files
	if collectorImage := os.Getenv("COLLECTOR_IMAGE"); collectorImage != "" {
		config.CollectorImage = collectorImage
	} else {
		collectorImage, err := os.ReadFile("./etc/config/COLLECTOR_IMAGE")
		if err != nil {
			setupLog.Error(err, "unable to read COLLECTOR_IMAGE")
			os.Exit(1)
		}
		config.CollectorImage = string(collectorImage)
	}

	if processorImage := os.Getenv("PROCESSOR_IMAGE"); processorImage != "" {
		config.ProcessorImage = processorImage
	} else {
		processorImage, err := os.ReadFile("./etc/config/PROCESSOR_IMAGE")
		if err != nil {
			setupLog.Error(err, "unable to read PROCESSOR_IMAGE")
			os.Exit(1)
		}
		config.ProcessorImage = string(processorImage)
	}

	if imagePullSecret := os.Getenv("IMAGE_PULL_SECRET"); imagePullSecret != "" {
		config.ImagePullSecret = imagePullSecret
	} else {
		imagePullSecret, err := os.ReadFile("./etc/config/IMAGE_PULL_SECRET")
		if err != nil {
			setupLog.Error(err, "unable to read IMAGE_PULL_SECRET")
			os.Exit(1)
		}
		config.ImagePullSecret = string(imagePullSecret)
	}

	if kafkaBrokers := os.Getenv("KAFKA_BROKERS"); kafkaBrokers != "" {
		config.KafkaBrokers = kafkaBrokers
	} else {
		kafkaBrokers, err := os.ReadFile("./etc/config/KAFKA_BROKERS")
		if err != nil {
			setupLog.Error(err, "unable to read KAFKA_BROKERS")
			os.Exit(1)
		}
		config.KafkaBrokers = string(kafkaBrokers)
	}

	if kafkaTopic := os.Getenv("KAFKA_TOPIC"); kafkaTopic != "" {
		config.KafkaTopic = kafkaTopic
	} else {
		kafkaTopic, err := os.ReadFile("./etc/config/KAFKA_TOPIC")
		if err != nil {
			setupLog.Error(err, "unable to read KAFKA_TOPIC")
			os.Exit(1)
		}
		config.KafkaTopic = string(kafkaTopic)
	}

	return config
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

	// Reading postgres uri from the secrets volume or environment variable
	var postgresURI []byte
	if uri := os.Getenv("POSTGRES_URI"); uri != "" {
		postgresURI = []byte(uri)
	} else {
		// Reading postgres uri from the secrets volume
		var err error
		postgresURI, err = os.ReadFile("./etc/secrets/uri")
		if err != nil {
			setupLog.Error(err, "unable to read postgres uri from secrets")
			os.Exit(1)
		}
	}

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
		Scheme:                 scheme,
		Metrics:                metricsserver.Options{BindAddress: metricsAddr, SecureServing: secureMetrics, TLSOpts: tlsOpts},
		WebhookServer:          webhookServer,
		HealthProbeBindAddress: probeAddr,
		LeaderElection:         enableLeaderElection,
		LeaderElectionID:       "68777113.visualiseinfo.com",
	})
	if err != nil {
		setupLog.Error(err, "unable to start manager")
		os.Exit(1)
	}

	if err = (&controller.IoTListenerRequestReconciler{
		Client:          mgr.GetClient(),
		Scheme:          mgr.GetScheme(),
		CollectorImage:  config.CollectorImage,
		ProcessorImage:  config.ProcessorImage,
		ImagePullSecret: config.ImagePullSecret,
		DatabaseUri:     string(postgresURI),
		KafkaBrokers:    config.KafkaBrokers,
		KafkaTopic:      config.KafkaTopic,
	}).SetupWithManager(mgr); err != nil {
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

	setupLog.Info("starting manager")
	if err := mgr.Start(ctrl.SetupSignalHandler()); err != nil {
		setupLog.Error(err, "problem running manager")
		os.Exit(1)
	}
}