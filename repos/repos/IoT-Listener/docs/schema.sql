-- 1. 创建枚举类型
CREATE TYPE "iot_field_type" AS ENUM ('BOOLEAN', 'NUMBER', 'STRING');

CREATE TYPE "iot_unit" AS ENUM (
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

CREATE TYPE "iot_source_type" AS ENUM ('HTTP', 'MAS-MONITOR', 'MQTT');

-- 2. 创建 site 表
CREATE TABLE IF NOT EXISTS "site" (
    "id" SERIAL NOT NULL PRIMARY KEY,
    "name" TEXT NOT NULL UNIQUE,
    "description" TEXT NOT NULL,
    "location" JSONB NOT NULL,
    "public" BOOLEAN NOT NULL,
    "initial_view" JSONB NOT NULL,
    "created_at" TIMESTAMP NOT NULL,
    "updated_at" TIMESTAMP NOT NULL,
    "projection_string" TEXT NOT NULL
);

-- 3. 创建 iot_flow 表
CREATE TABLE IF NOT EXISTS "iot_flow" (
    "id" SERIAL NOT NULL PRIMARY KEY,
    "site_id" INTEGER NOT NULL,
    "name" TEXT NOT NULL UNIQUE,
    "nodes" JSONB NOT NULL,
    "edges" JSONB NOT NULL,
    "created_at" TIMESTAMP NOT NULL,
    "updated_at" TIMESTAMP NOT NULL,
    CONSTRAINT "fk_iot_flow_site_id" 
        FOREIGN KEY ("site_id") 
        REFERENCES "site" ("id") 
        ON DELETE NO ACTION 
        ON UPDATE NO ACTION
);

-- 4. 创建 sensor 表
CREATE TABLE IF NOT EXISTS "sensor" (
    "id" SERIAL NOT NULL PRIMARY KEY,
    "flow_id" INTEGER,
    "identifier" TEXT NOT NULL,
    "measuring" TEXT NOT NULL,
    "unit" iot_unit NOT NULL,
    "value_type" iot_field_type NOT NULL,
    "site_id" INTEGER NOT NULL,
    CONSTRAINT "fk_sensor_flow_id" 
        FOREIGN KEY ("flow_id") 
        REFERENCES "iot_flow" ("id") 
        ON DELETE SET NULL 
        ON UPDATE NO ACTION,
    CONSTRAINT "fk_sensor_site_id" 
        FOREIGN KEY ("site_id") 
        REFERENCES "site" ("id") 
        ON DELETE CASCADE 
        ON UPDATE NO ACTION
);

-- 5. 创建 reading 表
CREATE TABLE IF NOT EXISTS "reading" (
    "id" SERIAL NOT NULL,
    "sensor_id" INTEGER NOT NULL,
    "timestamp" TIMESTAMP NOT NULL,
    "raw_value" TEXT NOT NULL,
    "value" DOUBLE PRECISION,
    CONSTRAINT "fk_reading_sensor_id" 
        FOREIGN KEY ("sensor_id") 
        REFERENCES "sensor" ("id") 
        ON DELETE CASCADE 
        ON UPDATE NO ACTION
);