#!/bin/bash

# Subscribe to MQTT Messages

echo "=== Subscribing to MQTT Messages ==="
echo "Listening for messages on topic 'sensors/#' and 'devices/#'..."
echo "Press Ctrl+C to stop."

# Check if mosquitto-clients is installed
if ! command -v mosquitto_sub &> /dev/null
then
    echo "mosquitto-clients not found. Please install mosquitto-clients first."
    exit 1
fi

# Subscribe to messages
mosquitto_sub -h localhost -p 1883 -t "sensors/#" -t "devices/#"