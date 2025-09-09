#!/bin/bash

# View the contents of the iot_flow_debug_message table in the PostgreSQL database in the cluster

echo "Start viewing the contents of the iot_flow_debug_message table..."

# Wait for PostgreSQL Pod to start
echo "Waiting for PostgreSQL Pod to start..."
kubectl wait --for=condition=ready pod -l app=postgres --timeout=120s -n listener-operator-system

# Get PostgreSQL Pod name
POSTGRES_POD=$(kubectl get pod -l app=postgres -o jsonpath="{.items[0].metadata.name}" -n listener-operator-system)
echo "Found PostgreSQL Pod: $POSTGRES_POD"

# View the contents of iot_flow_debug_message table
echo "Contents of iot_flow_debug_message table:"
echo "=============================="

kubectl exec -it $POSTGRES_POD -n listener-operator-system -- psql -U postgres -d iot_dataflow_manager -c "SELECT * FROM iot_flow_debug_message ORDER BY timestamp DESC LIMIT 20;"

echo ""
echo "Table structure information:"
echo "==========="

kubectl exec -it $POSTGRES_POD -n listener-operator-system -- psql -U postgres -d iot_dataflow_manager -c "\d iot_flow_debug_message;"

echo ""
echo "Total number of records:"
echo "========="

kubectl exec -it $POSTGRES_POD -n listener-operator-system -- psql -U postgres -d iot_dataflow_manager -c "SELECT COUNT(*) FROM iot_flow_debug_message;"