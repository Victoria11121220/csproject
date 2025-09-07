#!/bin/bash

# Start MQTT Source

echo "=== Starting MQTT Source ==="

# Check if Docker is available
if ! command -v docker &> /dev/null
then
    echo "Docker is not installed. Please install Docker first."
    exit 1
fi

# Check if Docker Compose is available
if ! command -v docker-compose &> /dev/null
then
    echo "docker-compose is not installed. Please install docker-compose first."
    exit 1
fi

# Create necessary directories
mkdir -p mosquitto/data
mkdir -p mosquitto/log

# Start MQTT broker
echo "Starting MQTT broker..."
docker-compose up -d

# Wait for broker to start
echo "Waiting for MQTT broker to start..."
sleep 5

# Check if MQTT broker is running
if docker ps | grep -q mqtt-broker; then
    echo "MQTT broker is running on localhost:1883"
    echo ""
    echo "To publish test messages, use:"
    echo "  mosquitto_pub -h localhost -p 1883 -t 'sensors/temperature' -m '{\"temperature\": 22.5, \"unit\": \"celsius\"}'"
    echo ""
    echo "To subscribe to messages, use:"
    echo "  mosquitto_sub -h localhost -p 1883 -t 'sensors/#'"
else
    echo "Failed to start MQTT broker. Check the logs with 'docker-compose logs mosquitto'"
fi