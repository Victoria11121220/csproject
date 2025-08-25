#!/bin/bash
# 文件名：test-kafka.sh
# 用途：快速测试本地 Kafka 是否正常

TOPIC="test-topic"
# 【修改1】使用容器内部的监听地址。因为是在 kafka 容器内执行，用 localhost + 内部端口 即可。
BROKER="localhost:9092" 
TEST_MSG="Hello Kafka Test $(date +%s)"

echo "=== Step 1: 创建 Topic ==="
# 注意：--bootstrap-server 也要使用修改后的地址
docker exec -it kafka kafka-topics --create \
  --topic $TOPIC \
  --bootstrap-server $BROKER \
  --partitions 1 \
  --replication-factor 1 2>/dev/null || echo "Topic $TOPIC 已存在"

echo "=== Step 2: 发送测试消息 ==="
echo $TEST_MSG | docker exec -i kafka kafka-console-producer \
  --topic $TOPIC \
  --bootstrap-server $BROKER

echo "消息已发送: $TEST_MSG"

# 【修改2】在消费前等待一小会儿，确保消息已成功写入。
echo "等待 2 秒钟，让消息写入..."
sleep 2

echo "=== Step 3: 消费消息 ==="
# 增加 --max-messages 1 让它消费到一条消息后就退出，避免一直挂起
docker exec -it kafka kafka-console-consumer \
  --topic $TOPIC \
  --bootstrap-server $BROKER \
  --from-beginning \
  --timeout-ms 10000 \
  --max-messages 1