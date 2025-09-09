#!/bin/bash

# Update the MQTT host IP of the database in the cluster to the host machine's IP
# Used to solve the problem that host.docker.internal cannot be accessed in Linux environment

echo "Start updating the MQTT host IP in the database..."

# Get the MQTT host address. If the environment variable is not set, try to automatically detect it.
if [ -z "$MQTT_HOST" ]; then
    echo "MQTT_HOST环境变量未设置，尝试自动检测宿主机IP..."
    
    # Check whether you are in Linux environment
    PLATFORM=$(uname)
    
    if [[ "$PLATFORM" == "Linux" ]]; then
        echo "Linux environment detected"
        
        # On Linux, host.docker.internal may not be available
        # We need to obtain the IP address of the host machine
        
        # Method 1: Try to use docker network inspect to get the gateway IP
        if command -v docker &> /dev/null; then
            echo "Detecting Docker network..."
            # Find Kind Network
            KIND_NETWORK=$(docker network ls | grep kind | awk '{print $1}' | head -n 1)
            
            if [ -n "$KIND_NETWORK" ]; then
                echo "Found Kind network: $KIND_NETWORK"
                # Get the network gateway IP
                HOST_IP=$(docker network inspect $KIND_NETWORK | grep Gateway | head -1 | awk -F'"' '{print $4}')
                
                if [ -n "$HOST_IP" ]; then
                    echo "Obtained host IP from Docker network: $HOST_IP"
                else
                    echo "Unable to get gateway IP from Docker network"
                fi
            else
                echo "Kind network not found"
            fi
        fi

        # Method 2: If Method 1 fails, try to use the routing table
        if [ -z "$HOST_IP" ]; then
            echo "Using routing table to detect host IP..."
            HOST_IP=$(ip route | grep default | awk '{print $3}' | head -n 1)
            
            if [ -n "$HOST_IP" ]; then
                echo "Obtained host IP from routing table: $HOST_IP"
            else
                echo "Unable to get default gateway from routing table"
            fi
        fi

        # Method 3: If all else fails, try using 172.17.0.1 (Docker default bridge gateway)
        if [ -z "$HOST_IP" ]; then
            echo "Trying Docker default bridge gateway: 172.17.0.1"
            HOST_IP="172.17.0.1"
            echo "Using default Docker bridge gateway: $HOST_IP"
        fi
    else
        # macOS and other environments use host.docker.internal
        HOST_IP="host.docker.internal"
        echo "For non-Linux environments, use the default value: host.docker.internal"
    fi
    
    # Set the MQTT host address
    MQTT_HOST=$HOST_IP
else
    echo "Use the MQTT host address set in the environment variable: $MQTT_HOST"
fi

echo "The MQTT host address will be used: $MQTT_HOST"

# Waiting for the PostgreSQL Pod to start
echo "Waiting for the PostgreSQL Pod to start..."
kubectl wait --for=condition=ready pod -l app=postgres --timeout=120s -n listener-operator-system

# Get the PostgreSQL Pod name
POSTGRES_POD=$(kubectl get pod -l app=postgres -o jsonpath="{.items[0].metadata.name}" -n listener-operator-system)
echo "Found PostgreSQL Pod: $POSTGRES_POD"

# Update the MQTT host address in the database
echo "Updating the MQTT host address in the database..."

kubectl exec -it $POSTGRES_POD -n listener-operator-system -- psql -U postgres -d iot_dataflow_manager << EOF

-- Update the MQTT host address in the iot_flow table
UPDATE iot_flow 
SET nodes = jsonb_set(nodes, '{0,data,source,config,host}', '"$MQTT_HOST"', false)
WHERE jsonb_path_exists(nodes, '\$[*] ? (@.data.source.type == "MQTT")');

-- Verify the update result
SELECT id, name, nodes FROM iot_flow WHERE id = 1;

\q
EOF

echo "Database update completed"

# Update the MQTT host address in the ConfigMap
echo "Updating the MQTT host address in the ConfigMap..."
kubectl patch configmap mqtt-config -n listener-operator-system -p "{\"data\":{\"MQTT_BROKER_HOST\":\"$MQTT_HOST\"}}"

echo "MQTT configuration update completed"
echo "New MQTT host address: $MQTT_HOST"