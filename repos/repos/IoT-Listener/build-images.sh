#!/bin/bash

echo "Building IoT Collector image..."
docker buildx build --target collector --output type=docker,name=iot-collector .

echo "Building IoT Processor image..."
docker buildx build --target processor --output type=docker,name=iot-processor .

echo "Loading images into Kind cluster..."
kind load docker-image iot-processor:latest --name listener-cluster
kind load docker-image iot-collector:latest --name listener-cluster

echo "Build and load complete!"
echo "Available images:"
docker images | grep iot-