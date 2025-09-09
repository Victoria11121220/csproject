#!/bin/bash

# Start the HTTP origin server

echo "=== Starting HTTP Source Server ==="

# Check if Python is available
if ! command -v python3 &> /dev/null
then
    echo "Python3 is not installed. Please install Python3 first."
    exit 1
fi

# Start the HTTP server
echo "Starting HTTP server on port 8080..."
echo "Access sensor data at: http://localhost:8080/sensor-data"
echo "Press Ctrl+C to stop the server"

python3 server.py