# test_server.py
from flask import Flask, jsonify
import random
from datetime import datetime  # 1. Import the datetime module

app = Flask(__name__)


@app.route('/sensor-data')
def sensor_data():
    # 2. Get the current UTC time
    now_utc = datetime.now(datetime.timezone.utc)

    # 3. Formatted as an ISO 8601 string and ending with 'Z'
    # .replace(microsecond=0) is to remove milliseconds to make the timestamp neater
    timestamp_str = now_utc.replace(microsecond=0).isoformat() + "Z"

    return jsonify({
        "temperature": random.uniform(20, 30),
        "humidity": random.uniform(40, 70),
        "timestamp": timestamp_str  # 4. Using dynamically generated timestamps
    })


if __name__ == '__main__':
    app.run(port=8080)