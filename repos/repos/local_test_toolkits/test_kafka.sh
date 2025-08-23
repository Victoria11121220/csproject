#!/bin/bash
# file name：test-kafka.sh
# Purpose: Quickly test whether the local Kafka is normal

TOPIC="test-topic"
# [Modification 1] Use the listener address inside the container. Since it is executed inside the Kafka container, use localhost + the internal port.
BROKER="localhost:9092" 
TEST_MSG="Hello Kafka Test $(date +%s)"

echo "=== Step 1: Create Topic ==="
# Note: --bootstrap-server also needs to use the modified address
docker exec -it kafka kafka-topics --create \
  --topic $TOPIC \
  --bootstrap-server $BROKER \
  --partitions 1 \
  --replication-factor 1 2>/dev/null || echo "Topic $TOPIC already exists"

echo "=== Step 2: Send a test message ==="
echo $TEST_MSG | docker exec -i kafka kafka-console-producer \
  --topic $TOPIC \
  --bootstrap-server $BROKER

echo "Message sent: $TEST_MSG"

# [Modification 2]: Wait a while before consuming to ensure that the message has been successfully written。
echo "Wait 2 seconds for the message to be written..."
sleep 2

echo "=== Step 3: Consume Message ==="
# Increase --max-messages 1 to make it exit after consuming a message to avoid hanging
docker exec -it kafka kafka-console-consumer \
  --topic $TOPIC \
  --bootstrap-server $BROKER \
  --from-beginning \
  --timeout-ms 10000 \
  --max-messages 1