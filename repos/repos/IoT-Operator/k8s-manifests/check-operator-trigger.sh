#!/bin/bash

# Script to check if Operator has detected new flow records

echo "Checking if Operator has detected new flow records..."

# Get Operator Pod name
OPERATOR_POD=$(kubectl get pod -l control-plane=controller-manager -o jsonpath="{.items[0].metadata.name}" -n listener-operator-system)

if [ -z "$OPERATOR_POD" ]; then
    echo "Operator Pod not found"
    exit 1
fi

echo "Found Operator Pod: $OPERATOR_POD"

# View Operator logs to check for processing information of new flow records
echo "Recent Operator logs:"
kubectl logs $OPERATOR_POD -n listener-operator-system --tail=50

echo ""
echo "Checking if collector and processor Pods have been created..."
kubectl get pods -n listener-operator-system

echo ""
echo "If you want to continuously monitor Operator logs, run the following command:"
echo "kubectl logs -f $OPERATOR_POD -n listener-operator-system"