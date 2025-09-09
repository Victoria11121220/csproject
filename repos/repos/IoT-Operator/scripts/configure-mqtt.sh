#!/bin/bash

# Automatically detect the environment and configure the correct MQTT address

echo "Start configuring the MQTT address..."

# Check whether you are in Linux environment
# If PLATFORM environment variable is not set, use uname command to detect
if [ -z "$PLATFORM" ]; then
    PLATFORM=$(uname)
fi

echo "Detected platform: $PLATFORM"

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

    # Export environment variable for database initialization script
    export MQTT_HOST=$HOST_IP
    echo "Exported environment variable MQTT_HOST=$HOST_IP"
fi