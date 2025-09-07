#!/usr/bin/env python3

from http.server import HTTPServer, BaseHTTPRequestHandler
import json
import random
from datetime import datetime
import time

class SensorDataHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == '/sensor-data':
            # Generate random sensor data
            data = {
                "temperature": round(random.uniform(20.0, 30.0), 2),
                "humidity": round(random.uniform(40.0, 60.0), 2),
                "pressure": round(random.uniform(1000.0, 1020.0), 2),
                "timestamp": datetime.now().isoformat()
            }
            
            self.send_response(200)
            self.send_header('Content-type', 'application/json')
            self.send_header('Access-Control-Allow-Origin', '*')
            self.end_headers()
            
            response = json.dumps(data).encode('utf-8')
            self.wfile.write(response)
        elif self.path == '/':
            self.send_response(200)
            self.send_header('Content-type', 'text/html')
            self.end_headers()
            
            html = """
            <!DOCTYPE html>
            <html>
            <head>
                <title>IoT HTTP Server</title>
            </head>
            <body>
                <h1>IoT HTTP Server</h1>
                <p>This server provides sensor data for testing the IoT Listener.</p>
                <p>Endpoint: <a href="/sensor-data">/sensor-data</a></p>
            </body>
            </html>
            """
            self.wfile.write(html.encode('utf-8'))
        else:
            self.send_response(404)
            self.end_headers()
            
    def do_POST(self):
        if self.path == '/sensor-data':
            # For POST requests, just return the same sensor data
            self.do_GET()
        else:
            self.send_response(404)
            self.end_headers()

def run_server(port=8080):
    server_address = ('', port)
    httpd = HTTPServer(server_address, SensorDataHandler)
    print(f"Starting HTTP server on port {port}...")
    print(f"Sensor data endpoint: http://localhost:{port}/sensor-data")
    print("Press Ctrl+C to stop the server")
    
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\nShutting down server...")
        httpd.shutdown()

if __name__ == '__main__':
    run_server()
    