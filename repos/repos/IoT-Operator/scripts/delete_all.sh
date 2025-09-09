#!/bin/bash

# IoT-Operator Delete Script

echo "Starting deletion of IoT-Operator..."
# kubectl delete deployment listener-operator-controller-manager -n listener-operator-system
# 1. Delete IoTListenerRequest instances
echo "1. Delete IoTListenerRequest instances..."
kubectl delete -k config/samples/ --ignore-not-found=true

# 2. Delete PostgreSQL
echo "2. Delete PostgreSQL..."
kubectl delete -f k8s-manifests/postgres-deployment.yaml -n listener-operator-system --ignore-not-found=true

# 3. Delete Kafka
echo "3. Delete Kafka..."
kubectl delete -f k8s-manifests/kafka-deployment.yaml -n listener-operator-system --ignore-not-found=true

# 4. Delete ConfigMap
echo "4. Delete ConfigMap..."
kubectl delete -f k8s-manifests/iot-env-config.yaml -n listener-operator-system --ignore-not-found=true

# 5. Uninstall Operator
echo "5. Uninstall Operator..."
make undeploy

# 6. Uninstall CRDs
echo "6. Uninstall CRDs..."
make uninstall

echo "Deletion complete!"