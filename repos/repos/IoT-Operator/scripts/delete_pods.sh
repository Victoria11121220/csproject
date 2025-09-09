#!/bin/bash

kubectl delete pod iot-collector-1 -n listener-operator-system
kubectl delete pod iot-processor-1 -n listener-operator-system