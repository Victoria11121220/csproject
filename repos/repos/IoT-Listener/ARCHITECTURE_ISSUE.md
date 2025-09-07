# IoT Listener Architecture Issue Analysis

## Current Architecture

### Data Flow
1. **Collector Process**:
   - Subscribes to external sources (MQTT, HTTP, etc.)
   - When data arrives, sources store it locally in their internal state (`RwLock<NodeData>`)
   - Collector sends only `NodeIndex` to Kafka (not the actual data)

2. **Processor Process**:
   - Receives `NodeIndex` from Kafka
   - Calls `graph.backpropagate()` to compute the graph
   - During computation, source nodes call `source.get()` to retrieve data
   - But the source data exists only in the collector process, not in the processor process

## The Problem

The current architecture has a fundamental flaw:

1. **Data Isolation**: Source data is stored locally within each source node in the collector process
2. **Process Separation**: The processor runs in a separate process and cannot access the collector's memory
3. **Data Loss**: When processor tries to get data from sources, it gets stale or default data, not the actual data that triggered the computation

This means the processor cannot correctly compute the graph because it doesn't have access to the actual source data that triggered the computation.

## Possible Solutions

### Solution 1: Send Data with NodeIndex (Recommended)

Modify the collector to send both `NodeIndex` and the actual `NodeData` to Kafka:

**Pros**:
- Simple and straightforward
- Processor has all the data it needs
- Minimal architectural changes

**Cons**:
- Larger Kafka messages
- Potential security concerns if data contains sensitive information

### Solution 2: Shared Data Store

Use a shared data store (Redis, database, etc.) where collector stores data and processor retrieves it:

**Pros**:
- Keeps Kafka messages small
- Data persistence across restarts

**Cons**:
- Additional infrastructure
- More complex implementation
- Latency in data retrieval

### Solution 3: Stateful Sources

Make sources stateful and have them fetch data in the processor:

**Pros**:
- No data transfer needed
- Processor fetches fresh data

**Cons**:
- Sources need to be able to refetch data (not all sources support this)
- Potential inconsistency between collector and processor data
- Network calls in processor can cause delays

## Recommended Approach

I recommend **Solution 1**: Modify the collector to send both `NodeIndex` and `NodeData` to Kafka.

### Implementation Plan

1. **Modify Collector**:
   - Change the data structure sent to Kafka to include both indices and data
   - Create a new data structure that contains `{ indices: Vec<NodeIndex>, data: HashMap<NodeIndex, NodeData> }`

2. **Modify Processor**:
   - Update processor to handle the new data structure
   - Pre-populate source nodes with the received data before backpropagation

3. **Data Structure**:
   ```rust
   struct TriggerData {
       indices: Vec<NodeIndex>,
       source_data: HashMap<NodeIndex, NodeData>,
   }
   ```

### Code Changes Needed

1. **Collector (`iot-collector.rs`)**:
   ```rust
   // Instead of sending just trigger_indices:
   let payload = serde_json::to_string(&trigger_indices)?;
   
   // Send trigger data with source data:
   let trigger_data = TriggerData {
       indices: trigger_indices,
       source_data: source_data_map, // Collect source data from triggered nodes
   };
   let payload = serde_json::to_string(&trigger_data)?;
   ```

2. **Processor (`iot-processor.rs`)**:
   ```rust
   // Instead of:
   match serde_json::from_slice::<Vec<NodeIndex>>(payload) {
       Ok(trigger_indices) => {
           // ...
           let (processed_readings, _node_errors, _) = graph.backpropagate(trigger_indices).await;
       }
   }
   
   // Do:
   match serde_json::from_slice::<TriggerData>(payload) {
       Ok(trigger_data) => {
           // Pre-populate source nodes with data
           for (node_index, data) in trigger_data.source_data {
               // Update source node data in graph
               // This might require adding a method to update source data
           }
           let (processed_readings, _node_errors, _) = graph.backpropagate(trigger_data.indices).await;
       }
   }
   ```

## Conclusion

The current architecture has a critical flaw where source data is not transferred from the collector to the processor. This prevents the processor from correctly computing the graph. The recommended solution is to modify the collector to send both NodeIndex and NodeData to Kafka, allowing the processor to have all the information it needs to perform computations correctly.