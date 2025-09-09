#!/bin/bash

# Script to detect host IP address
# Used to correctly configure MQTT broker address in different environments

echo "Detecting runtime environment..."

# Check if running in Docker environment
if [ -f /.dockerenv ]; then
    echo "Detected running in Docker container"
    # In Docker container, host.docker.internal should be available
    HOST_IP="host.docker.internal"
elif [ -n "$DOCKER_HOST" ]; then
    echo "Detected Docker host environment variable"
    # Extract Docker host IP
    HOST_IP=$(echo $DOCKER_HOST | sed 's/tcp:\/\///' | sed 's/:[0-9]*$//')
else
    # On Linux, we may need to use gateway IP
    # Check if there is a kind network
    if command -v docker &> /dev/null; then
        KIND_NETWORK=$(docker network ls | grep kind | awk '{print $1}')
        if [ -n "$KIND_NETWORK" ]; then
            echo "Detected Kind network"
            # Get Docker gateway IP
            HOST_IP=$(docker network inspect $KIND_NETWORK | grep Gateway | head -1 | awk -F'"' '{print $4}')
            if [ -z "$HOST_IP" ]; then
                # Fallback: use default gateway
                HOST_IP=$(ip route | grep default | awk '{print $3}' | head -n 1)
            fi
        else
            echo "Kind network not detected, using default gateway"
            # Use default gateway IP
            HOST_IP=$(ip route | grep default | awk '{print $3}' | head -n 1)
        fi
    else
        echo "Docker not detected, using default gateway"
        # Use default gateway IP
        HOST_IP=$(ip route | grep default | awk '{print $3}' | head -n 1)
    fi
fi

# If all else fails, use localhost as a last resort
if [ -z "$HOST_IP" ]; then
    HOST_IP="localhost"
fi

echo "Detected host IP: $HOST_IP"
echo "$HOST_IP"