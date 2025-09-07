#!/bin/bash

# Publish test MQTT messages

echo "=== Publishing Test MQTT Messages ==="

# Check if mosquitto-clients is installed
if ! command -v mosquitto_pub &> /dev/null
then
    echo "mosquitto-clients not found. Installing..."
    if command -v apt-get &> /dev/null
    then
        sudo apt-get update && sudo apt-get install -y mosquitto-clients
    elif command -v brew &> /dev/null
    then
        brew install mosquitto
    else
        echo "Please install mosquitto-clients manually"
        exit 1
    fi
fi

# Publish test messages
echo "Publishing temperature sensor data..."
mosquitto_pub -h localhost -p 1883 -t "sensors/temperature" -m '{"temperature": 22.5, "unit": "celsius", "timestamp": "2025-09-07T10:00:00Z"}'

echo "Publishing humidity sensor data..."
mosquitto_pub -h localhost -p 1883 -t "sensors/humidity" -m '{"humidity": 45.2, "unit": "percent", "timestamp": "2025-09-07T10:00:05Z"}'

echo "Publishing combined sensor data..."
mosquitto_pub -h localhost -p 1883 -t "sensors/combined" -m '{"temperature": 23.1, "humidity": 46.8, "pressure": 1013.25, "timestamp": "2025-09-07T10:00:10Z"}'

echo "Publishing device status message..."
mosquitto_pub -h localhost -p 1883 -t "devices/status" -m '{"device_id": "sensor_001", "status": "online", "battery": 87, "timestamp": "2025-09-07T10:00:15Z"}'

echo ""
echo "Test messages published successfully!"