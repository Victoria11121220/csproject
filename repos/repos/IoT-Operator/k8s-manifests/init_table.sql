-- PostgreSQL database table structure initialization and update script
-- This script contains a complete table structure definition and supports initialization and update operations.

-- Create custom types if not exists
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'iot_field_type') THEN
        CREATE TYPE public.iot_field_type AS ENUM (
            'BOOLEAN',
            'NUMBER',
            'STRING'
        );
    END IF;
END $$;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'iot_source_type') THEN
        CREATE TYPE public.iot_source_type AS ENUM (
            'HTTP',
            'MAS-MONITOR',
            'MQTT'
        );
    END IF;
END $$;

DO $$ 
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
END $$;

-- Create tables if not exists
-- debug_messages Table
CREATE TABLE IF NOT EXISTS public.debug_messages (
    id integer NOT NULL,
    flow_id integer NOT NULL,
    debug_node_id text NOT NULL,
    message text NOT NULL,
    "timestamp" timestamp with time zone DEFAULT now() NOT NULL
);

-- debug_messages_id_seq Sequence
CREATE SEQUENCE IF NOT EXISTS public.debug_messages_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

-- flow_error Table
CREATE TABLE IF NOT EXISTS public.flow_error (
    id integer NOT NULL,
    flow_id integer NOT NULL,
    node_id text NOT NULL,
    message text NOT NULL,
    "timestamp" timestamp with time zone DEFAULT now()
);

-- flow_error_id_seq Sequence
CREATE SEQUENCE IF NOT EXISTS public.flow_error_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

-- iot_deployment Table
CREATE TABLE IF NOT EXISTS public.iot_deployment (
    id integer NOT NULL,
    flow_id integer NOT NULL,
    deployed_at timestamp with time zone DEFAULT now() NOT NULL,
    nodes jsonb NOT NULL,
    edges jsonb NOT NULL
);

-- iot_deployment_id_seq Sequence
CREATE SEQUENCE IF NOT EXISTS public.iot_deployment_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

