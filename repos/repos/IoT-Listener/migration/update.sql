-- update_sensor_constraint.sql
-- 目的：为 "sensor" 表添加一个复合唯一约束，以支持 Rust 代码中的 ON CONFLICT 操作。
--
-- 这个约束将确保 (identifier, measuring) 的组合是唯一的。

BEGIN;

-- 在 (identifier, measuring) 列上添加一个 UNIQUE 约束。
-- 我们给它一个明确的名字 "sensor_identifier_measuring_key"。
ALTER TABLE "sensor"
ADD CONSTRAINT "sensor_identifier_measuring_key" UNIQUE ("identifier", "measuring");

COMMIT;