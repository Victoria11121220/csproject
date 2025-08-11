-- 第一步：插入基础数据（site和iot_flow）
-- Site表测试数据
INSERT INTO "site" (
    "name", 
    "description", 
    "location", 
    "public", 
    "initial_view", 
    "created_at", 
    "updated_at", 
    "projection_string"
) VALUES (
    'B15 Building',
    'Main building with IoT sensors for temperature monitoring',
    '{"latitude": 59.9139, "longitude": 10.7522, "address": "B15 Building, Oslo, Norway"}',
    true,
    '{"zoom": 15, "center": [10.7522, 59.9139]}',
    NOW(),
    NOW(),
    '+proj=utm +zone=32 +datum=WGS84 +units=m +no_defs'
);

-- Iot_flow表测试数据
INSERT INTO "iot_flow" (
    "site_id",
    "name",
    "nodes",
    "edges",
    "created_at",
    "updated_at"
) VALUES (
    1,
    'Temperature Average Calculation Flow',
    '[{"id":"datalake-0a68d69c-39d3-4e5e-86cf-2c269bb8b207","data":{"sensorId":1},"type":"datalake","dragging":false,"measured":{"width":150,"height":50},"position":{"x":441,"y":203},"selected":false},{"id":"average-fac93473-b247-4fb1-a3c0-6989f2456010","type":"average","dragging":false,"measured":{"width":150,"height":33},"position":{"x":895,"y":360},"selected":false},{"id":"sensor-116184a2-c5a5-4ea1-afe0-3dfe0821e857","data":{"unit":"DEGREES_CELCIUS","value":"NUMBER","measuring":"temperature","timestamp":"%+","identifier":"B15Average"},"type":"sensor","dragging":false,"measured":{"width":150,"height":150},"position":{"x":1147,"y":314},"selected":true},{"id":"current_timestamp-7eb96e0d-ee29-4592-b14b-83ca66d4eec5","type":"current_timestamp","dragging":false,"measured":{"width":200,"height":33},"position":{"x":846,"y":413},"selected":false},{"id":"debug-5faa7851-fdd1-4963-994b-d25a0d28da90","type":"debug","dragging":false,"measured":{"width":150,"height":33},"position":{"x":1354.6085426882833,"y":88.38249017504229},"selected":false},{"id":"datalake-ea102020-e855-4c74-af3c-80f609f1e69b","data":{"sensorId":58792},"type":"datalake","dragging":false,"measured":{"width":215,"height":50},"position":{"x":440,"y":276},"selected":false},{"id":"datalake-169a5b88-b412-4a19-8a05-b75476b95b68","data":{"sensorId":218},"type":"datalake","dragging":false,"measured":{"width":216,"height":50},"position":{"x":438,"y":341},"selected":false},{"id":"datalake-3612ba28-dab5-49ab-98c4-17792afd690a","data":{"sensorId":30221},"type":"datalake","dragging":false,"measured":{"width":204,"height":50},"position":{"x":437,"y":401},"selected":false},{"id":"datalake-45705d18-a326-49c5-9822-2fc88454d417","data":{"sensorId":30227},"type":"datalake","dragging":false,"measured":{"width":204,"height":50},"position":{"x":434,"y":463},"selected":false},{"id":"datalake-ec224e80-ee2d-43bf-b71d-a3b55f473e9a","data":{"sensorId":30224},"type":"datalake","dragging":false,"measured":{"width":204,"height":50},"position":{"x":435,"y":526},"selected":false},{"id":"datalake-50342d1b-7546-4af1-9762-4f7a565de1d8","data":{"sensorId":30211},"type":"datalake","dragging":false,"measured":{"width":204,"height":50},"position":{"x":436,"y":590},"selected":false},{"id":"datalake-8d926de0-87b1-40c9-85a2-e1e65ce5e93f","data":{"sensorId":30208},"type":"datalake","dragging":false,"measured":{"width":203,"height":50},"position":{"x":438,"y":653},"selected":false},{"id":"metadata-3d771506-83a1-430e-bb31-05d6b6b5ed69","data":{"items":["location","B15"]},"type":"metadata","dragging":false,"measured":{"width":150,"height":58},"position":{"x":897.1492258280683,"y":466.13319647827245},"selected":false}]',
    '[{"id":"xy-edge__datalake-0a68d69c-39d3-4e5e-86cf-2c269bb8b207source-average-fac93473-b247-4fb1-a3c0-6989f2456010target","source":"datalake-0a68d69c-39d3-4e5e-86cf-2c269bb8b207","target":"average-fac93473-b247-4fb1-a3c0-6989f2456010","sourceHandle":"source","targetHandle":"target"},{"id":"xy-edge__average-fac93473-b247-4fb1-a3c0-6989f2456010source-sensor-116184a2-c5a5-4ea1-afe0-3dfe0821e857value","source":"average-fac93473-b247-4fb1-a3c0-6989f2456010","target":"sensor-116184a2-c5a5-4ea1-afe0-3dfe0821e857","sourceHandle":"source","targetHandle":"value"},{"id":"xy-edge__current_timestamp-7eb96e0d-ee29-4592-b14b-83ca66d4eec5source-sensor-116184a2-c5a5-4ea1-afe0-3dfe0821e857timestamp","source":"current_timestamp-7eb96e0d-ee29-4592-b14b-83ca66d4eec5","target":"sensor-116184a2-c5a5-4ea1-afe0-3dfe0821e857","sourceHandle":"source","targetHandle":"timestamp"},{"id":"xy-edge__sensor-116184a2-c5a5-4ea1-afe0-3dfe0821e857source-debug-5faa7851-fdd1-4963-994b-d25a0d28da90target","source":"sensor-116184a2-c5a5-4ea1-afe0-3dfe0821e857","target":"debug-5faa7851-fdd1-4963-994b-d25a0d28da90","sourceHandle":"source","targetHandle":"target"},{"id":"xy-edge__datalake-ea102020-e855-4c74-af3c-80f609f1e69bsource-average-fac93473-b247-4fb1-a3c0-6989f2456010target","source":"datalake-ea102020-e855-4c74-af3c-80f609f1e69b","target":"average-fac93473-b247-4fb1-a3c0-6989f2456010","sourceHandle":"source","targetHandle":"target"},{"id":"xy-edge__datalake-169a5b88-b412-4a19-8a05-b75476b95b68source-average-fac93473-b247-4fb1-a3c0-6989f2456010target","source":"datalake-169a5b88-b412-4a19-8a05-b75476b95b68","target":"average-fac93473-b247-4fb1-a3c0-6989f2456010","sourceHandle":"source","targetHandle":"target"},{"id":"xy-edge__datalake-3612ba28-dab5-49ab-98c4-17792afd690asource-average-fac93473-b247-4fb1-a3c0-6989f2456010target","source":"datalake-3612ba28-dab5-49ab-98c4-17792afd690a","target":"average-fac93473-b247-4fb1-a3c0-6989f2456010","sourceHandle":"source","targetHandle":"target"},{"id":"xy-edge__datalake-45705d18-a326-49c5-9822-2fc88454d417source-average-fac93473-b247-4fb1-a3c0-6989f2456010target","source":"datalake-45705d18-a326-49c5-9822-2fc88454d417","target":"average-fac93473-b247-4fb1-a3c0-6989f2456010","sourceHandle":"source","targetHandle":"target"},{"id":"xy-edge__datalake-ec224e80-ee2d-43bf-b71d-a3b55f473e9asource-average-fac93473-b247-4fb1-a3c0-6989f2456010target","source":"datalake-ec224e80-ee2d-43bf-b71d-a3b55f473e9a","target":"average-fac93473-b247-4fb1-a3c0-6989f2456010","sourceHandle":"source","targetHandle":"target"},{"id":"xy-edge__datalake-50342d1b-7546-4af1-9762-4f7a565de1d8source-average-fac93473-b247-4fb1-a3c0-6989f2456010target","source":"datalake-50342d1b-7546-4af1-9762-4f7a565de1d8","target":"average-fac93473-b247-4fb1-a3c0-6989f2456010","sourceHandle":"source","targetHandle":"target"},{"id":"xy-edge__datalake-8d926de0-87b1-40c9-85a2-e1e65ce5e93fsource-average-fac93473-b247-4fb1-a3c0-6989f2456010target","source":"datalake-8d926de0-87b1-40c9-85a2-e1e65ce5e93f","target":"average-fac93473-b247-4fb1-a3c0-6989f2456010","sourceHandle":"source","targetHandle":"target"},{"id":"xy-edge__metadata-3d771506-83a1-430e-bb31-05d6b6b5ed69source-sensor-116184a2-c5a5-4ea1-afe0-3dfe0821e857metadata","source":"metadata-3d771506-83a1-430e-bb31-05d6b6b5ed69","target":"sensor-116184a2-c5a5-4ea1-afe0-3dfe0821e857","sourceHandle":"source","targetHandle":"metadata"}]',
    NOW(),
    NOW()
);

