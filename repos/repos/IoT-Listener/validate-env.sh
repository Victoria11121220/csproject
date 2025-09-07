#!/bin/bash

# Verify environment variables in .env file

echo "=== Verify Environment Variables ==="

# Check nodes variable
if [[ -z "$nodes" ]]; then
    echo "Error: nodes environment variable is not set"
    exit 1
else
    echo "nodes environment variable is set"
    echo "nodes content:"
    echo "$nodes" | jq .
fi

# Check edges variable
if [[ -z "$edges" ]]; then
    echo "Error: edges environment variable is not set"
    exit 1
else
    echo "edges environment variable is set"
    echo "edges content:"
    echo "$edges" | jq .
fi

# Check flow_id variable
if [[ -z "$flow_id" ]]; then
    echo "Error: flow_id environment variable is not set"
    exit 1
else
    echo "flow_id environment variable is set: $flow_id"
fi

echo "=== Verify Environment Variables Completed ==="