-- iot_flow Table
CREATE TABLE IF NOT EXISTS public.iot_flow (
    id integer NOT NULL,
    site_id integer NOT NULL,
    name text NOT NULL,
    nodes jsonb NOT NULL,
    edges jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

-- iot_flow_debug_message Table
CREATE TABLE IF NOT EXISTS public.iot_flow_debug_message (
    id integer NOT NULL,
    flow_id integer NOT NULL,
    debug_node_id text NOT NULL,
    message text NOT NULL,
    "timestamp" timestamp with time zone DEFAULT now() NOT NULL
);

-- iot_flow_debug_message_id_seq Sequence
CREATE SEQUENCE IF NOT EXISTS public.iot_flow_debug_message_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

-- iot_flow_id_seq Sequence
CREATE SEQUENCE IF NOT EXISTS public.iot_flow_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

-- iot_source Table
CREATE TABLE IF NOT EXISTS public.iot_source (
    id integer NOT NULL,
    site_id integer NOT NULL,
    name text NOT NULL,
    description text,
    type public.iot_source_type NOT NULL,
    config jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

-- iot_source_id_seq Sequence
CREATE SEQUENCE IF NOT EXISTS public.iot_source_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

-- reading Table
CREATE TABLE IF NOT EXISTS public.reading (
    id integer NOT NULL,
    sensor_id integer NOT NULL,
    "timestamp" timestamp with time zone NOT NULL,
    raw_value text NOT NULL,
    value double precision
);

-- reading_id_seq Sequence
CREATE SEQUENCE IF NOT EXISTS public.reading_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

-- sensor Table
CREATE TABLE IF NOT EXISTS public.sensor (
    id integer NOT NULL,
    flow_id integer,
    identifier text NOT NULL,
    measuring text NOT NULL,
    unit public.iot_unit NOT NULL,
    value_type public.iot_field_type NOT NULL
);

-- sensor_id_seq Sequence
CREATE SEQUENCE IF NOT EXISTS public.sensor_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

-- sensor_metadata Table
CREATE TABLE IF NOT EXISTS public.sensor_metadata (
    id integer NOT NULL,
    sensor_id integer NOT NULL,
    key text NOT NULL,
    value text NOT NULL
);

-- sensor_metadata_id_seq Sequence
CREATE SEQUENCE IF NOT EXISTS public.sensor_metadata_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

-- sensors Table
CREATE TABLE IF NOT EXISTS public.sensors (
    id integer NOT NULL,
    flow_id integer,
    identifier text NOT NULL,
    measuring text NOT NULL,
    unit public.iot_unit NOT NULL,
    value_type public.iot_field_type NOT NULL
);

-- sensors_id_seq Sequence
CREATE SEQUENCE IF NOT EXISTS public.sensors_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

-- site Table
CREATE TABLE IF NOT EXISTS public.site (
    id integer NOT NULL,
    name text NOT NULL,
    description text NOT NULL,
    location jsonb NOT NULL,
    public boolean NOT NULL,
    initial_view jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

-- site_id_seq Sequence
CREATE SEQUENCE IF NOT EXISTS public.site_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

-- Setting Sequence Ownership
ALTER SEQUENCE IF EXISTS public.debug_messages_id_seq OWNED BY public.debug_messages.id;
ALTER SEQUENCE IF EXISTS public.flow_error_id_seq OWNED BY public.flow_error.id;
ALTER SEQUENCE IF EXISTS public.iot_deployment_id_seq OWNED BY public.iot_deployment.id;
ALTER SEQUENCE IF EXISTS public.iot_flow_debug_message_id_seq OWNED BY public.iot_flow_debug_message.id;
ALTER SEQUENCE IF EXISTS public.iot_flow_id_seq OWNED BY public.iot_flow.id;
ALTER SEQUENCE IF EXISTS public.iot_source_id_seq OWNED BY public.iot_source.id;
ALTER SEQUENCE IF EXISTS public.reading_id_seq OWNED BY public.reading.id;
ALTER SEQUENCE IF EXISTS public.sensor_id_seq OWNED BY public.sensor.id;
ALTER SEQUENCE IF EXISTS public.sensor_metadata_id_seq OWNED BY public.sensor_metadata.id;
ALTER SEQUENCE IF EXISTS public.sensors_id_seq OWNED BY public.sensors.id;
ALTER SEQUENCE IF EXISTS public.site_id_seq OWNED BY public.site.id;

-- Setting Default Values
ALTER TABLE ONLY public.debug_messages ALTER COLUMN id SET DEFAULT nextval('public.debug_messages_id_seq'::regclass);
ALTER TABLE ONLY public.flow_error ALTER COLUMN id SET DEFAULT nextval('public.flow_error_id_seq'::regclass);
ALTER TABLE ONLY public.iot_deployment ALTER COLUMN id SET DEFAULT nextval('public.iot_deployment_id_seq'::regclass);
ALTER TABLE ONLY public.iot_flow ALTER COLUMN id SET DEFAULT nextval('public.iot_flow_id_seq'::regclass);
ALTER TABLE ONLY public.iot_flow_debug_message ALTER COLUMN id SET DEFAULT nextval('public.iot_flow_debug_message_id_seq'::regclass);
ALTER TABLE ONLY public.iot_source ALTER COLUMN id SET DEFAULT nextval('public.iot_source_id_seq'::regclass);
ALTER TABLE ONLY public.reading ALTER COLUMN id SET DEFAULT nextval('public.reading_id_seq'::regclass);
ALTER TABLE ONLY public.sensor ALTER COLUMN id SET DEFAULT nextval('public.sensor_id_seq'::regclass);
ALTER TABLE ONLY public.sensor_metadata ALTER COLUMN id SET DEFAULT nextval('public.sensor_metadata_id_seq'::regclass);
ALTER TABLE ONLY public.sensors ALTER COLUMN id SET DEFAULT nextval('public.sensors_id_seq'::regclass);
ALTER TABLE ONLY public.site ALTER COLUMN id SET DEFAULT nextval('public.site_id_seq'::regclass);

-- Create the index if it does not exist
CREATE INDEX IF NOT EXISTS idx_reading_sensor_id ON public.reading USING btree (sensor_id);
CREATE INDEX IF NOT EXISTS idx_reading_timestamp ON public.reading USING btree ("timestamp");
CREATE INDEX IF NOT EXISTS idx_sensor_flow_id ON public.sensor USING btree (flow_id);
CREATE INDEX IF NOT EXISTS idx_iot_flow_site_id ON public.iot_flow USING btree (site_id);
CREATE INDEX IF NOT EXISTS idx_iot_flow_created_at ON public.iot_flow USING btree (created_at);

-- Add a primary key constraint if one does not exist
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'debug_messages_pkey') THEN
        ALTER TABLE ONLY public.debug_messages
            ADD CONSTRAINT debug_messages_pkey PRIMARY KEY (id);
    END IF;
END $$;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'flow_error_pkey') THEN
        ALTER TABLE ONLY public.flow_error
            ADD CONSTRAINT flow_error_pkey PRIMARY KEY (id);
    END IF;
END $$;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'iot_deployment_pkey') THEN
        ALTER TABLE ONLY public.iot_deployment
            ADD CONSTRAINT iot_deployment_pkey PRIMARY KEY (id);
    END IF;
