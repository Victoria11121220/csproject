#!/bin/bash

# IoT-Operator Deletion Script

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

# 4. Delete Kafka Topics
echo "4. Delete Kafka Topics..."
# Note: Kafka topics are automatically deleted when the Kafka pod is deleted, but if manual deletion is needed, the following commands can be used
# kubectl exec -n listener-operator-system $(kubectl get pods -n listener-operator-system -l app=kafka -o name) -- kafka-topics.sh --delete --topic iot_triggers --bootstrap-server localhost:9092
# kubectl exec -n listener-operator-system $(kubectl get pods -n listener-operator-system -l app=kafka -o name) -- kafka-topics.sh --delete --topic flow-updates --bootstrap-server localhost:9092

# 5. Delete ConfigMap
echo "5. Delete ConfigMap..."
kubectl delete -f k8s-manifests/iot-env-config.yaml -n listener-operator-system --ignore-not-found=true

# 6. Uninstall Operator
echo "6. Uninstall Operator..."
make undeploy

# 7. Uninstall CRDs
echo "7. Uninstall CRDs..."
make uninstall

echo "Deletion complete!"