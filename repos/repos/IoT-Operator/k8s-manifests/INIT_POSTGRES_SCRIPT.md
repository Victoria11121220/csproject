# PostgreSQL Instructions for using the database initialization script

## Script functionality

`init-postgres.sh` The script is used to initialize the table structure of the PostgreSQL database in the Kubernetes environment. It will:

1. Extract database connection information from the `../dotenv-listener/.env` file
2. Wait for the PostgreSQL Pod to start.
3. Connect to the database and create the required table structure
4. Insert initial data

## How to use

1. Ensure that the PostgreSQL service has been deployed in the Kubernetes environment.
2. Make sure the `../dotenv-listener/.env` file exists and contains the correct database connection information
3. Run the script:
   ```bash
   cd k8s-manifests
   ./init-postgres.sh
   ```

## How the script works

1. **Extract database connection information**：
   The script parses the following information from the `../dotenv-listener/.env` file:
   - Database users (DB_USER)
   - Database password (DB_PASSWORD)
   - Database host (DB_HOST)
   - Database port (DB_PORT)
   - Database name (DB_NAME)

2. **Wait for the PostgreSQL Pod to be ready**：
   脚本会等待 PostgreSQL Pod 进入就绪状态，超时时间为 120 秒。

3. **Create table structure**：
   The script creates the following tables：
   - `site`：Site Information Table
   - `iot_flow`：IoT process definition table
   - `iot_source`：IoT data source table
   - `sensor`：Sensor table
   - `reading`：Sensor reading table
   - `sensors`：Sensor Information Table
   - `sensor_metadata`：Sensor metadata table
   - `iot_deployment`：IoT deployment status table
   - `flow_error`：Process error record table
   - `debug_messages`：Debug message table
   - `iot_flow_debug_message`：IoT process debugging message table

4. **Create a trigger**：
   The script creates a trigger that sends a notification when a new record is inserted into the `iot_flow` table.

5. **Insert initial data**：
   The script inserts a test site and an initial IoT process definition.

## Precautions

1. Make sure the PostgreSQL service in the Kubernetes environment is deployed and running before running the script.
2. Make sure the `../dotenv-listener/.env` file exists and contains the correct database connection information.
3. If the table structure already exists, the script will not overwrite the existing data.
4. The script creates the necessary indexes to optimize query performance.

## Troubleshooting

If the script fails, check：

1. Check whether the PostgreSQL Pod is running properly:
   ```bash
   kubectl get pods -l app=postgres
   ```

2. Is the database connection information correct：
   Check the `uri` configuration in the `../dotenv-listener/.env` file.

3. Do you have sufficient permissions to perform database operations：
   Make sure that the database user you use has permissions to create tables and insert data.