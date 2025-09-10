#!/bin/bash

# Dedicated script for configuring MQTT addresses for Linux environments

echo "Configuring MQTT addresses for Linux environment..."

# In Linux environments, we use several methods to detect the host IP

# Method 1: Use Docker network inspection
HOST_IP=""
if command -v docker &> /dev/null; then
    echo "Checking Docker network..."
    # Find kind network
    KIND_NETWORK=$(docker network ls | grep kind | awk '{print $1}' | head -n 1)
    
    if [ -n "$KIND_NETWORK" ]; then
        echo "Found Kind network: $KIND_NETWORK"
        # Get network gateway IP
        HOST_IP=$(docker network inspect $KIND_NETWORK | grep Gateway | head -1 | awk -F'"' '{print $4}')
        
        if [ -n "$HOST_IP" ]; then
            echo "Got host IP from Docker network: $HOST_IP"
        else
            echo "Failed to get gateway IP from Docker network"
        fi
    else
        echo "Kind network not found"
    fi
fi

# Method 2: If Method 1 fails, try to use the routing table
if [ -z "$HOST_IP" ]; then
    echo "Trying to detect host IP using routing table..."
    HOST_IP=$(ip route | grep default | awk '{print $3}' | head -n 1)
    
    if [ -n "$HOST_IP" ]; then
        echo "Got host IP from routing table: $HOST_IP"
    else
        echo "Failed to get default gateway from routing table"
    fi
fi

# Method 3: If all else fails, try using 172.17.0.1 (Docker default bridge gateway)
if [ -z "$HOST_IP" ]; then
    echo "Trying to use Docker default bridge gateway: 172.17.0.1"
    HOST_IP="172.17.0.1"
    echo "Using default Docker bridge gateway: $HOST_IP"
fi

# Update configuration
echo "Setting MQTT configuration to use detected address: $HOST_IP"

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