END $$;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'iot_deployment_flow_id_key') THEN
        ALTER TABLE ONLY public.iot_deployment
            ADD CONSTRAINT iot_deployment_flow_id_key UNIQUE (flow_id);
    END IF;
END $$;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'iot_flow_debug_message_pkey') THEN
        ALTER TABLE ONLY public.iot_flow_debug_message
            ADD CONSTRAINT iot_flow_debug_message_pkey PRIMARY KEY (id);
    END IF;
END $$;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'iot_flow_pkey') THEN
        ALTER TABLE ONLY public.iot_flow
            ADD CONSTRAINT iot_flow_pkey PRIMARY KEY (id);
    END IF;
END $$;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'iot_flow_name_key') THEN
        ALTER TABLE ONLY public.iot_flow
            ADD CONSTRAINT iot_flow_name_key UNIQUE (name);
    END IF;
END $$;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'iot_source_pkey') THEN
        ALTER TABLE ONLY public.iot_source
            ADD CONSTRAINT iot_source_pkey PRIMARY KEY (id);
    END IF;
END $$;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'reading_pkey') THEN
        ALTER TABLE ONLY public.reading
            ADD CONSTRAINT reading_pkey PRIMARY KEY (id);
    END IF;
END $$;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'reading_sensor_id_timestamp_key') THEN
        ALTER TABLE ONLY public.reading
            ADD CONSTRAINT reading_sensor_id_timestamp_key UNIQUE (sensor_id, "timestamp");
    END IF;
END $$;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'sensor_pkey') THEN
        ALTER TABLE ONLY public.sensor
            ADD CONSTRAINT sensor_pkey PRIMARY KEY (id);
    END IF;
END $$;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'sensor_identifier_measuring_key') THEN
        ALTER TABLE ONLY public.sensor
            ADD CONSTRAINT sensor_identifier_measuring_key UNIQUE (identifier, measuring);
    END IF;
END $$;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'sensor_metadata_pkey') THEN
        ALTER TABLE ONLY public.sensor_metadata
            ADD CONSTRAINT sensor_metadata_pkey PRIMARY KEY (id);
    END IF;
END $$;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'sensors_pkey') THEN
        ALTER TABLE ONLY public.sensors
            ADD CONSTRAINT sensors_pkey PRIMARY KEY (id);
    END IF;
END $$;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'site_pkey') THEN
        ALTER TABLE ONLY public.site
            ADD CONSTRAINT site_pkey PRIMARY KEY (id);
    END IF;
END $$;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'site_name_key') THEN
        ALTER TABLE ONLY public.site
            ADD CONSTRAINT site_name_key UNIQUE (name);
    END IF;
END $$;

