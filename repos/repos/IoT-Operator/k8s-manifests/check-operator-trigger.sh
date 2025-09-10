#!/bin/bash

# Script to check whether the Operator has listened to new flow records

echo "Checking if the Operator has listened to new flow records..."

# Get the Operator Pod name
OPERATOR_POD=$(kubectl get pod -l control-plane=controller-manager -o jsonpath="{.items[0].metadata.name}" -n listener-operator-system)

if [ -z "$OPERATOR_POD" ]; then
    echo "Operator Pod not found"
    exit 1
fi

echo "Found Operator Pod: $OPERATOR_POD"

# Check the Operator logs for any processing information about new flow records
echo "Recent Operator logs:"
kubectl logs $OPERATOR_POD -n listener-operator-system --tail=50

echo ""
echo "Checking if collector and processor Pods have been created..."
kubectl get pods -n listener-operator-system

echo ""
echo "If you want to continuously monitor the Operator logs, please run the following command:"
echo "kubectl logs -f $OPERATOR_POD -n listener-operator-system"