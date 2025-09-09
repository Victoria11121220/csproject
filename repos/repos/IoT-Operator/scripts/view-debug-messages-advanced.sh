#!/bin/bash

# View the contents of the iot_flow_debug_message table in the PostgreSQL database in the cluster
# Support filtering by time range and flow_id

# Default parameters
FLOW_ID=""
START_TIME=""
END_TIME=""
LIMIT=20

# Parsing command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -f|--flow-id)
            FLOW_ID="$2"
            shift 2
            ;;
        -s|--start-time)
            START_TIME="$2"
            shift 2
            ;;
        -e|--end-time)
            END_TIME="$2"
            shift 2
            ;;
        -l|--limit)
            LIMIT="$2"
            shift 2
            ;;
        -h|--help)
            echo "usage: $0 [options]"
            echo "options:"
            echo "  -f, --flow-id ID      Filter by flow_id"
            echo "  -s, --start-time TIME Filter by start time (format: 'YYYY-MM-DD HH:MM:SS')"
            echo "  -e, --end-time TIME   Filter by end time (format: 'YYYY-MM-DD HH:MM:SS')"
            echo "  -l, --limit COUNT     Limit the number of returned records (default: 20)"
            echo "  -h, --help            Show help information"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use $0 --help to see available options"
            exit 1
            ;;
    esac
done

echo "Start viewing the contents of the iot_flow_debug_message table..."

# Wait for PostgreSQL Pod to start
echo "Waiting for PostgreSQL Pod to start..."
kubectl wait --for=condition=ready pod -l app=postgres --timeout=120s -n listener-operator-system

# Get PostgreSQL Pod name
POSTGRES_POD=$(kubectl get pod -l app=postgres -o jsonpath="{.items[0].metadata.name}" -n listener-operator-system)
echo "Found PostgreSQL Pod: $POSTGRES_POD"

# Build query conditions
WHERE_CLAUSE=""
if [[ -n "$FLOW_ID" ]]; then
    WHERE_CLAUSE="WHERE flow_id = $FLOW_ID"
fi

if [[ -n "$START_TIME" ]]; then
    if [[ -n "$WHERE_CLAUSE" ]]; then
        WHERE_CLAUSE="$WHERE_CLAUSE AND timestamp >= '$START_TIME'"
    else
        WHERE_CLAUSE="WHERE timestamp >= '$START_TIME'"
    fi
fi

if [[ -n "$END_TIME" ]]; then
    if [[ -n "$WHERE_CLAUSE" ]]; then
        WHERE_CLAUSE="$WHERE_CLAUSE AND timestamp <= '$END_TIME'"
    else
        WHERE_CLAUSE="WHERE timestamp <= '$END_TIME'"
    fi
fi

# Build query statement
QUERY="SELECT * FROM iot_flow_debug_message $WHERE_CLAUSE ORDER BY timestamp DESC LIMIT $LIMIT;"

echo "Executing query: $QUERY"
echo ""
echo "Contents of iot_flow_debug_message table:"
echo "=============================="

kubectl exec -it $POSTGRES_POD -n listener-operator-system -- psql -U postgres -d iot_dataflow_manager -c "$QUERY"

echo ""
echo "Total number of records matching the criteria:"
echo "=================="

COUNT_QUERY="SELECT COUNT(*) FROM iot_flow_debug_message $WHERE_CLAUSE;"
kubectl exec -it $POSTGRES_POD -n listener-operator-system -- psql -U postgres -d iot_dataflow_manager -c "$COUNT_QUERY"