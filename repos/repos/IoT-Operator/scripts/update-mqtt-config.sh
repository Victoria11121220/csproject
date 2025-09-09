#!/bin/bash

# Update MQTT configuration to use host.docker.internal

echo "Setting MQTT configuration to use host.docker.internal..."

# Update mqtt-config.yaml file
cat > k8s-manifests/mqtt-config.yaml << EOF
apiVersion: v1
kind: ConfigMap
metadata:
  name: mqtt-config
data:
  MQTT_BROKER_HOST: "host.docker.internal"
  MQTT_BROKER_PORT: "1883"
EOF

echo "Updated k8s-manifests/mqtt-config.yaml file"
echo "MQTT_BROKER_HOST set to: host.docker.internal"