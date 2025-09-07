#!/bin/bash

# Stop MQTT Source

echo "=== Stopping MQTT Source ==="

# Check if Docker is available
if ! command -v docker &> /dev/null
then
    echo "Docker is not installed."
    exit 1
fi

# Check if Docker Compose is available
if ! command -v docker-compose &> /dev/null
then
    echo "docker-compose is not installed."
    exit 1
fi

# Stop MQTT broker
echo "Stopping MQTT broker..."
docker-compose down

echo "MQTT broker stopped."