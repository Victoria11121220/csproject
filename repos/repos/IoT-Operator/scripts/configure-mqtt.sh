#!/bin/bash

# Automatically detect the environment and configure the correct MQTT address

echo "Starting MQTT address configuration..."

# Check if running in a Linux environment
PLATFORM=$(uname)

if [[ "$PLATFORM" == "Linux" ]]; then
    echo "Detected Linux environment, using dedicated configuration script..."
    ./scripts/configure-mqtt-linux.sh
    exit $?
else
    # macOS and other environments use host.docker.internal
    HOST_IP="host.docker.internal"
    echo "Non-Linux environment, using default value: host.docker.internal"

    # Update mqtt-config.yaml file
    cat > k8s-manifests/mqtt-config.yaml << EOF
apiVersion: v1
kind: ConfigMap
metadata:
  name: mqtt-config
data:
  MQTT_BROKER_HOST: "$HOST_IP"
  MQTT_BROKER_PORT: "1883"
EOF

    echo "Updated k8s-manifests/mqtt-config.yaml file"
    echo "MQTT_BROKER_HOST set to: $HOST_IP"

    # Export environment variables for use by database initialization scripts
    export MQTT_HOST=$HOST_IP
    echo "Exported environment variables MQTT_HOST=$HOST_IP"
fi