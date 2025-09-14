#!/bin/bash

# Script to insert new flow records into iot_flow table

echo "Preparing to insert new flow records into iot_flow table..."

# Wait for PostgreSQL Pod to start
echo "Waiting for PostgreSQL Pod to start..."
kubectl wait --for=condition=ready pod -l app=postgres --timeout=120s

# Get PostgreSQL Pod name
POSTGRES_POD=$(kubectl get pod -l app=postgres -o jsonpath="{.items[0].metadata.name}" -n listener-operator-system)

# Generate random ID (to avoid conflicts)
RANDOM_ID=$((1000 + RANDOM % 9000))

# Get MQTT host address, try to auto-detect if environment variable is not set
if [ -z "$MQTT_HOST" ]; then
    echo "MQTT_HOST environment variable not set, trying to auto-detect host IP..."
    
    # Check if on Linux environment
    PLATFORM=$(uname)
    
    if [[ "$PLATFORM" == "Linux" ]]; then
        echo "Linux environment detected"
        
        # On Linux, host.docker.internal may not be available
        # We need to get the host machine's IP address
        
        # Method 1: Try to use docker network inspect to get gateway IP
        if command -v docker &> /dev/null; then
            echo "Detecting Docker network..."
            # Find kind network
            KIND_NETWORK=$(docker network ls | grep kind | awk '{print $1}' | head -n 1)
            
            if [ -n "$KIND_NETWORK" ]; then
                echo "Found Kind network: $KIND_NETWORK"
                # Get network gateway IP, fix docker network inspect command
                HOST_IP=$(docker network inspect "$KIND_NETWORK" 2>/dev/null | grep Gateway | head -1 | awk -F'"' '{print $4}')
                
                if [ -n "$HOST_IP" ]; then
                    echo "Host IP obtained through Docker network: $HOST_IP"
                else
                    echo "Failed to get gateway IP from Docker network"
                fi
            else
                echo "Kind network not found"
            fi
        fi
        
        # Method 2: If method 1 fails, try using routing table (check if ip command exists)
        if [ -z "$HOST_IP" ]; then
            echo "Detecting host IP using routing table..."
            # Check if ip command exists
            if command -v ip &> /dev/null; then
                HOST_IP=$(ip route | grep default | awk '{print $3}' | head -n 1)
            elif command -v route &> /dev/null; then
                # On some systems use route command
                HOST_IP=$(route -n | grep '^0.0.0.0' | awk '{print $2}' | head -n 1)
            fi
            
            if [ -n "$HOST_IP" ]; then
                echo "Host IP obtained through routing table: $HOST_IP"
            else
                echo "Failed to get default gateway from routing table"
            fi
        fi
        
        # Method 3: If all above methods fail, try using 172.17.0.1 (Docker default bridge gateway)
        if [ -z "$HOST_IP" ]; then
            echo "Trying Docker default bridge gateway: 172.17.0.1"
            HOST_IP="172.17.0.1"
            echo "Using default Docker bridge gateway: $HOST_IP"
        fi
    else
        # macOS and other environments use host.docker.internal
        HOST_IP="host.docker.internal"
        echo "Non-Linux environment, using default value: host.docker.internal"
    fi
    
    # Set MQTT host address
    MQTT_HOST=$HOST_IP
else
    echo "Using MQTT host address set in environment variable: $MQTT_HOST"
fi

echo "Will use MQTT host address: $MQTT_HOST"

# Connect to postgres database in k8s and insert new flow records
# Separate shell script logic from SQL statements to avoid using shell syntax in SQL
kubectl exec -it "$POSTGRES_POD" -n listener-operator-system -- psql -U postgres -d iot_dataflow_manager << EOF

-- Insert new flow records
INSERT INTO iot_flow (id, site_id, name, nodes, edges) 
VALUES ($RANDOM_ID, 1, 'Initial Flow $RANDOM_ID', 
        ('[{"id": "mqtt_source_1", "type": "source", "data": {"source": {"type": "MQTT", "config": {"host": "$MQTT_HOST", "port": 1883}}, "config": {"topic": "sensors/temperature"}}}, {"id":"debug-1","type":"debug","data":{}}]')::jsonb, 
        ('[{"source":"mqtt_source_1","target":"debug-1","sourceHandle":"source","targetHandle":"target"}]')::jsonb)
ON CONFLICT (id) DO NOTHING;

-- Verify if insertion was successful
SELECT * FROM iot_flow WHERE id = $RANDOM_ID;

EOF

echo "Completed inserting new flow records into iot_flow table with ID: $RANDOM_ID"
echo "Please check Operator logs to confirm if new records were detected"