-- 第二步：插入传感器数据
-- 源传感器数据
INSERT INTO "sensor" (
    "flow_id",
    "identifier",
    "measuring",
    "unit",
    "value_type",
    "site_id"
) VALUES
(12, 'Temperature Sensor 1', 'temperature', 'DEGREES_CELCIUS', 'NUMBER', 1),
(12, 'Temperature Sensor 2', 'temperature', 'DEGREES_CELCIUS', 'NUMBER', 1),
(12, 'Temperature Sensor 3', 'temperature', 'DEGREES_CELCIUS', 'NUMBER', 1),
(12, 'Temperature Sensor 4', 'temperature', 'DEGREES_CELCIUS', 'NUMBER', 1),
(12, 'Temperature Sensor 5', 'temperature', 'DEGREES_CELCIUS', 'NUMBER', 1),
(12, 'Temperature Sensor 6', 'temperature', 'DEGREES_CELCIUS', 'NUMBER', 1),
(12, 'Temperature Sensor 7', 'temperature', 'DEGREES_CELCIUS', 'NUMBER', 1),
(12, 'Temperature Sensor 8', 'temperature', 'DEGREES_CELCIUS', 'NUMBER', 1);

-- 目标传感器数据（计算平均值的结果）
INSERT INTO "sensor" (
    "flow_id",
    "identifier",
    "measuring",
    "unit",
    "value_type",
    "site_id"
) VALUES
(NULL, 'B15Average', 'temperature', 'DEGREES_CELCIUS', 'NUMBER', 1);

