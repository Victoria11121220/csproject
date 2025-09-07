# IoT-listener

## Quick Start with MQTT Source

To quickly start the IoT listener with an MQTT source, you can use the provided scripts:

1. Start the complete environment (MQTT broker + collector):
   ```bash
   ./start-environment.sh
   ```

2. In another terminal, publish test MQTT messages:
   ```bash
   ./publish-test-messages.sh
   ```

3. To stop the environment:
   ```bash
   ./stop-environment.sh
   ```

## External MQTT Source Setup

For a more comprehensive MQTT testing environment, check out the `externalsources/mqtt` directory:

1. Navigate to the MQTT directory:
   ```bash
   cd externalsources/mqtt
   ```

2. Start the MQTT broker:
   ```bash
   ./start-mqtt.sh
   ```

3. Publish test messages:
   ```bash
   ./publish-test-messages.sh
   ```

4. To stop the MQTT broker:
   ```bash
   ./stop-mqtt.sh
   ```

## External HTTP Source Setup

For testing with HTTP sources, check out the `externalsources/http` directory:

1. Navigate to the HTTP directory:
   ```bash
   cd externalsources/http
   ```

2. Start the HTTP server:
   ```bash
   ./start-http.sh
   ```

3. The server will provide sensor data at:
   ```
   http://localhost:8080/sensor-data
   ```

## Manual MQTT Broker Setup

If you prefer to set up the MQTT broker manually:

1. Start MQTT broker using Docker:
   ```bash
   docker run -d --name mosquitto -p 1883:1883 -p 9001:9001 eclipse-mosquitto:latest
   ```

2. Set environment variables for the MQTT source:
   ```bash
   export nodes='[{
     "id": "mqtt_source_1",
     "type": "source",
     "data": {
       "source": {
         "type": "MQTT",
         "config": {
           "host": "localhost",
           "port": 1883
         }
       },
       "config": {
         "topic": "sensors/temperature"
       }
     }
   }]'
   export edges='[]'
   export flow_id='1'
   ```

3. Build and run the collector:
   ```bash
   cargo build
   ./target/debug/iot-collector
   ```

4. Publish test messages:
   ```bash
   # Install mosquitto-clients if not already installed
   # On macOS: brew install mosquitto
   # On Ubuntu: sudo apt-get install mosquitto-clients
   
   mosquitto_pub -h localhost -p 1883 -t "sensors/temperature" -m '{"temperature": 22.5, "unit": "celsius"}'
   ```

## To generate entities from the database
Set environment variable `DATABASE_URL` and then run:
`sea-orm-cli generate entity -o src/entities --enum-extra-attributes 'serde(rename_all="SCREAMING_SNAKE_CASE")' --with-serde both --ignore-tables seaql_migrations,area,databasechangelog,databasechangeloglock,infohub_page,infohub_page_ownership,iot_deployment,navigation_edge,navigation_point,poi,poi_group,poi_sensor_link,point_cloud`