-- Add a foreign key constraint if one does not exist
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'flow_error_flow_id_fkey') THEN
        ALTER TABLE ONLY public.flow_error
            ADD CONSTRAINT flow_error_flow_id_fkey FOREIGN KEY (flow_id) REFERENCES public.iot_flow(id) ON DELETE CASCADE;
    END IF;
END $$;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'iot_deployment_flow_id_fkey') THEN
        ALTER TABLE ONLY public.iot_deployment
            ADD CONSTRAINT iot_deployment_flow_id_fkey FOREIGN KEY (flow_id) REFERENCES public.iot_flow(id);
    END IF;
END $$;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'iot_flow_debug_message_flow_id_fkey') THEN
        ALTER TABLE ONLY public.iot_flow_debug_message
            ADD CONSTRAINT iot_flow_debug_message_flow_id_fkey FOREIGN KEY (flow_id) REFERENCES public.iot_flow(id) ON DELETE CASCADE;
    END IF;
END $$;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'iot_flow_site_id_fkey') THEN
        ALTER TABLE ONLY public.iot_flow
            ADD CONSTRAINT iot_flow_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.site(id);
    END IF;
END $$;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'iot_source_site_id_fkey') THEN
        ALTER TABLE ONLY public.iot_source
            ADD CONSTRAINT iot_source_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.site(id);
    END IF;
END $$;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'reading_sensor_id_fkey') THEN
        ALTER TABLE ONLY public.reading
            ADD CONSTRAINT reading_sensor_id_fkey FOREIGN KEY (sensor_id) REFERENCES public.sensor(id) ON DELETE CASCADE;
    END IF;
END $$;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'sensor_flow_id_fkey') THEN
        ALTER TABLE ONLY public.sensor
            ADD CONSTRAINT sensor_flow_id_fkey FOREIGN KEY (flow_id) REFERENCES public.iot_flow(id) ON DELETE SET NULL;
    END IF;
END $$;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'sensor_metadata_sensor_id_fkey') THEN
        ALTER TABLE ONLY public.sensor_metadata
            ADD CONSTRAINT sensor_metadata_sensor_id_fkey FOREIGN KEY (sensor_id) REFERENCES public.sensor(id) ON DELETE CASCADE;
    END IF;
END $$;

-- Insert initial data (use ON CONFLICT DO NOTHING to avoid duplicate inserts)
INSERT INTO public.site (id, name, description, location, public, initial_view) 
VALUES (1, 'Test Site', 'A site for testing purposes', 
        '{"latitude": 40.7128, "longitude": -74.0060}', true, 
        '{"zoom": 12, "center": [40.7128, -74.0060]}')
ON CONFLICT (id) DO NOTHING;

INSERT INTO public.iot_flow (id, site_id, name, nodes, edges) 
VALUES (1, 1, 'Initial Flow', 
        '[{"id": "mqtt_source_1", "type": "source", "data": {"source": {"type": "MQTT", "config": {"host": "localhost", "port": 1883}}, "config": {"topic": "sensors/temperature"}}}, {"id":"debug-1","type":"debug","data":{}}]', 
        '[{"source":"mqtt_source_1","target":"debug-1","sourceHandle":"source","targetHandle":"target"}]')
ON CONFLICT (id) DO NOTHING;

-- Table structure update part
-- Add table structure update statements here, for example:
-- ALTER TABLE public.site ADD COLUMN IF NOT EXISTS timezone TEXT;
-- ALTER TABLE public.site ALTER COLUMN description DROP NOT NULL;

-- Create a function to send notifications
CREATE OR REPLACE FUNCTION notify_iot_flow_insert() RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('iot_flow_insert', NEW.id::text);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create a trigger to send a notification when a new row is inserted into the iot_flow table
DROP TRIGGER IF EXISTS iot_flow_insert_trigger ON iot_flow;
CREATE TRIGGER iot_flow_insert_trigger
    AFTER INSERT ON iot_flow
    FOR EACH ROW
    EXECUTE FUNCTION notify_iot_flow_insert();