-- 第三步：插入读数数据
-- 注意：这里假设传感器ID按顺序分配（1-8）
-- 在实际使用中，应该先查询数据库确认传感器的实际ID
INSERT INTO "reading" (
    "sensor_id",
    "timestamp",
    "raw_value",
    "value"
) VALUES
-- 传感器1的读数（ID=1）
(1, NOW() - INTERVAL '5 minutes', '22.5', 22.5),
(1, NOW() - INTERVAL '4 minutes', '23.0', 23.0),
(1, NOW() - INTERVAL '3 minutes', '22.8', 22.8),
(1, NOW() - INTERVAL '2 minutes', '23.2', 23.2),
(1, NOW() - INTERVAL '1 minute', '22.9', 22.9),

-- 传感器2的读数（ID=2）
(2, NOW() - INTERVAL '5 minutes', '21.8', 21.8),
(2, NOW() - INTERVAL '4 minutes', '22.1', 22.1),
(2, NOW() - INTERVAL '3 minutes', '22.0', 22.0),
(2, NOW() - INTERVAL '2 minutes', '21.9', 21.9),
(2, NOW() - INTERVAL '1 minute', '22.2', 22.2),

-- 传感器3的读数（ID=3）
(3, NOW() - INTERVAL '5 minutes', '23.2', 23.2),
(3, NOW() - INTERVAL '4 minutes', '23.5', 23.5),
(3, NOW() - INTERVAL '3 minutes', '23.3', 23.3),
(3, NOW() - INTERVAL '2 minutes', '23.4', 23.4),
(3, NOW() - INTERVAL '1 minute', '23.1', 23.1),

-- 传感器4的读数（ID=4）
(4, NOW() - INTERVAL '5 minutes', '22.9', 22.9),
(4, NOW() - INTERVAL '4 minutes', '23.0', 23.0),
(4, NOW() - INTERVAL '3 minutes', '22.8', 22.8),
(4, NOW() - INTERVAL '2 minutes', '23.1', 23.1),
(4, NOW() - INTERVAL '1 minute', '23.0', 23.0),

-- 传感器5的读数（ID=5）
(5, NOW() - INTERVAL '5 minutes', '22.3', 22.3),
(5, NOW() - INTERVAL '4 minutes', '22.5', 22.5),
(5, NOW() - INTERVAL '3 minutes', '22.4', 22.4),
(5, NOW() - INTERVAL '2 minutes', '22.6', 22.6),
(5, NOW() - INTERVAL '1 minute', '22.5', 22.5),

-- 传感器6的读数（ID=6）
(6, NOW() - INTERVAL '5 minutes', '23.1', 23.1),
(6, NOW() - INTERVAL '4 minutes', '23.2', 23.2),
(6, NOW() - INTERVAL '3 minutes', '23.0', 23.0),
(6, NOW() - INTERVAL '2 minutes', '23.3', 23.3),
(6, NOW() - INTERVAL '1 minute', '23.1', 23.1),

-- 传感器7的读数（ID=7）
(7, NOW() - INTERVAL '5 minutes', '22.7', 22.7),
(7, NOW() - INTERVAL '4 minutes', '22.8', 22.8),
(7, NOW() - INTERVAL '3 minutes', '22.6', 22.6),
(7, NOW() - INTERVAL '2 minutes', '22.9', 22.9),
(7, NOW() - INTERVAL '1 minute', '22.8', 22.8),

-- 传感器8的读数（ID=8）
(8, NOW() - INTERVAL '5 minutes', '23.4', 23.4),
(8, NOW() - INTERVAL '4 minutes', '23.5', 23.5),
(8, NOW() - INTERVAL '3 minutes', '23.3', 23.3),
(8, NOW() - INTERVAL '2 minutes', '23.6', 23.6),
(8, NOW() - INTERVAL '1 minute', '23.4', 23.4);