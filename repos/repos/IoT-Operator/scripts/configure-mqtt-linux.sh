#!/bin/bash

# Dedicated script for configuring MQTT addresses for Linux environments

echo "Configuring MQTT addresses for Linux environments..."

# In the Linux environment, we use several methods to detect the host IP

# Method 1: Use Docker Network Check
HOST_IP=""
if command -v docker &> /dev/null; then
    echo "Detecting Docker network..."
    # Find Kind Network
    KIND_NETWORK=$(docker network ls | grep kind | awk '{print $1}' | head -n 1)
    
    if [ -n "$KIND_NETWORK" ]; then
        echo "Found Kind network: $KIND_NETWORK"
        # Get network gateway IP
        HOST_IP=$(docker network inspect $KIND_NETWORK | grep Gateway | head -1 | awk -F'"' '{print $4}')
        
        if [ -n "$HOST_IP" ]; then
            echo "Successfully obtained host IP from Docker network: $HOST_IP"
        else
            echo "Failed to obtain gateway IP from Docker network"
        fi
    else
        echo "Kind network not found"
    fi
fi

# Method 2: If Method 1 fails, try using the routing table
if [ -z "$HOST_IP" ]; then
    echo "Use the routing table to detect the host IP..."
    HOST_IP=$(ip route | grep default | awk '{print $3}' | head -n 1)
    
    if [ -n "$HOST_IP" ]; then
        echo "Successfully obtained host IP from routing table: $HOST_IP"
    else
        echo "Failed to obtain default gateway from routing table"
    fi
fi

# 方法3: 如果以上方法都失败，尝试使用172.17.0.1（Docker默认网桥网关）
if [ -z "$HOST_IP" ]; then
    echo "尝试使用Docker默认网桥网关: 172.17.0.1"
    HOST_IP="172.17.0.1"
    echo "使用默认Docker网桥网关: $HOST_IP"
fi

# 更新配置
echo "设置MQTT配置使用检测到的地址: $HOST_IP"

# 更新mqtt-config.yaml文件
cat > k8s-manifests/mqtt-config.yaml << EOF
apiVersion: v1
kind: ConfigMap
metadata:
  name: mqtt-config
data:
  MQTT_BROKER_HOST: "$HOST_IP"
  MQTT_BROKER_PORT: "1883"
EOF

echo "已更新 k8s-manifests/mqtt-config.yaml 文件"
echo "MQTT_BROKER_HOST 设置为: $HOST_IP"

# 导出环境变量供数据库初始化脚本使用
export MQTT_HOST=$HOST_IP
echo "已导出环境变量 MQTT_HOST=$HOST_IP"