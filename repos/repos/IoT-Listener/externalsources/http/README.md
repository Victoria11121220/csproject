# External HTTP Source

This directory contains all the necessary files to set up an external HTTP source for testing the IoT Listener.

## Directory Structure

```
externalsources/http/
├── server.py          # Python HTTP server that generates sensor data
├── start-http.sh      # Script to start the HTTP server
└── README.md          # This file
```

## Prerequisites

- Python 3.x installed

## Quick Start

1. Start the HTTP server:
   ```bash
   ./start-http.sh
   ```

2. The server will start on port 8080 and provide sensor data at:
   ```
   http://localhost:8080/sensor-data
   ```

## HTTP Endpoint

The server provides a JSON endpoint that returns random sensor data:

```
GET /sensor-data
```

Sample response:
```json
{
  "temperature": 25.67,
  "humidity": 48.23,
  "pressure": 1015.78,
  "timestamp": "2025-09-08T10:30:45.123456"
}
```

## Using with IoT Listener

To use this HTTP source with the IoT Listener, configure a source node with the following configuration:

```json
{
  "id": "http-source-1",
  "type": "source",
  "data": {
    "source": {
      "type": "HTTP",
      "config": {
        "endpoint": "http://localhost:8080/sensor-data",
        "method": "GET",
        "interval": 5
      }
    },
    "config": null
  }
}
```

This configuration will:
- Poll the HTTP endpoint every 5 seconds
- Send a GET request to retrieve sensor data
- Forward the data to the IoT Listener pipeline

## Customization

You can modify the `server.py` file to:
- Change the sensor data format
- Add more sensor types
- Adjust the data generation logic
- Add authentication or other HTTP features