#!/bin/bash

# Script to test MQTT host address replacement

echo "Testing MQTT host address replacement functionality"

# Test default value
echo "Test 1: Using default value"
unset MQTT_HOST
DEFAULT_HOST=${MQTT_HOST:-host.docker.internal}
echo "Default MQTT host address: $DEFAULT_HOST"

# Test environment variable setting
echo -e "\nTest 2: Using environment variable"
export MQTT_HOST=192.168.1.100
VARIABLE_HOST=${MQTT_HOST:-host.docker.internal}
echo "Environment variable MQTT host address: $VARIABLE_HOST"

# Test actual replacement effect in script
echo -e "\nTest 3: Actual replacement effect test"
TEST_MQTT_HOST=${MQTT_HOST:-host.docker.internal}
echo "Testing SQL snippets:"
echo "DO \$\$
DECLARE
    mqtt_host TEXT := '$TEST_MQTT_HOST';
BEGIN
    INSERT INTO iot_flow (id, site_id, name, nodes, edges) 
    VALUES (1, 1, 'Initial Flow', 
            '[{\"id\": \"mqtt_source_1\", \"type\": \"source\", \"data\": {\"source\": {\"type\": \"MQTT\", \"config\": {\"host\": \"' || mqtt_host || '\", \"port\": 1883}}, \"config\": {\"topic\": \"sensors/temperature\"}}}, {\"id\":\"debug-1\",\"type\":\"debug\",\"data\":{}}]', 
            '[{\"source\":\"mqtt_source_1\",\"target\":\"debug-1\",\"sourceHandle\":\"source\",\"targetHandle\":\"target\"}]')
    ON CONFLICT (id) DO NOTHING;
END \$\$;"

echo -e "\nTesting Completed"