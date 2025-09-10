#!/bin/bash

# Script to insert new flow records into the iot_flow table

echo "Prepare to insert a new flow record into the iot_flow table..."

# Wait for PostgreSQL Pod to start
echo "Waiting for PostgreSQL Pod to start..."
kubectl wait --for=condition=ready pod -l app=postgres --timeout=120s

# Get PostgreSQL Pod name
POSTGRES_POD=$(kubectl get pod -l app=postgres -o jsonpath="{.items[0].metadata.name}" -n listener-operator-system)

# Generate random ID (to avoid conflicts)
RANDOM_ID=$((1000 + RANDOM % 9000))

# Get MQTT host address, use default value if environment variable is not set
if [ -z "$MQTT_HOST" ]; then
    echo "MQTT_HOST environment variable is not set, trying to auto-detect host IP..."

    # Check if running in Linux environment
    PLATFORM=$(uname)
    
    if [[ "$PLATFORM" == "Linux" ]]; then
        echo "Detected Linux environment"

        # On Linux, host.docker.internal may not be available
        # We need to get the host IP address

        # Method 1: Try to get the gateway IP using docker network inspect
        if command -v docker &> /dev/null; then
            echo "Checking Docker network..."
            # Find kind network
            KIND_NETWORK=$(docker network ls | grep kind | awk '{print $1}' | head -n 1)
            
            if [ -n "$KIND_NETWORK" ]; then
                echo "Found Kind network: $KIND_NETWORK"
                # Get network gateway IP, fix docker network inspect command
                HOST_IP=$(docker network inspect "$KIND_NETWORK" 2>/dev/null | grep Gateway | head -1 | awk -F'"' '{print $4}')
                
                if [ -n "$HOST_IP" ]; then
                    echo "Got host IP from Docker network: $HOST_IP"
                else
                    echo "Failed to get gateway IP from Docker network"
                fi
            else
                echo "Kind network not found"
            fi
        fi

        # Method 2: If Method 1 fails, try to use the routing table (check if ip command exists)
        if [ -z "$HOST_IP" ]; then
            echo "Trying to detect host IP using routing table..."
            # Check if ip command exists
            if command -v ip &> /dev/null; then
                HOST_IP=$(ip route | grep default | awk '{print $3}' | head -n 1)
            elif command -v route &> /dev/null; then
                # Use route command on some systems
                HOST_IP=$(route -n | grep '^0.0.0.0' | awk '{print $2}' | head -n 1)
            fi
            
            if [ -n "$HOST_IP" ]; then
                echo "Got host IP from routing table: $HOST_IP"
            else
                echo "Failed to get default gateway from routing table"
            fi
        fi

        # Method 3: If all else fails, try using 172.17.0.1 (Docker default bridge gateway)
        if [ -z "$HOST_IP" ]; then
            echo "Trying to use Docker default bridge gateway: 172.17.0.1"
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

echo "The MQTT host address will be used: $MQTT_HOST"

# Connect to the Postgres database in k8s and insert a new flow record
# Separate shell script logic from SQL statements to avoid using shell syntax in SQL
kubectl exec -it "$POSTGRES_POD" -n listener-operator-system -- psql -U postgres -d iot_dataflow_manager << EOF

-- Insert a new flow record
INSERT INTO iot_flow (id, site_id, name, nodes, edges) 
VALUES ($RANDOM_ID, 1, 'Initial Flow $RANDOM_ID', 
        ('[{"id": "mqtt_source_1", "type": "source", "data": {"source": {"type": "MQTT", "config": {"host": "$MQTT_HOST", "port": 1883}}, "config": {"topic": "sensors/temperature"}}}, {"id":"debug-1","type":"debug","data":{}}]')::jsonb, 
        ('[{"source":"mqtt_source_1","target":"debug-1","sourceHandle":"source","targetHandle":"target"}]')::jsonb)
ON CONFLICT (id) DO NOTHING;

-- Verify that the insertion was successful
SELECT * FROM iot_flow WHERE id = $RANDOM_ID;

EOF

echo "The new flow record has been inserted into the iot_flow table with ID: $RANDOM_ID"
echo "Please check the Operator logs to confirm if the new record has been detected"