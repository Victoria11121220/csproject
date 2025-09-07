# IoT Listener Architecture Fix Proposal

## Problem Summary

The current architecture has a critical flaw where source data is stored locally in the collector process but is not transferred to the processor process. This prevents the processor from correctly computing the graph because it cannot access the actual source data that triggered the computation.

## Root Cause

1. **MQTT Source Data Storage**: MQTT source stores data in its internal `data` field (`Arc<RwLock<NodeData>>`)
2. **Serialization Issue**: This `data` field is marked with `#[serde(skip)]`, so it's not serialized when sending data between processes
3. **Process Isolation**: Collector and processor run in separate processes with isolated memory spaces

## Proposed Solution

Modify the architecture to send both trigger indices and source data through Kafka:

### 1. Modify Collector to Send Source Data

Update `iot-collector.rs` to collect and send source data:

```rust
// Add new data structure
#[derive(Serialize, Deserialize, Debug)]
struct TriggerData {
    indices: Vec<NodeIndex>,
    source_data: HashMap<String, serde_json::Value>, // Map node IDs to their data
}

// In the main loop, instead of:
let payload = serde_json::to_string(&trigger_indices)?;

// Do:
let mut source_data_map = HashMap::new();
// Collect source data from triggered nodes
for node_index in &trigger_indices {
    let graph = graph.read().await;
    if let Some(node) = graph.node_weight(*node_index) {
        if let NodeType::Source(source_node) = node.concrete_node() {
            // Get source data
            if let Ok(data) = source_node.source().get() {
                source_data_map.insert(node.id().to_string(), serde_json::to_value(data).unwrap());
            }
        }
    }
}

let trigger_data = TriggerData {
    indices: trigger_indices,
    source_data: source_data_map,
};
let payload = serde_json::to_string(&trigger_data)?;
```

### 2. Modify Processor to Handle Source Data

Update `iot-processor.rs` to pre-populate source nodes:

```rust
// Add new data structure
#[derive(Serialize, Deserialize, Debug)]
struct TriggerData {
    indices: Vec<NodeIndex>,
    source_data: HashMap<String, serde_json::Value>,
}

// In the main loop, instead of:
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
        let mut graph = graph.write().await;
        for (node_id, data_json) in trigger_data.source_data {
            // Find the node with this ID and update its data
            // This requires adding a method to update source data
            if let Some(node_index) = find_node_by_id(&graph, &node_id) {
                if let NodeType::Source(source_node) = graph[node_index].concrete_node() {
                    // Update source data - this requires modifying the source node implementation
                }
            }
        }
        
        // Now backpropagate with the updated data
        drop(graph); // Release write lock
        let (processed_readings, _node_errors, _) = graph.backpropagate(trigger_data.indices).await;
    }
}
```

### 3. Modify Source Nodes to Accept External Data

Add a method to SourceNode to update its source data:

```rust
// In src/nodes/sources/mod.rs
impl SourceNode {
    pub fn update_source_data(&mut self, data: NodeData) -> Result<(), NodeError> {
        // This requires making the source mutable and adding a method to update its internal data
        // For MQTT source, this would mean updating the RwLock<NodeData>
        match &mut self.source {
            Source::Mqtt(mqtt_source) => {
                // This requires making MQTTSource's data field accessible
                // Currently it's private and marked with #[serde(skip)]
            }
            // Handle other source types
        }
        Ok(())
    }
}
```

### 4. Make Source Data Accessible

Modify MQTTSource to allow external data updates:

```rust
// In src/sources/mqtt.rs
impl MQTTSource {
    pub fn update_data(&self, data: NodeData) -> Result<(), Box<dyn std::error::Error>> {
        let mut write_guard = self.data.write().map_err(|_| "Failed to acquire write lock")?;
        *write_guard = data;
        Ok(())
    }
}
```

## Alternative Approach: Shared Data Store

If modifying the source nodes is too complex, we could use a shared data store:

1. **Redis or Database**: Store source data in Redis/database with a unique key
2. **Key Transfer**: Send only the keys through Kafka
3. **Data Retrieval**: Processor retrieves data from the shared store

This approach would require:
- Adding Redis/database dependency
- Modifying collector to store data in shared store
- Modifying processor to retrieve data from shared store

## Implementation Steps

1. **Create TriggerData structure** in both collector and processor
2. **Modify collector** to collect and send source data
3. **Modify processor** to handle TriggerData structure
4. **Add methods to update source data** in source nodes
5. **Test the solution** with MQTT source
6. **Update documentation** and examples

## Benefits of This Solution

1. **Correctness**: Processor will have access to actual source data
2. **Minimal Changes**: Only requires changes to data transfer, not core logic
3. **Backward Compatibility**: Can be implemented without breaking existing functionality
4. **Performance**: Data is transferred only when needed

## Potential Issues

1. **Larger Kafka Messages**: Including source data will increase message size
2. **Serialization Overhead**: Converting NodeData to JSON and back
3. **Memory Usage**: Storing data in multiple places temporarily

## Conclusion

This fix addresses the fundamental architectural issue by ensuring that the processor has access to the actual source data that triggered the computation. The solution involves minimal architectural changes while maintaining correctness and performance.