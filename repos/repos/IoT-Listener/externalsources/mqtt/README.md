# External MQTT Source

This directory contains all the necessary files to set up an external MQTT source for testing the IoT Listener.

## Directory Structure

```
externalsources/mqtt/
├── docker-compose.yml     # Docker Compose configuration
├── start-mqtt.sh          # Script to start the MQTT broker
├── stop-mqtt.sh           # Script to stop the MQTT broker
├── publish-test-messages.sh # Script to publish test messages
└── mosquitto/
    └── config/
        └── mosquitto.conf # Mosquitto configuration file
```

## Prerequisites

- Docker and Docker Compose installed
- mosquitto-clients (for publishing/subscribing to messages)

## Quick Start

1. Start the MQTT broker:
   ```bash
   ./start-mqtt.sh
   ```

2. In another terminal, configure the environment variables and start the IoT collector:
   ```bash
   # Set the environment variables (already included in .env file)
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

   # Build and run the collector
   cargo build
   ./target/debug/iot-collector
   ```

3. Publish test messages:
   ```bash
   ./publish-test-messages.sh
   ```

4. Stop the MQTT broker when done:
   ```bash
   ./stop-mqtt.sh
   ```

## Manual Usage

You can also manually control the MQTT broker using Docker Compose:

```bash
# Start the broker
docker-compose up -d

# Stop the broker
docker-compose down

# View logs
docker-compose logs mosquitto
```

## Publishing Custom Messages

You can publish custom messages using the `mosquitto_pub` command:

```bash
mosquitto_pub -h localhost -p 1883 -t "your/topic" -m "your message"
```

## Subscribing to Messages

To subscribe to messages from a topic:

```bash
mosquitto_sub -h localhost -p 1883 -t "your/topic"
```

## Configuration

The MQTT broker is configured to:
- Listen on port 1883 for MQTT connections
- Allow anonymous connections
- Persist data to the `mosquitto/data` directory
- Log to the `mosquitto/log` directory

You can modify the `mosquitto/config/mosquitto.conf` file to change these settings.