# IoT-listener
## To generate entities from the database
Set environment variable `DATABASE_URL` and then run:
`sea-orm-cli generate entity -o src/entities --enum-extra-attributes 'serde(rename_all="SCREAMING_SNAKE_CASE")' --with-serde both --ignore-tables seaql_migrations,area,databasechangelog,databasechangeloglock,infohub_page,infohub_page_ownership,iot_deployment,navigation_edge,navigation_point,poi,poi_group,poi_sensor_link,point_cloud`
## Data Sources
The IoT Listener supports multiple data sources:
### MQTT Source
Configuration example:
### MQTT Source
Configuration example:

```json
{
  "id": "mqtt-source-1",
  "type": "source",
  "data": {
    "type": "MQTT",
    "config": {
      "topic": "sensor/temperature"
    }
  }
}
```

### MAS Monitor Source
Configuration example:
```json
{
  "id": "mas-monitor-source-1",
  "type": "source",
  "data": {
    "type": "MAS-MONITOR",
    "config": {
      "deviceTypeId": "temperature-sensor",
      "deviceName": "sensor-001",
      "interval": 30
    }
  }
}
```

### Kafka Source
Configuration example:
```json
{
  "id": "kafka-source-1",
  "type": "source",
  "data": {
    "type": "KAFKA",
    "config": {
      "bootstrapServers": "localhost:9092",
      "topic": "iot-sensor-data",
      "groupId": "iot-listener-group",
      "pollInterval": 5
    }
  }
}
```

The Kafka source will consume messages from the specified topic and make them available to the data processing graph. Messages are expected to be JSON formatted.

Configuration parameters:

- `bootstrapServers`: Comma-separated list of Kafka brokers (e.g., "localhost:9092" or "host1:9092,host2:9092")
- `topic`: The Kafka topic to consume messages from
- `groupId`: The consumer group ID
- `pollInterval`: Polling interval in seconds (default: 5)


