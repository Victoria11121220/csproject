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

---

### 🔴 问题出在：你用了错误的符号。

你用了 **3个单引号 `'''`**，这不是 Markdown 的语法。

---

### ✅ 正确写法应该是：**3个反引号（Backtick）** → ` ``` `

这两个长得很像，但是完全不同的字符：

| 符号 | 名称        | 键盘位置（英文输入法）         |
|------|-------------|------------------------------|
| `'`  | 单引号      | 在回车键左边                 |
| <code>\`\`\`</code> | 反引号/Backtick | 通常在 ESC 键下方、数字 1 键的左边 |

---

### ✅ 修正后的完整 Markdown：

```markdown
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

