#!/bin/bash

# IoT-Operator deployment script

echo "Starting deployment of IoT-Operator..."

make generate
# 1. Build Docker image
echo "1. Build Docker image..."
make docker-build IMG=listener-operator:dev

# 2. Load image into Kind cluster (if using Kind)
echo "2. Load image into Kind cluster..."
kind load docker-image listener-operator:dev --name listener-cluster

# 3. Install CRDs
echo "3. Install CRDs..."
make install

echo "4. Create listener-operator-system namespace..."
kubectl create namespace listener-operator-system 2>/dev/null || true



# 4. Deploy Operator
echo "4. Deploy Operator..."
make deploy IMG=listener-operator:dev

# 5. Deploy PostgreSQL
echo "5. Deploy PostgreSQL..."
kubectl apply -f k8s-manifests/postgres-deployment.yaml -n listener-operator-system

# 6. Deploy Kafka
echo "6. Deploy Kafka..."
kubectl apply -f k8s-manifests/kafka-deployment.yaml -n listener-operator-system

# 7. Wait for Kafka to be ready
echo "7. Wait for Kafka to be ready..."
kubectl wait --for=condition=ready pod -l app=kafka --timeout=120s -n listener-operator-system

# 8. Create Kafka Topic
echo "8. Create Kafka Topic..."
kubectl exec -n listener-operator-system $(kubectl get pods -n listener-operator-system -l app=kafka -o name) -- kafka-topics.sh --create --topic iot_triggers --bootstrap-server localhost:9092 --if-not-exists
kubectl exec -n listener-operator-system $(kubectl get pods -n listener-operator-system -l app=kafka -o name) -- kafka-topics.sh --create --topic flow-updates --bootstrap-server localhost:9092 --if-not-exists

# 9. Automatically detect and update MQTT configuration
echo "9. Automatically detect and update MQTT configuration..."
./scripts/configure-mqtt.sh

# 10. Create ConfigMap for environment variables
echo "10. Create environment variable ConfigMap..."
kubectl apply -f k8s-manifests/iot-env-config.yaml -n listener-operator-system
kubectl apply -f k8s-manifests/mqtt-config.yaml -n listener-operator-system

# 11. Wait for PostgreSQL to be ready
echo "11. Wait for PostgreSQL to be ready..."
kubectl wait --for=condition=ready pod -l app=postgres --timeout=120s

# 12. Initialize PostgreSQL database
echo "12. Initialize PostgreSQL database..."
./k8s-manifests/init-postgres.sh

# 13. Create IoTListenerRequest instances (optional, for testing old CRD mechanism)
echo "13. Create IoTListenerRequest instances..."
kubectl apply -k config/samples/

echo "Deployment complete!"
echo "Check Pod status: kubectl get pods -n listener-operator-system"
echo "View Operator logs: kubectl logs -n listener-operator-system -l control-plane=controller-manager"