-- update_sensor_constraint.sql
-- Purpose: Add a composite unique constraint to the "sensor" table to support ON CONFLICT operations in Rust code.
--
-- This constraint will ensure that the combination of (identifier, measuring) is unique.

BEGIN;

-- Add a UNIQUE constraint on the (identifier, measuring) columns.
-- We give it a clear name "sensor_identifier_measuring_key".
ALTER TABLE "sensor"
ADD CONSTRAINT "sensor_identifier_measuring_key" UNIQUE ("identifier", "measuring");

COMMIT;