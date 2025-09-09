#!/bin/bash

# Script to initialize all postgres tables in k8s

# Get MQTT host address, use default if not set
MQTT_HOST=${MQTT_HOST:-host.docker.internal}

echo "Using MQTT host address: $MQTT_HOST"

# Waiting for the PostgreSQL Pod to start
echo "Waiting for PostgreSQL Pod to start..."
kubectl wait --for=condition=ready pod -l app=postgres --timeout=120s

# Get PostgreSQL Pod name
POSTGRES_POD=$(kubectl get pod -l app=postgres -o jsonpath="{.items[0].metadata.name}" -n listener-operator-system)

# Connect to the postgres database in k8s and create tables
kubectl exec -it $POSTGRES_POD -n listener-operator-system -- psql -U postgres -d iot_dataflow_manager << EOF

-- Create custom types
DO \$\$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'iot_field_type') THEN
        CREATE TYPE public.iot_field_type AS ENUM (
            'BOOLEAN',
            'NUMBER',
            'STRING'
        );
    END IF;
END \$\$;

DO \$\$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'iot_source_type') THEN
        CREATE TYPE public.iot_source_type AS ENUM (
            'HTTP',
            'MAS-MONITOR',
            'MQTT'
        );
    END IF;
END \$\$;

DO \$\$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'iot_unit') THEN
        CREATE TYPE public.iot_unit AS ENUM (
            'AMOUNT',
            'AMPERE',
            'BOOLEAN',
            'CUBIC_METER',
            'DEGREES_CELCIUS',
            'METER_PER_SECOND',
            'PERCENT',
            'STRING',
            'VOLT',
            'WATT',
            'WATT_HOUR'
        );
    END IF;
END \$\$;

-- Create site table
CREATE TABLE IF NOT EXISTS site (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    location JSONB NOT NULL,
    public BOOLEAN NOT NULL,
    initial_view JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create iot_flow table
CREATE TABLE IF NOT EXISTS iot_flow (
    id SERIAL PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES site(id),
    name TEXT NOT NULL,
    nodes JSONB NOT NULL,
    edges JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create other related tables
CREATE TABLE IF NOT EXISTS iot_source (
    id SERIAL PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES site(id),
    name TEXT NOT NULL,
    description TEXT,
    type public.iot_source_type NOT NULL,
    config JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS sensor (
    id SERIAL PRIMARY KEY,
    flow_id INTEGER REFERENCES iot_flow(id) ON DELETE SET NULL,
    identifier TEXT NOT NULL,
    measuring TEXT NOT NULL,
    unit public.iot_unit NOT NULL,
    value_type public.iot_field_type NOT NULL
);

CREATE TABLE IF NOT EXISTS reading (
    id SERIAL PRIMARY KEY,
    sensor_id INTEGER NOT NULL REFERENCES sensor(id) ON DELETE CASCADE,
    value NUMERIC,
    raw_value text NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS sensors (
    id SERIAL PRIMARY KEY,
    flow_id INTEGER REFERENCES iot_flow(id) ON DELETE SET NULL,
    identifier TEXT NOT NULL,
    measuring TEXT NOT NULL,
    unit public.iot_unit NOT NULL,
    value_type public.iot_field_type NOT NULL
);

CREATE TABLE IF NOT EXISTS sensor_metadata (
    id SERIAL PRIMARY KEY,
    sensor_id INTEGER NOT NULL REFERENCES sensors(id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS iot_deployment (
    id SERIAL PRIMARY KEY,
    flow_id INTEGER NOT NULL REFERENCES iot_flow(id),
    deployed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    nodes JSONB NOT NULL,
    edges JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS flow_error (
    id SERIAL PRIMARY KEY,
    flow_id INTEGER NOT NULL REFERENCES iot_flow(id) ON DELETE CASCADE,
    node_id TEXT NOT NULL,
    message TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS debug_messages (
    id SERIAL PRIMARY KEY,
    flow_id INTEGER REFERENCES iot_flow(id) ON DELETE CASCADE,
    debug_node_id TEXT NOT NULL,
    message TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS iot_flow_debug_message (
    id SERIAL PRIMARY KEY,
    flow_id INTEGER NOT NULL REFERENCES iot_flow(id) ON DELETE CASCADE,
    debug_node_id TEXT NOT NULL,
    message TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create a function to send notifications
CREATE OR REPLACE FUNCTION notify_iot_flow_insert() RETURNS TRIGGER AS \$\$
BEGIN
    PERFORM pg_notify('iot_flow_insert', NEW.id::text);
    RETURN NEW;
END;
\$\$ LANGUAGE plpgsql;

-- Create a trigger to send a notification when a new row is inserted into the iot_flow table
DROP TRIGGER IF EXISTS iot_flow_insert_trigger ON iot_flow;
CREATE TRIGGER iot_flow_insert_trigger
    AFTER INSERT ON iot_flow
    FOR EACH ROW
    EXECUTE FUNCTION notify_iot_flow_insert();

-- Add a primary key constraint if one does not exist
DO \$\$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'iot_flow_name_key') THEN
        ALTER TABLE ONLY public.iot_flow
            ADD CONSTRAINT iot_flow_name_key UNIQUE (name);
    END IF;
END \$\$;

-- Insert initial site data
INSERT INTO site (id, name, description, location, public, initial_view) 
VALUES (1, 'Test Site', 'A site for testing purposes', 
        '{"latitude": 40.7128, "longitude": -74.0060}', true, 
        '{"zoom": 12, "center": [40.7128, -74.0060]}')
ON CONFLICT (id) DO NOTHING;

-- Insert a line from dotenv-listener/.env as the initial line of the iot_flow table
-- Set the MQTT host address using environment variables or default values
INSERT INTO iot_flow (id, site_id, name, nodes, edges) 
VALUES (1, 1, 'Initial Flow', 
        ('[{"id": "mqtt_source_1", "type": "source", "data": {"source": {"type": "MQTT", "config": {"host": "' || '${MQTT_HOST}' || '", "port": 1883}}, "config": {"topic": "sensors/temperature"}}}, {"id":"debug-1","type":"debug","data":{}}]')::jsonb, 
        ('[{"source":"mqtt_source_1","target":"debug-1","sourceHandle":"source","targetHandle":"target"}]')::jsonb)
ON CONFLICT (id) DO NOTHING;

EOF